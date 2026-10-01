//! Critical alerts raised by KCSU services, carried over pub/sub.
//!
//! - Producing: a service creates one [`AlertPublisher`] at startup and
//!   sends each [`Alert`] to the `alerts` topic. See its docs for an example.
//! - Consuming: the `alerting` binary runs [`consumer::serve`], which receives
//!   alerts from a push subscription and passes each one to every [`AlertHandler`].

pub mod consumer;
mod error;
mod producer;

pub mod consumers {
    pub mod google_chat;
}

pub use consumer::AlertHandler;
pub use error::{ConsumeError, SendError};
pub use producer::{AlertBuilder, AlertPublisher, to_message};
pub use proto::alerts::v1::Alert;

/// The first required field that's blank, if any.
pub(crate) fn empty_field(alert: &Alert) -> Option<&'static str> {
    [
        ("source", &alert.source),
        ("dedup_key", &alert.dedup_key),
        ("summary", &alert.summary),
    ]
    .into_iter()
    .find(|(_, value)| value.trim().is_empty())
    .map(|(field, _)| field)
}
