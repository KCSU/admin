//! Monitors the uptime of KCSU's sites, for the status page.

mod error;
pub mod probe;
pub mod sites;
pub mod state;
pub mod summary;

pub use error::Error;

use alerts::AlertPublisher;
use common::{FirestoreError, Precondition};
use futures_util::future::join_all;

use crate::sites::SITES;
use crate::summary::{COLLECTION, DOCUMENT_ID, SiteStatus, Summary};

/// Checks every site once, records the results, and sends any alerts due.
pub async fn run(firestore: &common::Firestore, alerts: &AlertPublisher) -> Result<(), Error> {
    let http_client = reqwest::Client::builder()
        .timeout(probe::TIMEOUT)
        .build()
        .map_err(Error::HttpClient)?;

    let document = firestore
        .get(COLLECTION, DOCUMENT_ID)
        .await
        .map_err(Error::Read)?;
    let (summary, precondition) = match &document {
        Some(document) => (
            Summary::from_fields(&document.fields).map_err(Error::Decode)?,
            Precondition::UpdatedAt(&document.update_time),
        ),
        None => (Summary::default(), Precondition::Missing),
    };

    // Update the summary, for when the websites changed since the last check.
    let mut summary = summary.with_sites(SITES);

    // Send probing checks concurrently.
    let checks = join_all(SITES.iter().map(|site| probe::check(&http_client, site))).await;
    for (site, check) in summary.sites.iter_mut().zip(checks) {
        site.record(check);
    }

    // Send an alert to the alerting service.
    let now = common::unix_now();
    for site in summary.sites.iter_mut().filter(|site| site.alert_due(now)) {
        // Only mark as alerted once sent successfully.
        match send_alert(alerts, site).await {
            Ok(()) => site.record_alert(now),
            Err(err) => eprintln!(
                "sending the down alert for {} failed: {}",
                site.id,
                common::report(&err)
            ),
        }
    }

    summary.updated_at = now;
    let fields = summary.to_fields().map_err(Error::Encode)?;
    match firestore
        .set_if(COLLECTION, DOCUMENT_ID, fields, precondition)
        .await
    {
        Ok(()) => Ok(()),
        // Uptime probe runs somehow overlapped, and the other one saved first.
        Err(FirestoreError::Conflict) => {
            eprintln!(
                "the summary changed unexpectedly during this run; discarding this run's results"
            );
            Ok(())
        }
        Err(err) => Err(Error::Write(err)),
    }
}

async fn send_alert(alerts: &AlertPublisher, site: &SiteStatus) -> Result<(), alerts::SendError> {
    let mut alert = alerts
        .alert(
            format!("down:{}", site.id),
            format!("{} is down", site.name),
        )
        .extra("url", &site.url)
        .extra(
            "failed_checks",
            format!(
                "{} of the last {}",
                site.failures(state::OPEN_WINDOW),
                state::OPEN_WINDOW
            ),
        );
    if let Some(outage_since) = site.outage_since {
        alert = alert.extra("outage_since", common::fmt_unix_timestamp(outage_since));
    }
    if let Some(error) = site.latest.as_ref().and_then(|check| check.error.as_ref()) {
        alert = alert.detail(error);
    }
    alert.send().await
}
