//! Drops repeats of an alert.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::Alert;

pub struct Dedup {
    /// Specifies for how long after the last alert its repeats are dropped.
    window: Duration,
    /// When each `(source, dedup_key)` was last let through.
    last_sent: Mutex<HashMap<(String, String), Instant>>,
}

impl Dedup {
    pub fn new(window: Duration) -> Self {
        Self {
            window,
            last_sent: Mutex::new(HashMap::new()),
        }
    }

    /// Marks `alert` as sent at `now`, unless it's a repeat.
    ///
    /// Returns `true` if no alert with the same `(source, dedup_key)` was marked in
    /// the window before `now`. Otherwise returns `false`, and marks nothing.
    ///
    /// If handling it then fails, call [`unmark_sent`](Self::unmark_sent) with the
    /// same `now`.
    pub fn try_mark_sent(&self, alert: &Alert, now: Instant) -> bool {
        let mut last_sent = self.last_sent.lock().unwrap_or_else(|err| err.into_inner());
        // forget all alerts outside the window
        last_sent.retain(|_, sent| now.duration_since(*sent) < self.window);

        let key = (alert.source.clone(), alert.dedup_key.clone());
        if last_sent.contains_key(&key) {
            return false;
        }
        last_sent.insert(key, now);
        true
    }

    /// Undoes [`try_mark_sent`](Self::try_mark_sent) of `alert` at `marked_at`.
    pub fn unmark_sent(&self, alert: &Alert, marked_at: Instant) {
        let mut last_sent = self.last_sent.lock().unwrap_or_else(|err| err.into_inner());
        let key = (alert.source.clone(), alert.dedup_key.clone());
        if last_sent.get(&key) == Some(&marked_at) {
            last_sent.remove(&key);
        }
    }
}
