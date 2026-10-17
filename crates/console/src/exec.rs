//! Editing the exec shown on kcsu.org.uk.

use std::sync::Arc;

use api::exec::{COLLECTION, DOCUMENT_ID, Exec, Role};
use axum::extract::State;
use axum::http::StatusCode;
use axum::{Extension, Json};
use common::{FirestoreError, Precondition};
use serde::Deserialize;

use crate::auth::User;
use crate::error::{ErrorResponse, error};

/// The body of `PUT /api/exec`.
#[derive(Deserialize)]
pub struct Update {
    /// The Unix timestamp of the exec the edit started from. 0 if there was none.
    updated_at: i64,
    roles: Vec<Role>,
}

/// `GET /api/exec`: the current exec, uncached.
pub async fn get(
    State(firestore): State<Arc<common::Firestore>>,
) -> Result<Json<Exec>, ErrorResponse> {
    let (exec, _) = read(&firestore).await?;
    Ok(Json(exec))
}

/// `PUT /api/exec`: replaces the exec. Returns it as validated and trimmed.
pub async fn put(
    State(firestore): State<Arc<common::Firestore>>,
    Extension(user): Extension<User>,
    Json(update): Json<Update>,
) -> Result<Json<Exec>, ErrorResponse> {
    let roles = api::exec::validate_roles(update.roles)
        .map_err(|message| error(StatusCode::BAD_REQUEST, &message))?;

    let (current, update_time) = read(&firestore).await?;
    if current.updated_at != update.updated_at {
        return Err(conflict());
    }
    let precondition = match &update_time {
        Some(time) => Precondition::UpdatedAt(time),
        None => Precondition::Missing,
    };

    let exec = Exec {
        updated_at: common::unix_now(),
        roles,
    };
    let fields =
        common::firestore::to_fields(&exec).map_err(|err| failed("encoding the exec", &err))?;
    match firestore
        .set_if(COLLECTION, DOCUMENT_ID, fields, precondition)
        .await
    {
        Ok(()) => {}
        // Someone published between our read and write.
        Err(FirestoreError::Conflict) => return Err(conflict()),
        Err(err) => return Err(failed("writing the exec", &err)),
    }
    eprintln!("{} updated the exec", user.email);
    Ok(Json(exec))
}

/// The stored exec and its document's `update_time`, or no roles and `None` if it hasn't
/// been written yet.
async fn read(firestore: &common::Firestore) -> Result<(Exec, Option<String>), ErrorResponse> {
    let document = firestore
        .get(COLLECTION, DOCUMENT_ID)
        .await
        .map_err(|err| failed("reading the exec", &err))?;
    match document {
        Some(document) => {
            let exec = document
                .decode()
                .map_err(|err| failed("decoding the exec", &err))?;
            Ok((exec, Some(document.update_time)))
        }
        None => Ok((Exec::default(), None)),
    }
}

fn conflict() -> ErrorResponse {
    error(
        StatusCode::CONFLICT,
        "Someone else published changes since you opened the exec. Reload to see them.",
    )
}

fn failed(doing: &str, err: &(dyn std::error::Error + 'static)) -> ErrorResponse {
    eprintln!("{doing} failed: {}", common::report(err));
    error(
        StatusCode::BAD_GATEWAY,
        "The exec couldn't be reached. Please try again.",
    )
}
