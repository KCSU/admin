//! Auth middleware.

use axum::extract::{Request, State};
use axum::http::{HeaderMap, Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

use crate::error::error;

// IAP adds this header to every request.
const HEADER: &str = "x-goog-authenticated-user-email";

/// IAP's prefix on [`HEADER`] before the email address.
const PREFIX: &str = "accounts.google.com:";

/// The only Google Workspace domain allowed in.
const DOMAIN: &str = "@kcsu.org.uk";

/// The signed-in user.
#[derive(Clone, Debug, Serialize)]
pub struct User {
    pub email: String,
}

/// How requests are signed in.
#[derive(Clone)]
pub enum Auth {
    Iap,
    /// Local development only. Everyone is this user.
    #[cfg(debug_assertions)]
    Dev(User),
}

impl Auth {
    fn user(&self, headers: &HeaderMap) -> Option<User> {
        match self {
            Auth::Iap => {
                let email = headers.get(HEADER)?.to_str().ok()?.strip_prefix(PREFIX)?;
                // IAP only lets in accounts granted access, but check anyway in case an
                // IAM grant is ever broader than KCSU's Workspace.
                email.ends_with(DOMAIN).then(|| User {
                    email: email.to_owned(),
                })
            }
            #[cfg(debug_assertions)]
            Auth::Dev(user) => Some(user.clone()),
        }
    }
}

/// Refuses requests that aren't signed in, and gives the rest a [`User`] extension.
pub async fn require_user(State(auth): State<Auth>, mut request: Request, next: Next) -> Response {
    match auth.user(request.headers()) {
        Some(user) => {
            request.extensions_mut().insert(user);
            next.run(request).await
        }
        None => {
            log::warn!(
                "refused {} {}: {HEADER} is {:?}",
                request.method(),
                request.uri().path(),
                request.headers().get(HEADER)
            );
            error(StatusCode::UNAUTHORIZED, "not signed in").into_response()
        }
    }
}

/// Refuse writes sent from other websites (CSRF).
pub async fn same_origin(request: Request, next: Next) -> Response {
    let safe = matches!(
        *request.method(),
        Method::GET | Method::HEAD | Method::OPTIONS
    );
    let same_origin = request
        .headers()
        .get("sec-fetch-site")
        .is_some_and(|value| value == "same-origin");
    if safe || same_origin {
        next.run(request).await
    } else {
        error(StatusCode::FORBIDDEN, "cross-site requests are not allowed").into_response()
    }
}
