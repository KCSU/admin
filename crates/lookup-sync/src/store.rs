//! The Firestore document for a group snapshot.

use proto::lookup::v1::GroupSnapshot;
use serde_json::{Value, json};

/// The Firestore collection name alerts are stored in.
pub(crate) const FIRESTORE_COLLECTION_NAME: &str = "lookup_groups";

/// Convert [`GroupSnapshot`] to Firestore's REST document format.
pub(crate) fn fields(snapshot: &GroupSnapshot) -> Result<Value, common::TimestampOutOfRange> {
    let string_value = |text: &str| json!({ "stringValue": text });

    let members: Vec<Value> = snapshot
        .members
        .iter()
        .map(|member| {
            json!({ "mapValue": { "fields": {
                "crsid": string_value(&member.crsid),
                "name": string_value(&member.name),
            }}})
        })
        .collect();

    Ok(json!({
        "id": string_value(&snapshot.id),
        "name": string_value(&snapshot.name),
        "description": string_value(&snapshot.description),
        "fetched_at": { "timestampValue": common::rfc3339(snapshot.fetched_at_unix)? },
        "members": { "arrayValue": { "values": members } },
    }))
}
