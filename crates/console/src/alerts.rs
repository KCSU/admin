//! The alerts KCSU's services have raised, obtained from Firestore.

use std::collections::BTreeMap;
use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};

use crate::error::{ErrorResponse, error};

const COLLECTION: &str = "alerts";

/// Days of alerts returned by default.
const DEFAULT_DAYS: i64 = 30;
/// Alerts are only kept this long.
const MAX_DAYS: i64 = 180;
/// At most this many alerts per response.
const MAX_ALERTS: u32 = 1000;

#[derive(Debug, Deserialize, Serialize)]
struct Alert {
    /// The Firestore document ID.
    #[serde(skip_deserializing)]
    id: String,
    source: String,
    dedup_key: String,
    summary: String,
    #[serde(default)]
    detail: Option<String>,
    #[serde(default)]
    extra: BTreeMap<String, String>,
    /// RFC 3339.
    occurred_at: String,
    /// RFC 3339. Firestore's TTL policy deletes the alert then.
    expires_at: String,
}

#[derive(Deserialize)]
pub struct ListQuery {
    days: Option<i64>,
}

/// The body of `GET /api/alerts`.
#[derive(Serialize)]
pub struct ListResponse {
    /// Newest first.
    alerts: Vec<Alert>,
}

/// `GET /api/alerts?days=30`: the alerts of the last `days` days, newest first.
pub async fn list(
    State(firestore): State<Arc<common::Firestore>>,
    Query(query): Query<ListQuery>,
) -> Result<Json<ListResponse>, ErrorResponse> {
    let days = query.days.unwrap_or(DEFAULT_DAYS).clamp(1, MAX_DAYS);
    let since = common::rfc3339(common::unix_now() - days * 24 * 60 * 60).map_err(|_| {
        error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "the clock is out of range",
        )
    })?;

    let documents = firestore
        .recent(COLLECTION, "occurred_at", &since, MAX_ALERTS)
        .await
        .map_err(|err| {
            eprintln!("listing alerts failed: {}", common::report(&err));
            error(StatusCode::BAD_GATEWAY, "alerts couldn't be loaded")
        })?;

    let alerts: Vec<Alert> = documents
        .into_iter()
        .filter_map(|document| match document.decode::<Alert>() {
            Ok(mut alert) => {
                alert.id = document.id;
                Some(alert)
            }
            Err(err) => {
                eprintln!("skipping alert {}: {err}", document.id);
                None
            }
        })
        .collect();
    Ok(Json(ListResponse { alerts }))
}
