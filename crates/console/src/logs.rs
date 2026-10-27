//! The logs of KCSU's Cloud Run services and jobs, from Cloud Logging.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use serde::de::IntoDeserializer;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{ErrorResponse, error};

const PROJECTS: &[Project] = &[
    Project {
        id: "kcsu-admin",
        prefix: "admin",
        shorten: |name| name,
    },
    Project {
        id: "kcsu-library-of-things",
        prefix: "library",
        shorten: |name| {
            name.strip_prefix("library-of-things-")
                .or_else(|| name.strip_prefix("library-"))
                .unwrap_or(name)
        },
    },
    Project {
        id: "kifomasy",
        prefix: "store",
        shorten: |name| name,
    },
];
/// Logs are kept this long.
const MAX_HOURS: i64 = 30 * 24;
const MAX_ENTRIES: usize = 1000;
/// The most `entries.list` returns at once.
const PAGE_SIZE: u32 = 1000;
/// Counts lines by severity and HTTP status.
const METRIC: &str = "logging.googleapis.com/user/log_lines";
/// Time slots per volume, roughly.
const BUCKETS: i64 = 72;
const SEVERITIES: [&str; 9] = [
    "DEFAULT",
    "DEBUG",
    "INFO",
    "NOTICE",
    "WARNING",
    "ERROR",
    "CRITICAL",
    "ALERT",
    "EMERGENCY",
];

const STATUSES: [(&str, Option<u16>); 3] = [
    ("httpRequest.status>=400 httpRequest.status<500", Some(400)),
    ("httpRequest.status>=500", Some(500)),
    ("(-httpRequest.status:* OR httpRequest.status<400)", None),
];

/// Services are shown as `{prefix}/{short name}`, e.g. `store/api`.
struct Project {
    id: &'static str,
    prefix: &'static str,
    /// e.g. `library-of-things-production` to `production`.
    shorten: fn(&str) -> &str,
}

/// `kifomasy/api` to its project and name; `None` for all services.
fn parse_service(id: Option<&str>) -> Result<Option<(&'static str, &str)>, ErrorResponse> {
    let Some(id) = id.filter(|id| !id.is_empty()) else {
        return Ok(None);
    };
    let unknown = || error(StatusCode::BAD_REQUEST, "unknown service");
    let (project, name) = id.split_once('/').ok_or_else(unknown)?;
    let project = PROJECTS
        .iter()
        .find(|p| p.id == project)
        .ok_or_else(unknown)?;
    Ok(Some((project.id, name)))
}

fn service_filter(name: &str) -> String {
    let name = quote(name);
    format!("(resource.labels.service_name={name} OR resource.labels.job_name={name})")
}

#[derive(Deserialize)]
pub struct ListQuery {
    hours: Option<i64>,
    /// Unix seconds.
    from: Option<i64>,
    to: Option<i64>,
    /// e.g. `kifomasy/api`.
    service: Option<String>,
    /// Text the message or URL contains.
    contains: Option<String>,
    /// e.g. `WARNING,ERROR`; all if missing.
    levels: Option<String>,
}

#[derive(Deserialize)]
pub struct VolumeQuery {
    hours: Option<i64>,
    from: Option<i64>,
    to: Option<i64>,
    service: Option<String>,
}

/// The body of `GET /api/logs/volume`.
#[derive(Serialize)]
pub struct VolumeResponse {
    /// Unix seconds.
    start: i64,
    /// Seconds.
    period: i64,
    /// Oldest first.
    counts: Vec<Counts>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "UPPERCASE")]
struct Counts {
    default: u64,
    debug: u64,
    info: u64,
    warning: u64,
    error: u64,
}

impl Counts {
    fn add(&mut self, severity: Severity, lines: u64) {
        *match severity {
            Severity::Default => &mut self.default,
            Severity::Debug => &mut self.debug,
            Severity::Info => &mut self.info,
            Severity::Warning => &mut self.warning,
            Severity::Error => &mut self.error,
        } += lines;
    }
}

/// The body of `GET /api/logs`.
#[derive(Serialize)]
pub struct ListResponse {
    /// Newest first.
    entries: Vec<Entry>,
    /// Whether older entries were left out.
    truncated: bool,
}

