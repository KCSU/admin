//! Persists alerts in Firestore, one document per alert.

use std::time::Duration;

use google_cloud_auth::credentials::{AccessTokenCredentials, Builder};
use serde_json::{Value, json};

use crate::consumer::BoxError;
use crate::{Alert, ConsumeError};

/// How long an alert is kept, after which Firestore's TTL policy on `expires_at`
/// will delete the document.
const RETENTION_SECS: i64 = 180 * 24 * 60 * 60; // 180 days

/// Stores alerts in the `alerts` collection of a project's Firestore database.
pub struct Firestore {
    documents_url: String,
    http_client: reqwest::Client,
    credentials: AccessTokenCredentials,
}

impl Firestore {
    /// Creates a handler that writes to the default database of `project`.
    ///
    /// Fails if the HTTP client can't be built, or no Google credentials are found.
    pub fn new(project: &str) -> Result<Self, BoxError> {
        Ok(Self {
            documents_url: format!(
                "https://firestore.googleapis.com/v1/projects/{project}/databases/(default)/documents"
            ),
            http_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()?,
            credentials: Builder::default().build_access_token_credentials()?,
        })
    }

    /// Adds `alert` to the `alerts` collection as a new document.
    pub async fn handle(&self, alert: &Alert) -> Result<(), ConsumeError> {
        // Obtain short-lived Google OAuth2 access token
        let token = self
            .credentials
            .access_token()
            .await
            .map_err(ConsumeError::Credentials)?
            .token;

        // Convert alert to Firestore document and upload to Firebase
        let response = self
            .http_client
            .post(format!("{}/alerts", self.documents_url))
            .bearer_auth(token)
            .json(&document(alert, common::unix_now())?)
            .send()
            .await
            .map_err(ConsumeError::FirestoreRequest)?;

        // Extract status code and return
        let status = response.status();
        if status.is_success() {
            Ok(())
        } else {
            let body = response.text().await.unwrap_or_default();
            Err(ConsumeError::FirestoreStatus { status, body })
        }
    }
}

/// Convert [`Alert`] to a Firestore REST document.
fn document(alert: &Alert, now: i64) -> Result<Value, ConsumeError> {
    let timestamp = |unix| common::rfc3339(unix).map_err(ConsumeError::Timestamp);
    let string_value = |text: &str| json!({ "stringValue": text });

    let extra: serde_json::Map<String, Value> = alert
        .extra
        .iter()
        .map(|(key, value)| (key.clone(), string_value(value)))
        .collect();

    let mut fields = json!({
        "source": string_value(&alert.source),
        "dedup_key": string_value(&alert.dedup_key),
        "summary": string_value(&alert.summary),
        "extra": { "mapValue": { "fields": extra } },
        "occurred_at": { "timestampValue": timestamp(alert.occurred_at_unix)? },
        "expires_at": { "timestampValue": timestamp(now + RETENTION_SECS)? },
    });
    if let Some(detail) = &alert.detail {
        fields["detail"] = string_value(detail);
    }
    Ok(json!({ "fields": fields }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_a_firestore_document() {
        let alert = Alert {
            source: "admin/lookup-sync".into(),
            dedup_key: "sync-failed".into(),
            occurred_at_unix: 1_790_864_909,
            summary: "Lookup sync failed".into(),
            detail: Some("timed out".into()),
            extra: [("group_id".into(), "105777".into())].into(),
        };
        assert_eq!(
            document(&alert, 0).unwrap(),
            json!({ "fields": {
                "source": { "stringValue": "admin/lookup-sync" },
                "dedup_key": { "stringValue": "sync-failed" },
                "summary": { "stringValue": "Lookup sync failed" },
                "detail": { "stringValue": "timed out" },
                "extra": { "mapValue": { "fields": {
                    "group_id": { "stringValue": "105777" },
                }}},
                "occurred_at": { "timestampValue": "2026-10-01T14:28:29Z" },
                "expires_at": { "timestampValue": "1970-06-30T00:00:00Z" },
            }})
        );
    }
}
