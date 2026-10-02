//! A minimal client for Firestore's REST API.

use std::time::Duration;

use google_cloud_auth::credentials::{AccessTokenCredentials, Builder};
use serde_json::{Value, json};

#[derive(Debug, thiserror::Error)]
pub enum FirestoreError {
    #[error("HTTP client setup failed")]
    HttpClient(#[source] reqwest::Error),

    #[error("Google credentials setup failed")]
    CredentialsSetup(#[source] google_cloud_auth::build_errors::Error),

    #[error("getting a Google access token failed")]
    AccessToken(#[source] google_cloud_auth::errors::CredentialsError),

    #[error("Firestore request failed")]
    Request(#[source] reqwest::Error),

    /// Firestore explains the failure in the response body.
    #[error("Firestore returned {status}: {body}")]
    Status {
        status: reqwest::StatusCode,
        body: String,
    },
}

/// Writes documents to the default Firestore database of a project.
pub struct Firestore {
    documents_url: String,
    http_client: reqwest::Client,
    credentials: AccessTokenCredentials,
}

impl Firestore {
    /// Creates a client for the default database of `project`.
    pub fn new(project: &str) -> Result<Self, FirestoreError> {
        Ok(Self {
            documents_url: format!(
                "https://firestore.googleapis.com/v1/projects/{project}/databases/(default)/documents"
            ),
            http_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .map_err(FirestoreError::HttpClient)?,
            credentials: Builder::default()
                .build_access_token_credentials()
                .map_err(FirestoreError::CredentialsSetup)?,
        })
    }

    /// Adds a new document `fields` to `collection`.
    ///
    /// `fields` is the document's contents in Firestore's REST format.
    /// See <https://firebase.google.com/docs/firestore/use-rest-api> for more details.
    pub async fn add(&self, collection: &str, fields: Value) -> Result<(), FirestoreError> {
        let url = format!("{}/{collection}", self.documents_url);
        self.send(self.http_client.post(url), fields).await
    }

    /// Creates the document `collection/id` and populates it with `fields`, or replaces
    /// it if it exists.
    ///
    /// `fields` is the document's contents in Firestore's REST format.
    /// See <https://firebase.google.com/docs/firestore/use-rest-api> for more details.
    pub async fn set(
        &self,
        collection: &str,
        id: &str,
        fields: Value,
    ) -> Result<(), FirestoreError> {
        let url = format!("{}/{collection}/{id}", self.documents_url);
        self.send(self.http_client.patch(url), fields).await
    }

    async fn send(
        &self,
        request: reqwest::RequestBuilder,
        fields: Value,
    ) -> Result<(), FirestoreError> {
        // A short-lived Google OAuth2 access token.
        let token = self
            .credentials
            .access_token()
            .await
            .map_err(FirestoreError::AccessToken)?
            .token;

        let response = request
            .bearer_auth(token)
            .json(&json!({ "fields": fields }))
            .send()
            .await
            .map_err(FirestoreError::Request)?;

        let status = response.status();
        if status.is_success() {
            Ok(())
        } else {
            let body = response.text().await.unwrap_or_default();
            Err(FirestoreError::Status { status, body })
        }
    }
}
