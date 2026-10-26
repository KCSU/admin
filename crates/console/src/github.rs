//! Asks GitHub Actions to rebuild KCSU/web.

use std::time::Duration;

const DISPATCH_URL: &str = "https://api.github.com/repos/KCSU/web/dispatches";

#[derive(Debug, thiserror::Error)]
pub enum GithubError {
    #[error("HTTP client setup failed")]
    HttpClient(#[source] reqwest::Error),

    #[error("dispatch request failed")]
    Request(#[source] reqwest::Error),

    #[error("GitHub returned {status}: {body}")]
    Status {
        status: reqwest::StatusCode,
        body: String,
    },
}

/// Sends `repository_dispatch` events to KCSU/web.
pub struct Dispatcher {
    token: String,
    http_client: reqwest::Client,
}

impl Dispatcher {
    /// `token` is a fine-grained token with Contents: write on KCSU/web only.
    pub fn new(token: String) -> Result<Self, GithubError> {
        Ok(Self {
            token,
            http_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .user_agent("kcsu-admin")
                .build()
                .map_err(GithubError::HttpClient)?,
        })
    }

    /// Sends a `repository_dispatch` of type `event_type`.
    pub(crate) async fn dispatch(&self, event_type: &str) -> Result<(), GithubError> {
        let response = self
            .http_client
            .post(DISPATCH_URL)
            .bearer_auth(&self.token)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .json(&serde_json::json!({ "event_type": event_type }))
            .send()
            .await
            .map_err(GithubError::Request)?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(GithubError::Status { status, body });
        }
        Ok(())
    }
}
