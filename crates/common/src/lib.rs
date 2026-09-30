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
