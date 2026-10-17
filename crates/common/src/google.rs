//! An HTTP client for Google APIs, signing requests with the environment's credentials.

use std::time::Duration;

use google_cloud_auth::credentials::{AccessTokenCredentials, Builder};

pub(crate) enum SetupError {
    HttpClient(reqwest::Error),
    Credentials(google_cloud_auth::build_errors::Error),
}

pub(crate) enum SendError {
    AccessToken(google_cloud_auth::errors::CredentialsError),
    Request(reqwest::Error),
}

pub(crate) struct GoogleClient {
    /// For building requests. Send them with [`GoogleClient::send`].
    pub http: reqwest::Client,
    credentials: AccessTokenCredentials,
}

impl GoogleClient {
    /// A client whose requests give up after `timeout`.
    pub fn new(timeout: Duration) -> Result<Self, SetupError> {
        Ok(Self {
            http: reqwest::Client::builder()
                .timeout(timeout)
                .build()
                .map_err(SetupError::HttpClient)?,
            credentials: Builder::default()
                .build_access_token_credentials()
                .map_err(SetupError::Credentials)?,
        })
    }

    /// Sends `request` with a short-lived access token. The caller checks the status.
    pub async fn send(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<reqwest::Response, SendError> {
        let token = self
            .credentials
            .access_token()
            .await
            .map_err(SendError::AccessToken)?
            .token;
        request
            .bearer_auth(token)
            .send()
            .await
            .map_err(SendError::Request)
    }
}
