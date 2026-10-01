//! Harness for a container that consumes alerts from a push subscription.
//!
//! Each consumer is a variant of [`AlertHandler`].
//!
//! Delivery is at most once: every push is acknowledged, whether the handlers
//! succeed, fail, or the message is malformed. Failures are logged, not
//! retried.

use std::sync::Arc;

use axum::Router;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::post;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use prost::Message as _;

use crate::consumers::firestore::Firestore;
use crate::consumers::google_chat::GoogleChat;
use crate::{Alert, ConsumeError, empty_field};

pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Something that acts on an [`Alert`].
/// To add a handler, add a variant in [`handle`](Self::handle).
pub enum AlertHandler {
    GoogleChat(GoogleChat),
    Firestore(Firestore),
}

impl AlertHandler {
    pub async fn handle(&self, alert: &Alert) -> Result<(), ConsumeError> {
        match self {
            Self::GoogleChat(chat) => chat.handle(alert).await,
            Self::Firestore(store) => store.handle(alert).await,
        }
    }
}

/// Serve the pub/sub push endpoint at `port`.
/// Runs every handler on each alert.
pub async fn serve(handlers: Arc<[AlertHandler]>, port: &str) -> std::io::Result<()> {
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
    let app = Router::new().route("/", post(receive)).with_state(handlers);

    axum::serve(listener, app).await
}

/// Handles one push request.
async fn receive(State(handlers): State<Arc<[AlertHandler]>>, body: Bytes) -> StatusCode {
    match read_push(&body) {
        Ok(alert) => {
            for handler in handlers.iter() {
                if let Err(err) = handler.handle(&alert).await {
                    eprintln!(
                        "handling alert {}/{} failed: {}",
                        alert.source,
                        alert.dedup_key,
                        common::report(&err)
                    );
                }
            }
        }
        Err(err) => eprintln!("dropping unreadable push request: {}", common::report(&err)),
    }

    // Always 204, since alert handlers should not handle the same alert twice.
    StatusCode::NO_CONTENT
}

/// Extract the alert from the JSON body.
fn read_push(body: &[u8]) -> Result<Alert, ConsumeError> {
    #[derive(serde::Deserialize)]
    struct Push {
        message: PushMessage,
    }

    #[derive(serde::Deserialize)]
    struct PushMessage {
        #[serde(default)]
        data: String,
    }

    // Decode JSON, extract base64 body, decode into alert
    let push: Push = serde_json::from_slice(body).map_err(ConsumeError::Envelope)?;
    let data = STANDARD
        .decode(push.message.data)
        .map_err(ConsumeError::Base64)?;
    let alert = Alert::decode(data.as_slice()).map_err(ConsumeError::Decode)?;
    match empty_field(&alert) {
        Some(field) => Err(ConsumeError::Empty(field)),
        None => Ok(alert),
    }
}
