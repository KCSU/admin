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

/// Pretty print a unix timestamp.
pub fn fmt_unix_timestamp(unix: i64) -> String {
    let format = time::macros::format_description!(
        "[day padding:none] [month repr:short] [year], [hour]:[minute] UTC"
    );
    time::OffsetDateTime::from_unix_timestamp(unix)
        .ok()
        .and_then(|at| at.format(format).ok())
        .unwrap_or_else(|| "unknown".to_owned())
}
