#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Couldn't build the HTTP client.
    #[error("HTTP client setup failed")]
    HttpClient(#[source] reqwest::Error),

    /// Couldn't get an OAuth2 access token.
    #[error("API Gateway token request failed")]
    Token(#[source] reqwest::Error),

    /// A Lookup request failed due to network / parsing error.
    #[error("lookup request for group {group_id} failed")]
    Lookup {
        group_id: String,
        #[source]
        source: reqwest::Error,
    },

    /// Lookup returned no current members.
    #[error("lookup group {group_id} has no members")]
    EmptyGroup { group_id: String },

    /// Couldn't create the Pub/Sub publisher, e.g. no Google credentials found.
    #[error("pub/sub publisher setup failed")]
    PubSubSetup(#[source] google_cloud_gax::client_builder::Error),

    /// Pub/Sub rejected or failed to publish a snapshot.
    #[error("publishing snapshot for group {group_id} failed")]
    Publish {
        group_id: String,
        #[source]
        source: google_cloud_pubsub::error::PublishError,
    },

    /// Couldn't create the Firestore client, e.g. no Google credentials found.
    #[error("Firestore client setup failed")]
    StoreSetup(#[source] common::FirestoreError),

    /// The snapshot's fetch time can't be written as a timestamp.
    #[error("snapshot has an invalid timestamp")]
    Timestamp(#[source] common::TimestampOutOfRange),

    /// The snapshot couldn't be converted into a Firestore document.
    #[error("encoding snapshot for Firestore failed")]
    Encode(#[source] serde_json::Error),

    /// Firestore rejected or failed to store a snapshot.
    #[error("storing snapshot for group {group_id} failed")]
    Store {
        group_id: String,
        #[source]
        source: common::FirestoreError,
    },
}

impl Error {
    pub fn is_transient(&self) -> bool {
        match self {
            // A timeout, a dropped connection or a garbled response.
            Self::Token(source) | Self::Lookup { source, .. } => {
                source.status().is_none_or(|status| {
                    status.is_server_error() || status == reqwest::StatusCode::TOO_MANY_REQUESTS
                })
            }
            Self::EmptyGroup { .. } | Self::Publish { .. } | Self::Store { .. } => true,
            Self::HttpClient(_)
            | Self::PubSubSetup(_)
            | Self::StoreSetup(_)
            | Self::Timestamp(_)
            | Self::Encode(_) => false,
        }
    }
}
