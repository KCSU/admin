//! Publicly viewable uptime information of KCSU's sites.

use std::collections::BTreeMap;
use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use time::Date;
use uptime::sites::SITES;
use uptime::summary::{COLLECTION, Check, DOCUMENT_ID, Day, SiteStatus, Summary};

use crate::notices::{self, Notice, Notices};

/// The uptime job writes once a minute.
const CACHE_CONTROL: &str = "public, max-age=30";

/// The uptime API is public, therefore set Access-Control-Allow-Origin: *
const ALLOW_ORIGIN: &str = "*";

/// The status page's view of the summary.
#[derive(Serialize)]
struct Uptime<'a> {
    /// When the uptime job last ran, in Unix seconds.
    updated_at: i64,
    sites: Vec<Site<'a>>,
    /// Newest first.
    notices: Vec<Notice>,
}

#[derive(Serialize)]
struct Site<'a> {
    id: &'a str,
    name: &'a str,
    url: &'a str,
    latest: Option<&'a Check>,
    /// When the current outage started. `None` while up.
    outage_since: Option<i64>,
    /// Up/total checks per UTC day, keyed by "YYYY-MM-DD".
    days: &'a BTreeMap<Date, Day>,
}

impl<'a> From<&'a SiteStatus> for Site<'a> {
    fn from(site: &'a SiteStatus) -> Self {
        Self {
            id: &site.id,
            name: &site.name,
            url: &site.url,
            latest: site.latest.as_ref(),
            outage_since: site.outage_since,
            days: &site.days,
        }
    }
}

pub(crate) async fn get(State(firestore): State<Arc<common::Firestore>>) -> Response {
    let (summary, notices) =
        futures_util::future::join(read(&firestore), notices::read(&firestore)).await;

    let notices = notices.unwrap_or_else(|err| {
        eprintln!("reading notices failed: {}", common::report(&*err));
        Notices::default()
    });

    let summary = match summary {
        Ok(summary) => summary,
        Err(err) => {
            eprintln!(
                "reading the uptime summary failed: {}",
                common::report(&*err)
            );
            return (
                StatusCode::BAD_GATEWAY,
                [(header::ACCESS_CONTROL_ALLOW_ORIGIN, ALLOW_ORIGIN)],
            )
                .into_response();
        }
    };
    let body = Uptime {
        updated_at: summary.updated_at,
        sites: summary.sites.iter().map(Site::from).collect(),
        notices: notices.notices,
    };
    (
        [
            (header::CACHE_CONTROL, CACHE_CONTROL),
            (header::ACCESS_CONTROL_ALLOW_ORIGIN, ALLOW_ORIGIN),
        ],
        Json(body),
    )
        .into_response()
}

/// The stored summary in Firestore, matched to the current sites.
async fn read(
    firestore: &common::Firestore,
) -> Result<Summary, Box<dyn std::error::Error + Send + Sync>> {
    let summary = match firestore.get(COLLECTION, DOCUMENT_ID).await? {
        Some(document) => document.decode()?,
        None => Summary::default(),
    };
    Ok(summary.with_sites(SITES))
}
