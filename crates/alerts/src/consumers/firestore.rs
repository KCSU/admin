//! Persists alerts in Firestore, one document per alert.

use serde_json::{Value, json};

use crate::{Alert, ConsumeError};

/// The Firestore collection name alerts are stored in.
const FIRESTORE_COLLECTION_NAME: &str = "alerts";

/// How long an alert is kept, after which Firestore's TTL policy on `expires_at`
/// will delete the document.
const RETENTION_SECS: i64 = 180 * 24 * 60 * 60; // 180 days

/// Stores alerts in the `alerts` collection of a project's Firestore database.
pub struct Firestore {
    client: common::Firestore,
}

impl Firestore {
    /// Creates a handler that writes to the default database of `project`.
    ///
    /// Fails if the HTTP client can't be built, or no Google credentials are found.
    pub fn new(project: &str) -> Result<Self, common::FirestoreError> {
        Ok(Self {
            client: common::Firestore::new(project)?,
        })
    }

    /// Adds `alert` to the `alerts` collection as a new document.
    pub async fn handle(&self, alert: &Alert) -> Result<(), ConsumeError> {
        let fields = fields(alert, common::unix_now())?;
        self.client
            .add(FIRESTORE_COLLECTION_NAME, fields)
            .await
            .map_err(ConsumeError::Firestore)
    }
}

/// Convert [`Alert`] to Firestore's REST document format.
fn fields(alert: &Alert, now: i64) -> Result<Value, ConsumeError> {
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
    Ok(fields)
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
            fields(&alert, 0).unwrap(),
            json!({
                "source": { "stringValue": "admin/lookup-sync" },
                "dedup_key": { "stringValue": "sync-failed" },
                "summary": { "stringValue": "Lookup sync failed" },
                "detail": { "stringValue": "timed out" },
                "extra": { "mapValue": { "fields": {
                    "group_id": { "stringValue": "105777" },
                }}},
                "occurred_at": { "timestampValue": "2026-10-01T14:28:29Z" },
                "expires_at": { "timestampValue": "1970-06-30T00:00:00Z" },
            })
        );
    }
}
