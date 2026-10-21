//! A minimal client for Firestore's REST API.

use std::time::Duration;

use serde::de::DeserializeOwned;
use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::google::{GoogleClient, SendError, SetupError};
use crate::{TimestampOutOfRange, rfc3339};

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

    /// The write went through, but its response couldn't be read.
    #[error("reading Firestore's response to a successful write failed")]
    WrittenResponse(#[source] reqwest::Error),

    /// Firestore explains the failure in the response body.
    #[error("Firestore returned {status}: {body}")]
    Status {
        status: reqwest::StatusCode,
        /// The gRPC status from the body, e.g. "ABORTED", if it has one.
        code: Option<String>,
        body: String,
    },

    /// A Firestore [`Precondition`] wasn't met.
    #[error("the document changed since it was read")]
    Conflict,

    #[error("Firestore returned a document without an updateTime")]
    MissingUpdateTime,
}

impl From<SetupError> for FirestoreError {
    fn from(err: SetupError) -> Self {
        match err {
            SetupError::HttpClient(err) => Self::HttpClient(err),
            SetupError::Credentials(err) => Self::CredentialsSetup(err),
        }
    }
}

impl From<SendError> for FirestoreError {
    fn from(err: SendError) -> Self {
        match err {
            SendError::AccessToken(err) => Self::AccessToken(err),
            SendError::Request(err) => Self::Request(err),
        }
    }
}

/// A document read from Firestore.
pub struct Document {
    /// The document's ID within its collection.
    pub id: String,
    /// The document's contents in Firestore's REST format. See [`from_fields`].
    pub fields: Value,
    /// When the document was last written, e.g. "2026-10-12T20:30:01.123456Z".
    pub update_time: String,
}

impl Document {
    /// The document's fields as a `T`, e.g. a struct. See [`from_fields`].
    pub fn decode<T: DeserializeOwned>(&self) -> serde_json::Result<T> {
        serde_json::from_value(from_fields(&self.fields))
    }
}

/// The document a write saved.
#[derive(Debug)]
pub struct Written {
    /// The document's ID within its collection.
    pub id: String,
    /// The document's new `update_time`.
    pub update_time: String,
}

/// What a document must look like for a write to go ahead.
pub enum Precondition<'a> {
    /// The document must not exist.
    Missing,
    /// The document must not have been written since this `update_time` was read.
    UpdatedAt(&'a str),
}

/// Reads and writes documents in the default Firestore database of a project.
pub struct Firestore {
    documents_url: String,
    client: GoogleClient,
}

impl Firestore {
    /// Creates a client for the default database of `project`.
    pub fn new(project: &str) -> Result<Self, FirestoreError> {
        Ok(Self {
            documents_url: format!(
                "https://firestore.googleapis.com/v1/projects/{project}/databases/(default)/documents"
            ),
            client: GoogleClient::new(Duration::from_secs(10))?,
        })
    }

    /// Adds a new document `fields` to `collection`.
    ///
    /// `fields` is the document's contents in Firestore's REST format.
    /// See <https://firebase.google.com/docs/firestore/use-rest-api> for more details.
    pub async fn add(&self, collection: &str, fields: Value) -> Result<Written, FirestoreError> {
        let url = format!("{}/{collection}", self.documents_url);
        self.write(self.client.http.post(url), fields).await
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
    ) -> Result<Written, FirestoreError> {
        self.patch(collection, id, fields, None).await
    }

    /// Like [`set`](Self::set), but only if the document meets `precondition`.
    ///
    /// Fails with [`FirestoreError::Conflict`] if it doesn't.
    pub async fn set_if(
        &self,
        collection: &str,
        id: &str,
        fields: Value,
        precondition: Precondition<'_>,
    ) -> Result<Written, FirestoreError> {
        self.patch(collection, id, fields, Some(precondition)).await
    }

    /// Reads the document `collection/id`, or `None` if it doesn't exist.
    pub async fn get(
        &self,
        collection: &str,
        id: &str,
    ) -> Result<Option<Document>, FirestoreError> {
        let request = self.client.http.get(self.document_url(collection, id));
        let response = self.send(request).await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let document: Value = check_status(response)
            .await?
            .json()
            .await
            .map_err(FirestoreError::Request)?;
        document_from_rest(document).map(Some)
    }

    /// Reads up to `limit` documents in `collection` whose timestamp field `field` is at or
    /// after `since` (RFC 3339), newest first, in one request.
    pub async fn recent(
        &self,
        collection: &str,
        field: &str,
        since: &str,
        limit: u32,
    ) -> Result<Vec<Document>, FirestoreError> {
        let query = json!({ "structuredQuery": {
            "from": [{ "collectionId": collection }],
            "where": { "fieldFilter": {
                "field": { "fieldPath": field },
                "op": "GREATER_THAN_OR_EQUAL",
                "value": { "timestampValue": since },
            }},
            "orderBy": [{ "field": { "fieldPath": field }, "direction": "DESCENDING" }],
            "limit": limit,
        }});
        let request = self
            .client
            .http
            .post(format!("{}:runQuery", self.documents_url))
            .json(&query);
        let results: Vec<Value> = check_status(self.send(request).await?)
            .await?
            .json()
            .await
            .map_err(FirestoreError::Request)?;

        // An empty result is a single entry with only a `readTime`, and no `document`.
        results
            .into_iter()
            .filter_map(|mut result| match result["document"].take() {
                Value::Null => None,
                document => Some(document_from_rest(document)),
            })
            .collect()
    }

