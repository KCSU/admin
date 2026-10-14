//! The sites being monitored.

/// A site with a health endpoint.
pub struct Site {
    /// A stable ID used to key its history and its alerts.
    pub id: &'static str,
    pub name: &'static str,
    /// The site's home page.
    pub url: &'static str,
    /// Responds with `{"status": "ok"}` when healthy.
    pub health_path: &'static str,
}

impl Site {
    pub fn health_url(&self) -> String {
        format!(
            "{}/{}",
            self.url.trim_end_matches('/'),
            self.health_path.trim_start_matches('/')
        )
    }
}

/// All monitored sites.
pub const SITES: &[Site] = &[
    Site {
        id: "website",
        name: "KCSU Website",
        url: "https://www.kcsu.org.uk/",
        health_path: "/health",
    },
    Site {
        id: "library",
        name: "KCSU Library of Things",
        url: "https://library.kcsu.org.uk/",
        health_path: "/api/health",
    },
    Site {
        id: "kords",
        name: "KORDs v5 (King's Open Room Database)",
        url: "https://kords.kcsu.org.uk/",
        health_path: "/health",
    },
    Site {
        id: "store",
        name: "KiFoMaSy (KCSU Formal Ticket Store)",
        url: "https://store.kcsu.org.uk/",
        health_path: "/api/health",
    },
];
