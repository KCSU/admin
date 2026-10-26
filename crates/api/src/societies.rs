//! Publicly viewable information about King's societies.

use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

/// The Firestore document holding the societies.
pub const COLLECTION: &str = "societies";
pub const DOCUMENT_ID: &str = "current";

const CACHE_CONTROL: &str = "public, max-age=300";
const ALLOW_ORIGIN: &str = "*";

/// All societies, as stored in Firestore and served by the API.
#[derive(Debug, Default, PartialEq, Deserialize, Serialize)]
#[serde(default)]
pub struct Societies {
    /// When the societies were last edited, in Unix seconds.
    pub updated_at: i64,
    /// All societies, in display order.
    pub societies: Vec<Society>,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
#[serde(default)]
pub struct Society {
    /// A stable identifier, e.g. "kings-college-boat-club".
    pub id: String,
    /// e.g. "King's College Boat Club".
    pub name: String,
    pub category: Category,
    pub description: Option<String>,
    pub contacts: Vec<Contact>,
    pub photo_url: Option<String>,
    pub website: Option<String>,
    /// An Instagram handle without the leading "@".
    pub instagram: Option<String>,
    /// A link to the society's calendar.
    pub calendar: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Academic,
    Sports,
    #[default]
    Other,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
#[serde(default)]
pub struct Contact {
    /// e.g. "General Enquiries" or "President". `None` for a plain contact address.
    pub label: Option<String>,
    pub email: String,
}

pub(crate) async fn get(State(firestore): State<Arc<common::Firestore>>) -> Response {
    let societies = match read(&firestore).await {
        Ok(societies) => societies,
        Err(err) => {
            eprintln!("reading the societies failed: {}", common::report(&*err));
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
        Json(societies),
    )
        .into_response()
}

/// The stored societies in Firestore, or none if they haven't been written yet.
async fn read(
    firestore: &common::Firestore,
) -> Result<Societies, Box<dyn std::error::Error + Send + Sync>> {
    Ok(match firestore.get(COLLECTION, DOCUMENT_ID).await? {
        Some(document) => document.decode()?,
        None => Societies::default(),
    })
}
