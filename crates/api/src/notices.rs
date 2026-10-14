//! Notices on the status page: planned maintenance, incidents and general information.

use serde::{Deserialize, Serialize};

/// The Firestore document holding every notice.
pub(crate) const COLLECTION: &str = "status";
pub(crate) const DOCUMENT_ID: &str = "notices";

/// The `status/notices` document.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(crate) struct Notices {
    /// Newest first.
    pub notices: Vec<Notice>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub(crate) struct Notice {
    /// Stable, unique ID.
    pub id: String,
    pub kind: Kind,
    pub title: String,
    pub body: Option<String>,
    /// IDs of the uptime sites it affects, e.g. ["store"]. Can be empty.
    pub site_ids: Vec<String>,
    /// Unix seconds. The status page hides the notice until then. None if it has no set start.
    pub starts_at: Option<i64>,
    /// Unix seconds. The status page hides the notice from then on. None if it has no set end.
    pub ends_at: Option<i64>,
    /// Oldest first.
    pub updates: Vec<Update>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Kind {
    Maintenance,
    Incident,
    #[default]
    Info,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub(crate) struct Update {
    /// Unix seconds.
    pub at: i64,
    pub text: String,
}

impl Notices {
    /// Parses the document's fields, in Firestore's REST format.
    pub fn from_fields(fields: &serde_json::Value) -> serde_json::Result<Self> {
        serde_json::from_value(common::firestore::from_fields(fields))
    }
}

/// The stored notices, or none if the document hasn't been written yet.
pub(crate) async fn read(
    firestore: &common::Firestore,
) -> Result<Notices, Box<dyn std::error::Error + Send + Sync>> {
    Ok(match firestore.get(COLLECTION, DOCUMENT_ID).await? {
        Some(document) => Notices::from_fields(&document.fields)?,
        None => Notices::default(),
    })
}
