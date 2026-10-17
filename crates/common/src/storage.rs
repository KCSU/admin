//! A minimal client for uploading to Cloud Storage.

use std::time::Duration;

use crate::google::{GoogleClient, SendError, SetupError};

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("HTTP client setup failed")]
    HttpClient(#[source] reqwest::Error),

    #[error("Google credentials setup failed")]
    CredentialsSetup(#[source] google_cloud_auth::build_errors::Error),

    #[error("getting a Google access token failed")]
    AccessToken(#[source] google_cloud_auth::errors::CredentialsError),

    #[error("Cloud Storage request failed")]
    Request(#[source] reqwest::Error),

    #[error("Cloud Storage returned {status}: {body}")]
    Status {
        status: reqwest::StatusCode,
        body: String,
    },
}

impl From<SetupError> for StorageError {
    fn from(err: SetupError) -> Self {
        match err {
            SetupError::HttpClient(err) => Self::HttpClient(err),
            SetupError::Credentials(err) => Self::CredentialsSetup(err),
        }
    }
}

impl From<SendError> for StorageError {
    fn from(err: SendError) -> Self {
        match err {
            SendError::AccessToken(err) => Self::AccessToken(err),
            SendError::Request(err) => Self::Request(err),
        }
    }
}

/// Uploads objects to Cloud Storage buckets.
pub struct Storage {
    client: GoogleClient,
}

impl Storage {
    pub fn new() -> Result<Self, StorageError> {
        Ok(Self {
            client: GoogleClient::new(Duration::from_secs(30))?,
        })
    }

    /// Creates the object `bucket/name`. Fails if it already exists.
    pub async fn create(
        &self,
        bucket: &str,
        name: &str,
        content_type: &str,
        cache_control: &str,
        data: Vec<u8>,
    ) -> Result<(), StorageError> {
        let request = self
            .client
            .http
            .put(format!("https://storage.googleapis.com/{bucket}/{name}"))
            .header(reqwest::header::CONTENT_TYPE, content_type)
            .header(reqwest::header::CACHE_CONTROL, cache_control)
            // Only if there's no object by this name yet.
            .header("x-goog-if-generation-match", "0")
            .body(data);
        let response = self.client.send(request).await?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(StorageError::Status { status, body });
        }
        Ok(())
    }
}
