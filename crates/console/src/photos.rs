//! Uploading exec photos to the public bucket.

use std::sync::Arc;

use api::exec::photo::{BUCKET, FOLDER};
use axum::body::Bytes;
use axum::extract::State;
use axum::http::StatusCode;
use axum::{Extension, Json};
use serde::Serialize;

use crate::auth::User;
use crate::error::{ErrorResponse, error};

/// Photos are resized in the browser first, so this is probably fine.
pub const MAX_BYTES: usize = 2 * 1024 * 1024;

/// Every name is new, so the photo at a URL never changes.
const CACHE_CONTROL: &str = "public, max-age=31536000, immutable";

/// The response to `POST /api/exec/photos`.
#[derive(Serialize)]
pub struct Uploaded {
    /// The photo's public URL.
    url: String,
}

/// `POST /api/exec/photos`: stores the image in the body, and returns its public URL.
pub async fn upload(
    State(storage): State<Arc<common::Storage>>,
    Extension(user): Extension<User>,
    body: Bytes,
) -> Result<Json<Uploaded>, ErrorResponse> {
    let (content_type, extension) = sniff(&body).ok_or_else(|| {
        error(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "Photos must be JPEG, PNG or WebP.",
        )
    })?;
    // Random, so names don't collide in practice.
    let name = format!("{:016x}.{extension}", rand::random::<u64>());
    let object = format!("{FOLDER}{name}");
    storage
        .create(BUCKET, &object, content_type, CACHE_CONTROL, body.to_vec())
        .await
        .map_err(|err| {
            eprintln!("uploading {object} failed: {}", common::report(&err));
            error(
                StatusCode::BAD_GATEWAY,
                "The photo couldn't be uploaded. Please try again.",
            )
        })?;
    eprintln!("{} uploaded {object}", user.email);
    Ok(Json(Uploaded {
        url: api::exec::photo::url(&name),
    }))
}

/// The image type of `data` from its first bytes, as a MIME type and file extension.
fn sniff(data: &[u8]) -> Option<(&'static str, &'static str)> {
    if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some(("image/jpeg", "jpg"))
    } else if data.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some(("image/png", "png"))
    } else if data.len() >= 12 && &data[..4] == b"RIFF" && &data[8..12] == b"WEBP" {
        Some(("image/webp", "webp"))
    } else {
        None
    }
}
