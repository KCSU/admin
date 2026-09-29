//! Lookup API response types.

use proto::lookup::v1::group_snapshot::Member;

/// Every Lookup response wraps its payload in `{ "result": ... }`.
#[derive(Debug, serde::Deserialize)]
pub struct LookupResponse<T> {
    pub result: T,
}

/// Payload of `GET group/{id}`.
#[derive(Debug, serde::Deserialize)]
pub struct LookupGroupResult {
    pub group: LookupGroup,
}

#[derive(Debug, serde::Deserialize)]
pub struct LookupGroup {
    pub name: String,
    pub title: String,
}

/// Payload of `GET group/{id}/members`.
#[derive(Debug, serde::Deserialize)]
pub struct LookupMembers {
    pub people: Vec<LookupPerson>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LookupPerson {
    pub identifier: LookupIdentifier,
    #[serde(default)]
    pub cancelled: bool,
    pub visible_name: Option<String>,
    pub display_name: Option<String>,
    pub registered_name: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct LookupIdentifier {
    pub scheme: String,
    pub value: String,
}

impl LookupPerson {
    /// Convert to a member as defined in the protobuf schema, or None.
    pub fn into_proto_member(self) -> Option<Member> {
        // Member must have a CRSid.
        if self.cancelled || self.identifier.scheme != "crsid" {
            return None;
        }
        let crsid = self.identifier.value.trim().to_owned();
        if crsid.is_empty() {
            return None;
        }

        // Fallback in order of: visible name, display name, registered name, CRSid.
        let name = [self.visible_name, self.display_name, self.registered_name]
            .into_iter()
            .flatten()
            .map(|name| name.trim().to_owned())
            .find(|name| !name.is_empty())
            .unwrap_or_else(|| crsid.clone());

        Some(Member { crsid, name })
    }
}
