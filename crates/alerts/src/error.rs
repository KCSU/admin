/// Error thrown by an alert publisher.
#[derive(Debug, thiserror::Error)]
pub enum SendError {
    #[error("alert publisher setup failed")]
    Setup(#[source] google_cloud_gax::client_builder::Error),

    #[error("alert {0} must not be empty")]
    Empty(&'static str),

    #[error("publishing alert failed")]
    Publish(#[source] google_cloud_pubsub::error::PublishError),
}

/// Error thrown by an alert consumer.
#[derive(Debug, thiserror::Error)]
pub enum ConsumeError {
    #[error("not a pub/sub push request")]
    Envelope(#[source] serde_json::Error),

    #[error("message data isn't base64")]
    Base64(#[source] base64::DecodeError),

    #[error("failed to decode message into Alert")]
    Decode(#[source] prost::DecodeError),

    #[error("alert {0} is empty")]
    Empty(&'static str),

    #[error("posting alert to Google Chat failed")]
    GoogleChat(#[source] reqwest::Error),

    #[error("alert has an invalid timestamp")]
    Timestamp(#[source] common::TimestampOutOfRange),

    #[error("persisting alert to Firestore failed")]
    Firestore(#[source] common::FirestoreError),
}
