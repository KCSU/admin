//! Entry point of the alerting service.

use std::sync::Arc;

use alerts::AlertHandler;
use alerts::consumers::google_chat::GoogleChat;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let webhook = std::env::var("CHAT_WEBHOOK_URL").map_err(|_| "CHAT_WEBHOOK_URL is not set")?;

    let handlers = Arc::from(vec![AlertHandler::GoogleChat(GoogleChat::new(webhook)?)]);

    let port = std::env::var("PORT").expect("missing env var PORT");
    alerts::consumer::serve(handlers, &port).await?;
    Ok(())
}
