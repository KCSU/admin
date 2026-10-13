//! Axum routers for the admin API.

mod uptime;

use std::sync::Arc;

use axum::{Json, Router, routing::get};
use serde_json::{Value, json};

pub fn router(firestore: common::Firestore) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/uptime", get(uptime::get))
        .with_state(Arc::new(firestore))
}

async fn health() -> Json<Value> {
    Json(json!({ "health": "ok" }))
}