#[derive(Debug, PartialEq, Serialize)]
struct Entry {
    id: String,
    /// RFC 3339.
    timestamp: String,
    severity: Severity,
    /// e.g. `library/production`, for display.
    service: String,
    /// e.g. `kcsu-library-of-things/library-of-things-production`, the full name.
    service_id: String,
    revision: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    http: Option<Http>,
    #[serde(skip_serializing_if = "Option::is_none")]
    trace: Option<String>,
}

/// Cloud Logging's nine severities, folded into five.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
enum Severity {
    /// None given, e.g. plain text.
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

/// Log entries in a time window, newest first.
pub async fn list(
    State(logging): State<Arc<common::Logging>>,
    Query(query): Query<ListQuery>,
) -> Result<Json<ListResponse>, ErrorResponse> {
    let (from, to) = time_range(query.hours, query.from, query.to)?;
    let service = parse_service(query.service.as_deref())?;
    let levels = query
        .levels
        .as_deref()
        .filter(|levels| !levels.is_empty())
        .map(|levels| {
            levels
                .split(',')
                .map(|level| Severity::deserialize(level.into_deserializer()))
                .collect::<Result<Vec<_>, serde::de::value::Error>>()
        })
        .transpose()
        .map_err(|_| error(StatusCode::BAD_REQUEST, "unknown level"))?;
    let filter = filter(
        &rfc3339(from)?,
        &rfc3339(to)?,
        service.map(|(_, name)| service_filter(name)).as_deref(),
        query.contains.as_deref(),
        levels.as_deref(),
    );
    let projects: Vec<&str> = match service {
        Some((project, _)) => vec![project],
        None => PROJECTS.iter().map(|project| project.id).collect(),
    };

    let mut entries = Vec::new();
    let mut page_token = None;
    loop {
        let page = logging
            .list(&projects, &filter, PAGE_SIZE, page_token.as_deref())
            .await
            .map_err(|err| {
                log::error!(
                    "listing logs from Google Cloud failed: {}",
                    common::report(&err)
                );
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

/// Lines per time slot in a time window.
pub async fn volume(
    State(monitoring): State<Arc<common::Monitoring>>,
    Query(query): Query<VolumeQuery>,
) -> Result<Json<VolumeResponse>, ErrorResponse> {
    let (from, to) = time_range(query.hours, query.from, query.to)?;
    let minutes = (to - from + BUCKETS * 60 - 1) / (BUCKETS * 60);
    let period = minutes.max(1) * 60;
    let slots = (to - from + period - 1) / period;
    let start = to - slots * period;

    let service = parse_service(query.service.as_deref())?;
    let mut filter = format!("metric.type=\"{METRIC}\"");
    if let Some((_, name)) = service {
        filter.push_str(&format!(" AND {}", service_filter(name)));
    }
    let (since, until) = (rfc3339(start)?, rfc3339(to)?);
    let mut counts = vec![Counts::default(); slots as usize];
    let projects = PROJECTS
        .iter()
        .map(|project| project.id)
        .filter(|&project| service.is_none_or(|(only, _)| only == project));
    for project in projects {
        let series = monitoring
            .sums(
                project,
                &filter,
                &since,
                &until,
                period,
                &["metric.label.severity", "metric.label.status"],
            )
            .await
            .map_err(|err| {
                log::error!(
                    "counting logs in {project} failed: {}",
                    common::report(&err)
                );
                error(
                    StatusCode::BAD_GATEWAY,
                    "Couldn't reach Cloud Monitoring. Please try again.",
                )
            })?;
        for series in &series {
            let label = |name: &str| series.pointer(&format!("/metric/labels/{name}"))?.as_str();
            let status = label("status").and_then(|status| status.parse().ok());
            let severity = severity(label("severity").unwrap_or("DEFAULT"), status);
            for point in series["points"].as_array().into_iter().flatten() {
                let end = point
                    .pointer("/interval/endTime")
                    .and_then(Value::as_str)
                    .and_then(common::parse_rfc3339);
                let lines = point
                    .pointer("/value/int64Value")
                    .and_then(Value::as_str)
                    .and_then(|lines| lines.parse().ok());
                if let (Some(end), Some(lines)) = (end, lines)
                    && let Some(slot) = counts.get_mut(((end - start) / period - 1) as usize)
                {
                    slot.add(severity, lines);
                }
            }
        }
    }
    Ok(Json(VolumeResponse {
        start,
        period,
        counts,
    }))
}

/// In Unix seconds, within the last `MAX_HOURS`.
fn time_range(
    hours: Option<i64>,
    from: Option<i64>,
    to: Option<i64>,
) -> Result<(i64, i64), ErrorResponse> {
    let now = common::unix_now();
    let (from, to) = match (from, to) {
        (Some(from), to) => (
            from.max(now - MAX_HOURS * 60 * 60),
            to.unwrap_or(now).min(now),
        ),
        (None, None) => (now - hours.unwrap_or(24).clamp(1, MAX_HOURS) * 60 * 60, now),
        (None, Some(_)) => return Err(error(StatusCode::BAD_REQUEST, "`to` needs a `from`")),
    };
    if from >= to {
        return Err(error(StatusCode::BAD_REQUEST, "the time range is empty"));
    }
    Ok((from, to))
}

fn rfc3339(unix: i64) -> Result<String, ErrorResponse> {
    common::rfc3339(unix).map_err(|_| error(StatusCode::BAD_REQUEST, "that time is out of range"))
}

/// Our services' and jobs' logs, without audit logs.
fn filter(
    since: &str,
    until: &str,
    service_filter: Option<&str>,
    contains: Option<&str>,
    levels: Option<&[Severity]>,
) -> String {
    let mut filter = format!(
        "resource.type=(\"cloud_run_revision\" OR \"cloud_run_job\") \
         -logName:\"cloudaudit.googleapis.com\" timestamp>=\"{since}\" timestamp<=\"{until}\""
    );
    if let Some(service_filter) = service_filter {
        filter.push_str(&format!(" {service_filter}"));
    }
    if let Some(text) = contains.map(str::trim).filter(|text| !text.is_empty()) {
        let text = quote(text);
        filter.push_str(&format!(
            " (textPayload:{text} OR jsonPayload.message:{text} OR httpRequest.requestUrl:{text})"
        ));
    }
    if let Some(levels) = levels {
        let clauses: Vec<String> = levels.iter().map(|&level| level_filter(level)).collect();
        filter.push_str(&format!(" ({})", clauses.join(" OR ")));
    }
    filter
}

fn level_filter(level: Severity) -> String {
    let clauses: Vec<String> = STATUSES
        .iter()
        .filter_map(|&(status_filter, status)| {
            let severities: Vec<&str> = SEVERITIES
                .into_iter()
                .filter(|&raw| severity(raw, status) == level)
                .collect();
            match severities.len() {
                0 => None,
                n if n == SEVERITIES.len() => Some(format!("({status_filter})")),
                _ => Some(format!(
                    "({status_filter} severity=({}))",
                    severities.join(" OR ")
                )),
            }
        })
        .collect();
    clauses.join(" OR ")
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
    let project_id = labels("project_id").unwrap_or_default();
    let shown = PROJECTS
        .iter()
        .find(|project| project.id == project_id)
        .map_or_else(
            || service.to_owned(),
            |project| format!("{}/{}", project.prefix, (project.shorten)(service)),
        );

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
        severity: severity(
            str_at("/severity").unwrap_or("DEFAULT"),
            http.as_ref().map(|http| http.status),
        ),
        service: shown,
        service_id: format!("{project_id}/{service}"),
        revision: labels("revision_name").unwrap_or(service).to_owned(),
        message,
        trace: str_at("/trace")
            .and_then(|trace| trace.rsplit('/').next())
            .map(str::to_owned),
        http,
    })
}

fn severity(severity: &str, status: Option<u16>) -> Severity {
    match (severity, status) {
        (_, Some(400..=499)) => Severity::Info,
        (_, Some(500..)) => Severity::Error,
        ("DEBUG", _) => Severity::Debug,
        ("INFO" | "NOTICE", _) => Severity::Info,
        ("WARNING", _) => Severity::Warning,
        ("ERROR" | "CRITICAL" | "ALERT" | "EMERGENCY", _) => Severity::Error,
        _ => Severity::Default,
    }
}
