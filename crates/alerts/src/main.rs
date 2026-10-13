//! Entry point of the alerting service.

use std::sync::Arc;
use std::time::Duration;

use alerts::consumer::BoxError;
use alerts::consumers::firestore::Firestore;
use alerts::consumers::google_chat::GoogleChat;
use alerts::retry::Retry;
use alerts::{AlertHandler, Handler};

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    let webhook = common::require_env("CHAT_WEBHOOK_URL")?;
    let project = common::require_env("GOOGLE_CLOUD_PROJECT")?;

    let handlers = Arc::from(vec![
        Handler::new(AlertHandler::GoogleChat(GoogleChat::new(webhook)?)).retry(Retry {
            attempts: 5,
            first_delay: Duration::from_secs(1),
        }),
        Handler::new(AlertHandler::Firestore(Firestore::new(&project)?)).retry(Retry {
            attempts: 3,
            first_delay: Duration::from_secs(1),
        }),
    ]);

    let port = common::require_env("PORT")?;
    alerts::consumer::serve(handlers, &port).await?;
    Ok(())
}