    fn document_url(&self, collection: &str, id: &str) -> String {
        format!("{}/{collection}/{id}", self.documents_url)
    }

    /// Creates or replaces the document `collection/id`, if it meets `precondition`.
    async fn patch(
        &self,
        collection: &str,
        id: &str,
        fields: Value,
        precondition: Option<Precondition<'_>>,
    ) -> Result<Written, FirestoreError> {
        let mut request = self.client.http.patch(self.document_url(collection, id));
        let conditional = precondition.is_some();
        if let Some(precondition) = precondition {
            let (condition, value) = match precondition {
                Precondition::Missing => ("currentDocument.exists", "false"),
                Precondition::UpdatedAt(time) => ("currentDocument.updateTime", time),
            };
            request = request.query(&[(condition, value)]);
        }
        match self.write(request, fields).await {
            Err(FirestoreError::Status {
                code: Some(code), ..
            }) if conditional && (code == "FAILED_PRECONDITION" || code == "ALREADY_EXISTS") => {
                Err(FirestoreError::Conflict)
            }
            result => result,
        }
    }

    /// Sends `request` with `fields` as the document's contents.
    async fn write(
        &self,
        request: reqwest::RequestBuilder,
        fields: Value,
    ) -> Result<Written, FirestoreError> {
        let request = request.json(&json!({ "fields": fields }));
        // The response's body is the written document.
        let document: Value = check_status(self.send(request).await?)
            .await?
            .json()
            .await
            .map_err(FirestoreError::WrittenResponse)?;
        let document = document_from_rest(document)?;
        Ok(Written {
            id: document.id,
            update_time: document.update_time,
        })
    }

    /// Sends `request` with an access token. The caller checks the response's status.
    async fn send(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<reqwest::Response, FirestoreError> {
        Ok(self.client.send(request).await?)
    }
}

/// A [`Document`] from the REST API's representation of one.
fn document_from_rest(mut document: Value) -> Result<Document, FirestoreError> {
    let update_time = document["updateTime"]
        .as_str()
        .ok_or(FirestoreError::MissingUpdateTime)?
        .to_owned();
    // `name` is the full path, ending in the collection and the document's ID.
    let id = document["name"]
        .as_str()
        .and_then(|name| name.rsplit('/').next())
        .unwrap_or_default()
        .to_owned();
    // An empty document has no fields at all.
    let fields = match document["fields"].take() {
        Value::Null => json!({}),
        fields => fields,
    };
    Ok(Document {
        id,
        fields,
        update_time,
    })
}

/// `response` if it succeeded, else a [`FirestoreError::Status`] with its gRPC status.
async fn check_status(response: reqwest::Response) -> Result<reqwest::Response, FirestoreError> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }
    let body = response.text().await.unwrap_or_default();
    Err(FirestoreError::Status {
        status,
        code: grpc_status(&body),
        body,
    })
}

/// The gRPC status in a Firestore error body, e.g. "ABORTED" from
/// `{"error": {"code": 409, "status": "ABORTED", ...}}`.
fn grpc_status(body: &str) -> Option<String> {
    let body: Value = serde_json::from_str(body).ok()?;
    Some(body["error"]["status"].as_str()?.to_owned())
}

/// Marks a [`Timestamp`], which would otherwise be just a string in [`serde_json::Value`].
const TIMESTAMP_KEY: &str = "__firestore_timestamp__";

/// A timestamp field in a document written with [`to_fields`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Timestamp(String);

impl Timestamp {
    pub fn from_unix(unix: i64) -> Result<Self, TimestampOutOfRange> {
        rfc3339(unix).map(Self)
    }

    /// The time in RFC 3339.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Serialize for Timestamp {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry(TIMESTAMP_KEY, &self.0)?;
        map.end()
    }
}

impl<'de> Deserialize<'de> for Timestamp {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer).map(Self)
    }
}

/// Converts `value`, e.g. a struct, into Firestore's REST format for a document's fields.
///
/// Integers become `integerValue`, other numbers `doubleValue`, [`Timestamp`]s
/// `timestampValue`, and other objects `mapValue`. Fails if `value` doesn't serialize to a
/// JSON object.
pub fn to_fields(value: &impl Serialize) -> serde_json::Result<Value> {
    match serde_json::to_value(value)? {
        Value::Object(object) => Ok(object_to_fields(&object)),
        _ => Err(serde::ser::Error::custom(
            "a document's fields must be an object",
        )),
    }
}

