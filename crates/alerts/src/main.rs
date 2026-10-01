//! Entry point of the alerting service.

use std::sync::Arc;

use alerts::AlertHandler;
use alerts::consumers::google_chat::GoogleChat;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let webhook = common::require_env("CHAT_WEBHOOK_URL")?;

    let handlers = Arc::from(vec![AlertHandler::GoogleChat(GoogleChat::new(webhook)?)]);

    let port = common::require_env("PORT")?;
    alerts::consumer::serve(handlers, &port).await?;
    Ok(())
}
