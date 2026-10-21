//! The Firestore document for a group snapshot.

use common::Timestamp;
use proto::lookup::v1::GroupSnapshot;
use serde::Serialize;

/// The Firestore collection name alerts are stored in.
pub(crate) const FIRESTORE_COLLECTION_NAME: &str = "lookup_groups";

/// A [`GroupSnapshot`] as stored in Firestore.
#[derive(Serialize)]
pub(crate) struct GroupDocument<'a> {
    id: &'a str,
    name: &'a str,
    description: &'a str,
    fetched_at: Timestamp,
    members: Vec<MemberDocument<'a>>,
}

#[derive(Serialize)]
struct MemberDocument<'a> {
    crsid: &'a str,
    name: &'a str,
}

impl<'a> GroupDocument<'a> {
    pub(crate) fn new(snapshot: &'a GroupSnapshot) -> Result<Self, common::TimestampOutOfRange> {
        Ok(Self {
            id: &snapshot.id,
            name: &snapshot.name,
            description: &snapshot.description,
            fetched_at: Timestamp::from_unix(snapshot.fetched_at_unix)?,
            members: snapshot
                .members
                .iter()
                .map(|member| MemberDocument {
                    crsid: &member.crsid,
                    name: &member.name,
                })
                .collect(),
        })
    }
}
