//! Notices on the status page: planned maintenance, incidents and general information.

use serde::{Deserialize, Serialize};

mod validate;

pub use validate::notices as validate_notices;

/// The Firestore document holding every notice.
pub const COLLECTION: &str = "status";
pub const DOCUMENT_ID: &str = "notices";

/// The `status/notices` document.
#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Notices {
    /// Newest first.
    pub notices: Vec<Notice>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Notice {
    /// A UUIDv7. Empty only in an unvalidated edit of a new notice.
    pub id: String,
    pub kind: Kind,
    pub title: String,
    pub body: Option<String>,
    /// IDs of the uptime sites it affects, e.g. ["store"]. Can be empty.
    pub site_ids: Vec<String>,
    /// Unix seconds. The status page hides the notice until then.
    pub starts_at: i64,
    /// Unix seconds. The status page hides the notice from then on. None if it has no set end.
    pub ends_at: Option<i64>,
    /// Oldest first.
    pub updates: Vec<Update>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Maintenance,
    Incident,
    #[default]
    Info,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Update {
    /// Unix seconds.
    pub at: i64,
    pub text: String,
}

/// The stored notices, or none if the document hasn't been written yet.
pub(crate) async fn read(
    firestore: &common::Firestore,
) -> Result<Notices, Box<dyn std::error::Error + Send + Sync>> {
    Ok(match firestore.get(COLLECTION, DOCUMENT_ID).await? {
        Some(document) => document.decode()?,
        None => Notices::default(),
    })
}
