//! A minimal client for Firestore's REST API.

use std::time::Duration;

use google_cloud_auth::credentials::{AccessTokenCredentials, Builder};
use serde_json::{Map, Value, json};

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

/// A document read from Firestore.
pub struct Document {
    /// The document's contents in Firestore's REST format. See [`from_fields`].
    pub fields: Value,
    /// When the document was last written, e.g. "2026-10-12T20:30:01.123456Z".
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

    /// Like [`set`](Self::set), but only if the document meets `precondition`.
    ///
    /// Fails with [`FirestoreError::Conflict`] if it doesn't.
    pub async fn set_if(
        &self,
        collection: &str,
        id: &str,
        fields: Value,
        precondition: Precondition<'_>,
    ) -> Result<(), FirestoreError> {
        let (condition, value) = match precondition {
            Precondition::Missing => ("currentDocument.exists", "false"),
            Precondition::UpdatedAt(time) => ("currentDocument.updateTime", time),
        };
        let url = format!("{}/{collection}/{id}", self.documents_url);
        let request = self.http_client.patch(url).query(&[(condition, value)]);
        match self.send(request, fields).await {
            Err(FirestoreError::Status {
                code: Some(code), ..
            }) if code == "FAILED_PRECONDITION" || code == "ALREADY_EXISTS" => {
                Err(FirestoreError::Conflict)
            }
            result => result,
        }
    }

    /// Reads the document `collection/id`, or `None` if it doesn't exist.
    pub async fn get(
        &self,
        collection: &str,
        id: &str,
    ) -> Result<Option<Document>, FirestoreError> {
        let url = format!("{}/{collection}/{id}", self.documents_url);
        let response = self
            .http_client
            .get(url)
            .bearer_auth(self.token().await?)
            .send()
            .await
            .map_err(FirestoreError::Request)?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let response = check_status(response).await?;

        let mut document: Value = response.json().await.map_err(FirestoreError::Request)?;
        let update_time = document["updateTime"]
            .as_str()
            .ok_or(FirestoreError::MissingUpdateTime)?
            .to_owned();
        // An empty document has no fields at all.
        let fields = match document["fields"].take() {
            Value::Null => json!({}),
            fields => fields,
        };
        Ok(Some(Document {
            fields,
            update_time,
        }))
    }

    /// A short-lived Google OAuth2 access token.
    async fn token(&self) -> Result<String, FirestoreError> {
        Ok(self
            .credentials
            .access_token()
            .await
            .map_err(FirestoreError::AccessToken)?
            .token)
    }

    async fn send(
        &self,
        request: reqwest::RequestBuilder,
        fields: Value,
    ) -> Result<(), FirestoreError> {
        let response = request
            .bearer_auth(self.token().await?)
            .json(&json!({ "fields": fields }))
            .send()
            .await
            .map_err(FirestoreError::Request)?;
        check_status(response).await?;
        Ok(())
    }
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

/// Converts a JSON object into Firestore's REST format for a document's fields.
///
/// Integers become `integerValue`, other numbers `doubleValue`, and objects `mapValue`.
pub fn to_fields(object: &Map<String, Value>) -> Value {
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
        Value::Object(object) => json!({ "mapValue": { "fields": to_fields(object) } }),
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
        "booleanValue" | "doubleValue" | "stringValue" => inner.clone(),
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
        let fields = to_fields(object.as_object().unwrap());
        assert_eq!(fields["latency_ms"], json!({ "integerValue": "120" }));
        assert_eq!(fields["ratio"], json!({ "doubleValue": 0.5 }));
        assert_eq!(fields["error"], json!({ "nullValue": null }));
        assert_eq!(from_fields(&fields), object);
    }
}
