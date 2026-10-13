//! Checks a site's health endpoint.

use std::time::{Duration, Instant};

use crate::sites::Site;
use crate::summary::Check;

/// How long one request may take.
pub const TIMEOUT: Duration = Duration::from_secs(5);
const ATTEMPTS: u32 = 3;
const RETRY_DELAY: Duration = Duration::from_secs(1);

/// Checks `site`, retrying a failure up to [`ATTEMPTS`] times.
pub async fn check(http_client: &reqwest::Client, site: &Site) -> Check {
    let url = site.health_url();
    let mut result = check_once(http_client, &url).await;
    for _ in 1..ATTEMPTS {
        if result.ok() {
            break;
        }
        tokio::time::sleep(RETRY_DELAY).await;
        result = check_once(http_client, &url).await;
    }
    result
}

async fn check_once(http_client: &reqwest::Client, url: &str) -> Check {
    let checked_at = common::unix_now();
    let start = Instant::now();
    let failed = |status_code: Option<u16>, error: &reqwest::Error| Check {
        checked_at,
        status_code,
        latency_ms: None,
        error: Some(describe(error)),
    };

    let response = match http_client.get(url).send().await {
        Ok(response) => response,
        Err(err) => return failed(None, &err),
    };
    let status_code = response.status().as_u16();
    let body = match response.text().await {
        Ok(body) => body,
        Err(err) => return failed(Some(status_code), &err),
    };
    Check {
        checked_at,
        status_code: Some(status_code),
        latency_ms: Some(start.elapsed().as_millis().try_into().unwrap_or(u64::MAX)),
        error: evaluate(status_code, &body),
    }
}

/// `None` if the response is healthy, else a short reason.
pub fn evaluate(status_code: u16, body: &str) -> Option<String> {
    if status_code != 200 {
        return Some(format!("HTTP {status_code}"));
    }
    let Ok(body) = serde_json::from_str::<serde_json::Value>(body) else {
        return Some("invalid health response".to_owned());
    };
    match body.get("status") {
        Some(status) if status == "ok" => None,
        Some(status) => Some(format!("status: {status}")),
        None => Some("invalid health response".to_owned()),
    }
}

/// A short reason for a failed request, e.g. "timed out".
fn describe(error: &reqwest::Error) -> String {
    if error.is_timeout() {
        "timed out"
    } else if error.is_connect() {
        "connection failed"
    } else {
        "request failed"
    }
    .to_owned()
}
