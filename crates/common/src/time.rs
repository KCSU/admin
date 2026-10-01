use std::time::{SystemTime, UNIX_EPOCH};

/// Current time in whole seconds since the Unix epoch.
pub fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is after 1970")
        .as_secs()
        .try_into()
        .expect("timestamp fits in i64")
}

/// Pretty print a unix timestamp, e.g.: "1 Oct 2026, 14:28 UTC".
pub fn fmt_unix_timestamp(unix: i64) -> String {
    let format = time::macros::format_description!(
        "[day padding:none] [month repr:short] [year], [hour]:[minute] UTC"
    );
    time::OffsetDateTime::from_unix_timestamp(unix)
        .ok()
        .and_then(|at| at.format(format).ok())
        .unwrap_or_else(|| "unknown".to_owned())
}

#[derive(Debug, thiserror::Error)]
#[error("unix timestamp {0} is out of range")]
pub struct TimestampOutOfRange(i64);

/// Format a unix timestamp as RFC 3339, e.g. "2026-10-01T14:28:29Z".
pub fn rfc3339(unix: i64) -> Result<String, TimestampOutOfRange> {
    time::OffsetDateTime::from_unix_timestamp(unix)
        .ok()
        .and_then(|at| {
            at.format(&time::format_description::well_known::Rfc3339)
                .ok()
        })
        .ok_or(TimestampOutOfRange(unix))
}
