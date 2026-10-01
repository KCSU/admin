//! Entry point of the alerting service.

use std::sync::Arc;

use alerts::AlertHandler;
use alerts::consumer::BoxError;
use alerts::consumers::firestore::Firestore;
use alerts::consumers::google_chat::GoogleChat;

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    let webhook = common::require_env("CHAT_WEBHOOK_URL")?;
    let project = common::require_env("GOOGLE_CLOUD_PROJECT")?;

    let handlers = Arc::from(vec![
        AlertHandler::GoogleChat(GoogleChat::new(webhook)?),
        AlertHandler::Firestore(Firestore::new(&project)?),
    ]);

    let port = common::require_env("PORT")?;
    alerts::consumer::serve(handlers, &port).await?;
    Ok(())
}
