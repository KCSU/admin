//! Harness for a container that consumes alerts from a push subscription.
//!
//! Each consumer is a variant of [`AlertHandler`].
//!
//! Delivery is at most once: every push is acknowledged, whether the handlers
//! succeed, fail, or the message is malformed. A handler retries transient
//! failures itself; once it gives up, the failure is logged.

use std::sync::Arc;

use axum::Router;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::post;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use futures_util::future::join_all;
use prost::Message as _;

use crate::consumers::firestore::Firestore;
use crate::consumers::google_chat::GoogleChat;
use crate::retry::Retry;
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

    fn name(&self) -> &'static str {
        match self {
            Self::GoogleChat(_) => "GoogleChat",
            Self::Firestore(_) => "Firestore",
        }
    }
}

/// An [`AlertHandler`], with how it retries.
pub struct Handler {
    handler: AlertHandler,
    retry: Retry,
}

impl Handler {
    /// Handles every alert, trying each once.
    pub fn new(handler: AlertHandler) -> Self {
        Self {
            handler,
            retry: Retry::NEVER,
        }
    }

    pub fn retry(mut self, retry: Retry) -> Self {
        self.retry = retry;
        self
    }

    /// Handles `alert`. A failure is logged.
    async fn run(&self, alert: &Alert) {
        if let Err(err) = self.handle_with_retries(alert).await {
            eprintln!(
                "{} failed to handle alert {}/{}: {}",
                self.handler.name(),
                alert.source,
                alert.dedup_key,
                common::report(&err)
            );
        }
    }

    async fn handle_with_retries(&self, alert: &Alert) -> Result<(), ConsumeError> {
        let mut attempt = 1;
        loop {
            match self.handler.handle(alert).await {
                Err(err) if err.is_transient() && attempt < self.retry.attempts => {
                    let delay = self.retry.delay(attempt);
                    eprintln!(
                        "{} attempt {attempt} at alert {}/{} failed, retrying in {delay:?}: {}",
                        self.handler.name(),
                        alert.source,
                        alert.dedup_key,
                        common::report(&err)
                    );
                    tokio::time::sleep(delay).await;
                    attempt += 1;
                }
                result => return result,
            }
        }
    }
}

/// Serve the pub/sub push endpoint at `port`.
/// Runs every handler on each alert.
pub async fn serve(handlers: Arc<[Handler]>, port: &str) -> std::io::Result<()> {
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
    let app = Router::new().route("/", post(receive)).with_state(handlers);

    axum::serve(listener, app).await
}

/// Handles one push request.
async fn receive(State(handlers): State<Arc<[Handler]>>, body: Bytes) -> StatusCode {
    match read_push(&body) {
        // Run the handlers concurrently, and wait for all of them.
        Ok(alert) => {
            join_all(handlers.iter().map(|handler| handler.run(&alert))).await;
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
