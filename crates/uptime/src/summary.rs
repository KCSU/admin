//! The `uptime/summary` Firestore document.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use time::Date;

use crate::sites::Site;

pub const COLLECTION: &str = "uptime";
pub const DOCUMENT_ID: &str = "summary";

/// Days of history kept, including today.
pub const HISTORY_DAYS: i64 = 90;

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Summary {
    /// When the last run finished, in Unix seconds.
    pub updated_at: i64,
    pub sites: Vec<SiteStatus>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SiteStatus {
    pub id: String,
    pub name: String,
    pub url: String,
    pub latest: Option<Check>,
    /// Statistics per UTC day, keyed by date.
    pub days: BTreeMap<Date, Day>,
    /// [`CLOSE_WINDOW`](crate::state::CLOSE_WINDOW) bits representing the last checks. Bit 0 is the latest.
    pub recent_failure_bits: u32,
    /// When the current outage opened. `None` when there is no outage.
    pub outage_since: Option<i64>,
    /// When the last down alert was sent.
    pub last_alert_at: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Check {
    /// Unix seconds.
    pub checked_at: i64,
    /// `None` if no response arrived.
    pub status_code: Option<u16>,
    /// `None` if no complete response arrived.
    pub latency_ms: Option<u64>,
    /// `None` means the check passed.
    pub error: Option<String>,
}

impl Check {
    pub fn ok(&self) -> bool {
        self.error.is_none()
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Day {
    pub up: u32,
    pub total: u32,
}

impl Summary {
    /// Matches stored sites to `sites`.
    pub fn with_sites(mut self, sites: &[Site]) -> Self {
        self.sites = sites
            .iter()
            .map(|site| {
                let mut status = self
                    .sites
                    .iter()
                    .find(|status| status.id == site.id)
                    .cloned()
                    .unwrap_or_default();
                status.id = site.id.to_owned();
                status.name = site.name.to_owned();
                status.url = site.url.to_owned();
                status
            })
            .collect();
        self
    }
}
