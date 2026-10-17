//! JSON errors for API requests.

use axum::Json;
use axum::http::StatusCode;
use serde_json::{Value, json};

/// A JSON error response: `{"error": "..."}`.
pub type ErrorResponse = (StatusCode, Json<Value>);

pub fn error(status: StatusCode, message: &str) -> ErrorResponse {
    (status, Json(json!({ "error": message })))
}
