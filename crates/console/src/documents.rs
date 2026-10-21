//! Editing whole Firestore documents, with optimistic concurrency control.

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::{Extension, Json};
use common::{FirestoreError, Precondition};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::auth::User;
use crate::error::{ErrorResponse, error};

/// A Firestore document the console can edit.
pub trait Editable: Default + DeserializeOwned + Serialize {
    const COLLECTION: &'static str;
    const DOCUMENT_ID: &'static str;

    fn validate(self) -> Result<Self, String>;
}

/// A document and the version it was read at.
#[derive(Deserialize, Serialize)]
pub struct Versioned<T> {
    /// Firestore's `update_time`. `None` if never written.
    pub version: Option<String>,
    pub data: T,
}

/// `GET`: the document, uncached.
pub async fn get<T: Editable>(
    State(firestore): State<Arc<common::Firestore>>,
) -> Result<Json<Versioned<T>>, ErrorResponse> {
    Ok(Json(read(&firestore).await?))
}

/// `PUT`: replaces the document if it's still at `version`.
pub async fn put<T: Editable>(
    State(firestore): State<Arc<common::Firestore>>,
    Extension(user): Extension<User>,
    Json(edit): Json<Versioned<T>>,
) -> Result<Json<Versioned<T>>, ErrorResponse> {
    let saved = write(&firestore, edit.version.as_deref(), edit.data).await?;
    eprintln!("{} updated {}", user.email, path::<T>());
    Ok(Json(saved))
}

/// The stored document, or the default if never written.
pub async fn read<T: Editable>(
    firestore: &common::Firestore,
) -> Result<Versioned<T>, ErrorResponse> {
    let document = firestore
        .get(T::COLLECTION, T::DOCUMENT_ID)
        .await
        .map_err(|err| failed::<T>("reading", &err))?;
    match document {
        Some(document) => Ok(Versioned {
            data: document
                .decode()
                .map_err(|err| failed::<T>("decoding", &err))?,
            version: Some(document.update_time),
        }),
        None => Ok(Versioned {
            version: None,
            data: T::default(),
        }),
    }
}

/// Validates `data`, then saves it if the document is still at `version`.
pub async fn write<T: Editable>(
    firestore: &common::Firestore,
    version: Option<&str>,
    data: T,
) -> Result<Versioned<T>, ErrorResponse> {
    let data = data
        .validate()
        .map_err(|message| error(StatusCode::BAD_REQUEST, &message))?;
    let precondition = match version {
        Some(time) => Precondition::UpdatedAt(time),
        None => Precondition::Missing,
    };
    let fields =
        common::firestore::to_fields(&data).map_err(|err| failed::<T>("encoding", &err))?;
    match firestore
        .set_if(T::COLLECTION, T::DOCUMENT_ID, fields, precondition)
        .await
    {
        Ok(written) => Ok(Versioned {
            version: Some(written.update_time),
            data,
        }),
        Err(FirestoreError::Conflict) => Err(conflict()),
        Err(err) => Err(failed::<T>("writing", &err)),
    }
}

pub fn conflict() -> ErrorResponse {
    error(
        StatusCode::CONFLICT,
        "Someone else published changes since you opened this. Reload to see them.",
    )
}

fn failed<T: Editable>(doing: &str, err: &(dyn std::error::Error + 'static)) -> ErrorResponse {
    eprintln!("{doing} {} failed: {}", path::<T>(), common::report(err));
    error(
        StatusCode::BAD_GATEWAY,
        "Couldn't reach the database. Please try again.",
    )
}

/// The document's path, e.g. "status/notices", for logs.
fn path<T: Editable>() -> String {
    format!("{}/{}", T::COLLECTION, T::DOCUMENT_ID)
}

impl Editable for api::exec::Exec {
    const COLLECTION: &'static str = api::exec::COLLECTION;
    const DOCUMENT_ID: &'static str = api::exec::DOCUMENT_ID;

    fn validate(self) -> Result<Self, String> {
        Ok(Self {
            updated_at: self.updated_at,
            roles: api::exec::validate_roles(self.roles)?,
        })
    }
}

impl Editable for api::notices::Notices {
    const COLLECTION: &'static str = api::notices::COLLECTION;
    const DOCUMENT_ID: &'static str = api::notices::DOCUMENT_ID;

    fn validate(self) -> Result<Self, String> {
        api::notices::validate_notices(self)
    }
}
