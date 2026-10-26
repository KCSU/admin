//! Publicly viewable information about the current KCSU exec.

use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

pub mod photo;
mod validate;

pub use validate::roles as validate_roles;

/// The Firestore document holding the current exec.
pub const COLLECTION: &str = "exec";
pub const DOCUMENT_ID: &str = "current";

const CACHE_CONTROL: &str = "public, max-age=300";
const ALLOW_ORIGIN: &str = "*";

/// The current exec, as stored in Firestore and served by the API.
#[derive(Debug, Default, PartialEq, Deserialize, Serialize)]
#[serde(default)]
pub struct Exec {
    /// When the exec was last edited, in Unix seconds.
    pub updated_at: i64,
    /// All roles, in display order.
    pub roles: Vec<Role>,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
#[serde(default)]
pub struct Role {
    /// A stable identifier, e.g. "president".
    pub id: String,
    /// e.g. "President".
    pub title: String,
    pub emails: Vec<String>,
    /// People currently holding the role. Empty while it's vacant.
    pub holders: Vec<Holder>,
    pub bio: Option<String>,
    /// A public URL, in our bucket. See [`photo`].
    pub photo_url: Option<String>,
    /// Instagram handles without the leading "@".
    pub instagram: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
#[serde(default)]
pub struct Holder {
    pub name: String,
}

pub(crate) async fn get(State(firestore): State<Arc<common::Firestore>>) -> Response {
    let exec = match read(&firestore).await {
        Ok(exec) => exec,
        Err(err) => {
            log::error!(
                "reading exec data from Firestore failed: {}",
                common::report(&*err)
            );
            return (
                StatusCode::BAD_GATEWAY,
                [(header::ACCESS_CONTROL_ALLOW_ORIGIN, ALLOW_ORIGIN)],
            )
                .into_response();
        }
    };
    (
        [
            (header::CACHE_CONTROL, CACHE_CONTROL),
            (header::ACCESS_CONTROL_ALLOW_ORIGIN, ALLOW_ORIGIN),
        ],
        Json(exec),
    )
        .into_response()
}

/// The stored exec in Firestore, or no roles if it hasn't been written yet.
async fn read(
    firestore: &common::Firestore,
) -> Result<Exec, Box<dyn std::error::Error + Send + Sync>> {
    Ok(match firestore.get(COLLECTION, DOCUMENT_ID).await? {
        Some(document) => document.decode()?,
        None => Exec::default(),
    })
}
