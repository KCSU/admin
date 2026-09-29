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
}
