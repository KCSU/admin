//! A minimal client for reading Cloud Logging entries.

use std::time::Duration;

use serde::Deserialize;
use serde_json::{Value, json};

use crate::google::{GoogleClient, SendError, SetupError};

#[derive(Debug, thiserror::Error)]
pub enum LoggingError {
    #[error("HTTP client setup failed")]
    HttpClient(#[source] reqwest::Error),

    #[error("Google credentials setup failed")]
    CredentialsSetup(#[source] google_cloud_auth::build_errors::Error),

    #[error("getting a Google access token failed")]
    AccessToken(#[source] google_cloud_auth::errors::CredentialsError),

    #[error("Cloud Logging request failed")]
    Request(#[source] reqwest::Error),

    #[error("Cloud Logging returned {status}: {body}")]
    Status {
        status: reqwest::StatusCode,
        body: String,
    },
}

impl From<SetupError> for LoggingError {
    fn from(err: SetupError) -> Self {
        match err {
            SetupError::HttpClient(err) => Self::HttpClient(err),
            SetupError::Credentials(err) => Self::CredentialsSetup(err),
        }
    }
}

impl From<SendError> for LoggingError {
    fn from(err: SendError) -> Self {
        match err {
            SendError::AccessToken(err) => Self::AccessToken(err),
            SendError::Request(err) => Self::Request(err),
        }
    }
}

/// One page of `entries.list`, with the entries as raw JSON.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntriesPage {
    #[serde(default)]
    pub entries: Vec<Value>,
    pub next_page_token: Option<String>,
}

/// Reads log entries.
pub struct Logging {
    client: GoogleClient,
}

impl Logging {
    pub fn new() -> Result<Self, LoggingError> {
        Ok(Self {
            client: GoogleClient::new(Duration::from_secs(30))?,
        })
    }

    /// One page of the entries in `projects` matching `filter`, newest first.
    pub async fn list(
        &self,
        projects: &[&str],
        filter: &str,
        page_size: u32,
        page_token: Option<&str>,
    ) -> Result<EntriesPage, LoggingError> {
        let resource_names: Vec<String> =
            projects.iter().map(|p| format!("projects/{p}")).collect();
        let request = self
            .client
            .http
            .post("https://logging.googleapis.com/v2/entries:list")
            .json(&json!({
                "resourceNames": resource_names,
                "filter": filter,
                "orderBy": "timestamp desc",
                "pageSize": page_size,
                "pageToken": page_token,
            }));
        let response = self.client.send(request).await?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(LoggingError::Status { status, body });
        }
        response.json().await.map_err(LoggingError::Request)
    }
}
