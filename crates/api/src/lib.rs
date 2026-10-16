//! Axum routers for the public admin API.

mod exec;
mod notices;
mod uptime;

use std::sync::Arc;

use axum::http::StatusCode;
use axum::{Json, Router, routing::get};
use serde_json::{Value, json};

pub fn router(firestore: common::Firestore) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/exec", get(exec::get))
        .route("/uptime", get(uptime::get))
        .fallback(not_found)
        .with_state(Arc::new(firestore))
}

async fn health() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}

async fn not_found() -> (StatusCode, Json<Value>) {
    (StatusCode::NOT_FOUND, Json(json!({ "error": "not found" })))
}
