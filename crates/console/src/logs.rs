//! The logs of KCSU's Cloud Run services and jobs, from Cloud Logging.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{ErrorResponse, error};

/// The projects whose logs are shown.
const PROJECTS: &[&str] = &["kcsu-admin", "kcsu-library-of-things", "kifomasy"];
/// Logs are kept this long.
const MAX_HOURS: i64 = 30 * 24;
/// At most this many entries per response.
const MAX_ENTRIES: usize = 1000;
/// The most `entries.list` returns at once.
const PAGE_SIZE: u32 = 1000;

#[derive(Deserialize)]
pub struct ListQuery {
    hours: Option<i64>,
    /// A Cloud Run service or job.
    service: Option<String>,
    /// Text the message or URL contains.
    contains: Option<String>,
}

/// The body of `GET /api/logs`.
#[derive(Serialize)]
pub struct ListResponse {
    /// Newest first.
    entries: Vec<Entry>,
    /// Whether older entries in the range were left out.
    truncated: bool,
}

#[derive(Debug, PartialEq, Serialize)]
struct Entry {
    id: String,
    /// RFC 3339.
    timestamp: String,
    severity: Severity,
    service: String,
    revision: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    http: Option<Http>,
    #[serde(skip_serializing_if = "Option::is_none")]
    trace: Option<String>,
}

/// Cloud Logging's nine severities, folded into the five the page shows.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "UPPERCASE")]
enum Severity {
    /// None given, e.g. a plain-text line.
    Default,
    Debug,
    Info,
    Warning,
    Error,
}

#[derive(Debug, PartialEq, Serialize)]
struct Http {
    method: String,
    host: String,
    path: String,
    status: u16,
    latency_ms: u64,
}

/// Lists the last `hours` of log entries, newest first, optionally narrowed to one service
/// and to lines containing some text.
pub async fn list(
    State(logging): State<Arc<common::Logging>>,
    Query(query): Query<ListQuery>,
) -> Result<Json<ListResponse>, ErrorResponse> {
    let hours = query.hours.unwrap_or(24).clamp(1, MAX_HOURS);
    let since = common::rfc3339(common::unix_now() - hours * 60 * 60).map_err(|_| {
        error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "the clock is out of range",
        )
    })?;
    let service = query.service.filter(|s| !s.is_empty());
    let filter = filter(&since, service.as_deref(), query.contains.as_deref());

    let mut entries = Vec::new();
    let mut page_token = None;
    loop {
        let page = logging
            .list(PROJECTS, &filter, PAGE_SIZE, page_token.as_deref())
            .await
            .map_err(|err| {
                eprintln!("listing logs failed: {}", common::report(&err));
                match err {
                    common::LoggingError::Status {
                        status: StatusCode::TOO_MANY_REQUESTS,
                        ..
                    } => error(
                        StatusCode::SERVICE_UNAVAILABLE,
                        "Too many log searches in the last minute. Please try again shortly.",
                    ),
                    _ => error(
                        StatusCode::BAD_GATEWAY,
                        "Couldn't reach Cloud Logging. Please try again.",
                    ),
                }
            })?;
        entries.extend(page.entries.iter().filter_map(entry));
        page_token = page.next_page_token.filter(|token| !token.is_empty());
        if page_token.is_none() || entries.len() >= MAX_ENTRIES {
            break;
        }
    }
    let truncated = page_token.is_some() || entries.len() > MAX_ENTRIES;
    entries.truncate(MAX_ENTRIES);
    Ok(Json(ListResponse { entries, truncated }))
}

/// Our services' and jobs' logs, without audit logs.
fn filter(since: &str, service: Option<&str>, contains: Option<&str>) -> String {
    let mut filter = format!(
        "resource.type=(\"cloud_run_revision\" OR \"cloud_run_job\") \
         -logName:\"cloudaudit.googleapis.com\" timestamp>=\"{since}\""
    );
    if let Some(service) = service {
        let service = quote(service);
        filter.push_str(&format!(
            " (resource.labels.service_name={service} OR resource.labels.job_name={service})"
        ));
    }
    if let Some(text) = contains.map(str::trim).filter(|text| !text.is_empty()) {
        let text = quote(text);
        filter.push_str(&format!(
            " (textPayload:{text} OR jsonPayload.message:{text} OR httpRequest.requestUrl:{text})"
        ));
    }
    filter
}

/// `text` as a filter string literal.
fn quote(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}

/// `None` if the entry has no message, e.g. a system event.
fn entry(value: &Value) -> Option<Entry> {
    let str_at = |pointer: &str| value.pointer(pointer).and_then(Value::as_str);
    let labels = |name: &str| str_at(&format!("/resource/labels/{name}"));
    let service = labels("service_name").or_else(|| labels("job_name"))?;

    let http = value.get("httpRequest").map(|request| {
        let url = request["requestUrl"].as_str().unwrap_or_default();
        let without_scheme = url.split_once("://").map_or(url, |(_, rest)| rest);
        let (host, path) = without_scheme
            .split_once('/')
            .map_or((without_scheme, "/".to_owned()), |(host, path)| {
                (host, format!("/{path}"))
            });
        Http {
            method: request["requestMethod"]
                .as_str()
                .unwrap_or_default()
                .to_owned(),
            host: host.to_owned(),
            path,
            status: request["status"].as_u64().unwrap_or_default() as u16,
            latency_ms: request["latency"]
                .as_str()
                .and_then(|latency| latency.trim_end_matches('s').parse::<f64>().ok())
                .map_or(0, |seconds| (seconds * 1000.0).round() as u64),
        }
    });
    let message = match (&http, value.get("textPayload"), value.get("jsonPayload")) {
        (Some(http), _, _) => format!("{} {} {}", http.method, http.path, http.status),
        (None, Some(Value::String(text)), _) => text.trim_end().to_owned(),
        (None, _, Some(json)) => json
            .get("message")
            .and_then(Value::as_str)
            .map_or_else(|| json.to_string(), str::to_owned),
        _ => return None,
    };

    Some(Entry {
        id: str_at("/insertId").unwrap_or_default().to_owned(),
        timestamp: str_at("/timestamp")?.to_owned(),
        severity: severity(str_at("/severity").unwrap_or("DEFAULT"), http.as_ref()),
        service: service.to_owned(),
        revision: labels("revision_name").unwrap_or(service).to_owned(),
        message,
        trace: str_at("/trace")
            .and_then(|trace| trace.rsplit('/').next())
            .map(str::to_owned),
        http,
    })
}

fn severity(severity: &str, http: Option<&Http>) -> Severity {
    match (severity, http.map(|http| http.status)) {
        (_, Some(404)) => Severity::Info,
        (_, Some(500..)) => Severity::Error,
        ("DEBUG", _) => Severity::Debug,
        ("INFO" | "NOTICE", _) => Severity::Info,
        ("WARNING", _) => Severity::Warning,
        ("ERROR" | "CRITICAL" | "ALERT" | "EMERGENCY", _) => Severity::Error,
        _ => Severity::Default,
    }
}