fn object_to_fields(object: &Map<String, Value>) -> Value {
    Value::Object(
        object
            .iter()
            .map(|(key, value)| (key.clone(), to_firestore(value)))
            .collect(),
    )
}

/// Converts a document's fields in Firestore's REST format back into a JSON object.
///
/// The inverse of [`to_fields`]. Types it never writes come back as `null`.
pub fn from_fields(fields: &Value) -> Value {
    Value::Object(
        fields
            .as_object()
            .map(|fields| {
                fields
                    .iter()
                    .map(|(key, value)| (key.clone(), from_firestore(value)))
                    .collect()
            })
            .unwrap_or_default(),
    )
}

fn to_firestore(value: &Value) -> Value {
    match value {
        Value::Null => json!({ "nullValue": null }),
        Value::Bool(b) => json!({ "booleanValue": b }),
        Value::Number(n) => match n.as_i64() {
            Some(i) => json!({ "integerValue": i.to_string() }),
            None => json!({ "doubleValue": n }),
        },
        Value::String(s) => json!({ "stringValue": s }),
        Value::Array(items) => {
            let values: Vec<Value> = items.iter().map(to_firestore).collect();
            json!({ "arrayValue": { "values": values } })
        }
        Value::Object(object) => match timestamp(object) {
            Some(time) => json!({ "timestampValue": time }),
            None => json!({ "mapValue": { "fields": object_to_fields(object) } }),
        },
    }
}

/// The RFC 3339 time in `object`, if it's a serialized [`Timestamp`].
fn timestamp(object: &Map<String, Value>) -> Option<&Value> {
    match object.len() {
        1 => object.get(TIMESTAMP_KEY),
        _ => None,
    }
}

fn from_firestore(value: &Value) -> Value {
    let Some((kind, inner)) = value.as_object().and_then(|value| value.iter().next()) else {
        return Value::Null;
    };
    match kind.as_str() {
        // The REST API sends 64-bit integers as strings.
        "integerValue" => inner
            .as_i64()
            .or_else(|| inner.as_str()?.parse().ok())
            .map_or(Value::Null, Value::from),
        "booleanValue" | "doubleValue" | "stringValue" | "timestampValue" => inner.clone(),
        // An empty array or map has no `values` or `fields`.
        "arrayValue" => Value::Array(
            inner["values"]
                .as_array()
                .map(|values| values.iter().map(from_firestore).collect())
                .unwrap_or_default(),
        ),
        "mapValue" => from_fields(&inner["fields"]),
        _ => Value::Null,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_to_and_from_firestore_fields() {
        let object = json!({
            "name": "KCSU Website",
            "ok": true,
            "latency_ms": 120,
            "ratio": 0.5,
            "error": null,
            "tags": ["a", "b"],
            "days": { "2026-10-12": { "up": 1, "total": 2 } },
        });
        let fields = to_fields(&object).unwrap();
        assert_eq!(fields["latency_ms"], json!({ "integerValue": "120" }));
        assert_eq!(fields["ratio"], json!({ "doubleValue": 0.5 }));
        assert_eq!(fields["error"], json!({ "nullValue": null }));
        assert_eq!(from_fields(&fields), object);
    }

    #[test]
    fn converts_timestamps() {
        let at = Timestamp::from_unix(1_790_864_909).unwrap();
        let fields = to_fields(&json!({ "at": at, "nested": { "at": at } })).unwrap();
        assert_eq!(
            fields["at"],
            json!({ "timestampValue": "2026-10-01T14:28:29Z" })
        );
        assert_eq!(
            fields["nested"]["mapValue"]["fields"]["at"],
            json!({ "timestampValue": "2026-10-01T14:28:29Z" })
        );

        #[derive(Deserialize)]
        struct Read {
            at: Timestamp,
        }
        let read: Read = serde_json::from_value(from_fields(&fields)).unwrap();
        assert_eq!(read.at, at);
    }

    #[test]
    fn reads_a_document_from_rest() {
        let document = document_from_rest(json!({
            "name": "projects/p/databases/(default)/documents/alerts/abc123",
            "fields": { "summary": { "stringValue": "Down" } },
            "updateTime": "2026-10-17T12:00:00.123456Z",
        }))
        .unwrap();
        assert_eq!(document.id, "abc123");
        assert_eq!(document.update_time, "2026-10-17T12:00:00.123456Z");
        assert_eq!(
            document.decode::<Value>().unwrap(),
            json!({ "summary": "Down" })
        );

        // An empty document has no `fields`.
        let empty = document_from_rest(json!({ "name": "x/y", "updateTime": "t" })).unwrap();
        assert_eq!(empty.fields, json!({}));

        assert!(matches!(
            document_from_rest(json!({ "name": "x/y" })),
            Err(FirestoreError::MissingUpdateTime)
        ));
    }
}
