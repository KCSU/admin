//! Editing the exec shown on kcsu.org.uk.

use std::sync::Arc;

use api::exec::{Exec, Role};
use axum::extract::State;
use axum::{Extension, Json};
use serde::Deserialize;

use crate::auth::User;
use crate::documents;
use crate::error::ErrorResponse;

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
    Ok(Json(documents::read::<Exec>(&firestore).await?.data))
}

/// `PUT /api/exec`: replaces the exec. Returns the exec as validated and trimmed.
pub async fn put(
    State(firestore): State<Arc<common::Firestore>>,
    Extension(user): Extension<User>,
    Json(update): Json<Update>,
) -> Result<Json<Exec>, ErrorResponse> {
    let current = documents::read::<Exec>(&firestore).await?;
    if current.data.updated_at != update.updated_at {
        return Err(documents::conflict());
    }
    let exec = Exec {
        updated_at: common::unix_now(),
        roles: update.roles,
    };
    let saved = documents::write(&firestore, current.version.as_deref(), exec).await?;
    eprintln!("{} updated the exec", user.email);
    Ok(Json(saved.data))
}
