//! Controls when to send a downtime alert.

use common::utc_date;
use time::Duration;

use crate::summary::{Check, HISTORY_DAYS, SiteStatus};

const MINUTE: i64 = 60;

/// Open an outage once [`OPEN_FAILURES`] of the last [`OPEN_WINDOW`] checks failed.
pub const OPEN_FAILURES: u32 = 3;
pub const OPEN_WINDOW: u32 = 10;

/// Close an outage once none of the last [`CLOSE_WINDOW`] checks failed.
pub const CLOSE_WINDOW: u32 = 10;

// A longer OPEN_WINDOW would reopen an outage straight after it closes.
const _: () = assert!(OPEN_WINDOW <= CLOSE_WINDOW && CLOSE_WINDOW < u32::BITS);

/// Repeat an alert for an ongoing outage this often.
pub const ALERT_INTERVAL: i64 = 60 * MINUTE; // 1 hour

impl SiteStatus {
    /// Record `check` as the latest result.
    pub fn record(&mut self, check: Check) {
        let now = check.checked_at;
        if let Ok(date) = utc_date(now) {
            let day = self.days.entry(date).or_default();
            day.total += 1;
            if check.ok() {
                day.up += 1;
            }
            let oldest = date.saturating_sub(Duration::days(HISTORY_DAYS - 1));
            self.days.retain(|date, _| *date >= oldest);
        }
        self.recent_failure_bits =
            ((self.recent_failure_bits << 1) | u32::from(!check.ok())) & last(CLOSE_WINDOW);
        self.latest = Some(check);

        match self.outage_since {
            None if self.failures(OPEN_WINDOW) >= OPEN_FAILURES => self.outage_since = Some(now),
            Some(_) if self.failures(CLOSE_WINDOW) == 0 => self.outage_since = None,
            _ => {}
        }
    }

    /// How many of the last `window` checks failed.
    pub fn failures(&self, window: u32) -> u32 {
        (self.recent_failure_bits & last(window)).count_ones()
    }

    /// Whether a down alert should be sent at `now`.
    /// Alert once when an outage opens, then every [`ALERT_INTERVAL`] while outage continues.
    pub fn alert_due(&self, now: i64) -> bool {
        let Some(outage_since) = self.outage_since else {
            return false;
        };
        self.last_alert_at
            .is_none_or(|alerted| alerted < outage_since || now - alerted >= ALERT_INTERVAL)
    }

    /// Records that a down alert was sent at `now`.
    pub fn record_alert(&mut self, now: i64) {
        self.last_alert_at = Some(now);
    }
}

/// A mask of the lowest `window` bits.
fn last(window: u32) -> u32 {
    (1 << window) - 1
}

#[cfg(test)]
mod tests {
    use time::macros::date;

    use super::*;

    const DAY: i64 = 24 * 60 * MINUTE;

    /// 2026-10-12 00:00:00 UTC.
    const T0: i64 = 1_791_763_200;

    fn check(at: i64, ok: bool) -> Check {
        Check {
            checked_at: at,
            status_code: Some(if ok { 200 } else { 503 }),
            latency_ms: Some(10),
            error: (!ok).then(|| "HTTP 503".to_owned()),
        }
    }

    #[test]
    fn keeps_ninety_days() {
        let mut site = SiteStatus::default();
        site.record(check(T0 - 90 * DAY, true));
        site.record(check(T0 - 89 * DAY, true));
        site.record(check(T0, true));
        assert_eq!(site.days.len(), 2);
        assert!(site.days.contains_key(&date!(2026 - 07 - 15)));
    }

    /// Records `n` checks, once a minute from `from`.
    /// Returns the time a minute after the last check.
    fn checks(site: &mut SiteStatus, from: i64, n: u32, ok: bool) -> i64 {
        let mut at = from;
        for _ in 0..n {
            site.record(check(at, ok));
            at += MINUTE;
        }
        at
    }

    #[test]
    fn opens_once_enough_of_the_window_failed() {
        let mut site = SiteStatus::default();
        let at = checks(&mut site, T0, OPEN_FAILURES - 1, false);
        let at = checks(&mut site, at, OPEN_WINDOW - OPEN_FAILURES, true);
        assert_eq!(site.outage_since, None);
        checks(&mut site, at, 1, false);
        assert_eq!(site.outage_since, Some(at));
        assert!(site.alert_due(at));
    }

    #[test]
    fn closes_once_the_window_passes() {
        let mut site = SiteStatus::default();
        let at = checks(&mut site, T0, OPEN_FAILURES, false);
        let at = checks(&mut site, at, CLOSE_WINDOW - 1, true);
        assert!(site.outage_since.is_some());
        checks(&mut site, at, 1, true);
        assert_eq!(site.outage_since, None);
    }

    #[test]
    fn flapping_is_one_outage() {
        let mut site = SiteStatus::default();
        let mut at = checks(&mut site, T0, OPEN_FAILURES, false);
        site.record_alert(at);
        let alerted = at;
        while at <= alerted + ALERT_INTERVAL {
            at = checks(&mut site, at, CLOSE_WINDOW - 1, true);
            at = checks(&mut site, at, 1, false);
        }
        assert_eq!(
            site.outage_since,
            Some(T0 + i64::from(OPEN_FAILURES - 1) * MINUTE)
        );
        assert!(!site.alert_due(alerted + ALERT_INTERVAL - 1));
    }

    #[test]
    fn repeats_while_down() {
        let mut site = SiteStatus::default();
        let at = checks(&mut site, T0, OPEN_FAILURES, false);
        site.record_alert(at);
        assert!(!site.alert_due(at + ALERT_INTERVAL - 1));
        assert!(site.alert_due(at + ALERT_INTERVAL));
    }

    #[test]
    fn a_new_outage_alerts_again() {
        let mut site = SiteStatus::default();
        let at = checks(&mut site, T0, OPEN_FAILURES, false);
        site.record_alert(at);
        let at = checks(&mut site, at, CLOSE_WINDOW, true);
        let at = checks(&mut site, at, OPEN_FAILURES, false);
        assert!(site.alert_due(at));
    }
}
