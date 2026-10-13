/// Error thrown by an uptime run.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("HTTP client setup failed")]
    HttpClient(#[source] reqwest::Error),

    #[error("reading the uptime summary from Firestore failed")]
    Read(#[source] common::FirestoreError),

    #[error("the stored uptime summary is malformed")]
    Decode(#[source] serde_json::Error),

    #[error("encoding the uptime summary failed")]
    Encode(#[source] serde_json::Error),

    #[error("writing the uptime summary to Firestore failed")]
    Write(#[source] common::FirestoreError),
}
