//! Serve the private admin console API.

mod alerts;
mod auth;

use std::sync::Arc;

use axum::extract::Request;
use axum::http::{HeaderValue, StatusCode, header};
use axum::middleware::Next;
use axum::response::Response;
use axum::routing::{any, get};
use axum::{Extension, Json, Router, middleware};
use serde_json::{Value, json};
use tower_http::services::{ServeDir, ServeFile};

use auth::{Auth, User};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = common::require_env("PORT")?;
    let project = common::require_env("GOOGLE_CLOUD_PROJECT")?;
    let firestore = Arc::new(common::Firestore::new(&project)?);
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;

    let app = Router::new()
        .route("/api/me", get(me))
        .route("/api/alerts", get(alerts::list))
        .route("/api/{*rest}", any(api_not_found))
        .nest_service("/assets", ServeDir::new("web/dist/assets"))
        .with_state(firestore)
        .fallback_service(ServeDir::new("web/dist").fallback(ServeFile::new("web/dist/index.html")))
        .layer(middleware::from_fn(cache_control))
        .layer(middleware::from_fn(auth::same_origin))
        .layer(middleware::from_fn_with_state(
            auth_from_env(),
            auth::require_user,
        ));
    axum::serve(listener, app).await?;
    Ok(())
}

fn auth_from_env() -> Auth {
    #[cfg(debug_assertions)]
    if let Ok(email) = std::env::var("DEV_USER_EMAIL") {
        eprintln!("DEV_USER_EMAIL is set. Everyone is signed in as {email}");
        return Auth::Dev(User { email });
    }
    Auth::Iap
}

async fn api_not_found() -> (StatusCode, Json<Value>) {
    (StatusCode::NOT_FOUND, Json(json!({ "error": "not found" })))
}

async fn me(Extension(user): Extension<User>) -> Json<User> {
    Json(user)
}

async fn cache_control(request: Request, next: Next) -> Response {
    let immutable = request.uri().path().starts_with("/assets/");
    let mut response = next.run(request).await;
    let value = if immutable && response.status().is_success() {
        "private, max-age=31536000, immutable"
    } else {
        "private, no-cache"
    };
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static(value));
    response
}
