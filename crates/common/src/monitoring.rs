//! A minimal client for reading Cloud Monitoring time series.

use std::time::Duration;

use serde::Deserialize;
use serde_json::Value;

use crate::google::{GoogleClient, SendError, SetupError};

#[derive(Debug, thiserror::Error)]
pub enum MonitoringError {
    #[error("HTTP client setup failed")]
    HttpClient(#[source] reqwest::Error),

    #[error("Google credentials setup failed")]
    CredentialsSetup(#[source] google_cloud_auth::build_errors::Error),

    #[error("getting a Google access token failed")]
    AccessToken(#[source] google_cloud_auth::errors::CredentialsError),

    #[error("Cloud Monitoring request failed")]
    Request(#[source] reqwest::Error),

    #[error("Cloud Monitoring returned {status}: {body}")]
    Status {
        status: reqwest::StatusCode,
        body: String,
    },
}

impl From<SetupError> for MonitoringError {
    fn from(err: SetupError) -> Self {
        match err {
            SetupError::HttpClient(err) => Self::HttpClient(err),
            SetupError::Credentials(err) => Self::CredentialsSetup(err),
        }
    }
}

impl From<SendError> for MonitoringError {
    fn from(err: SendError) -> Self {
        match err {
            SendError::AccessToken(err) => Self::AccessToken(err),
            SendError::Request(err) => Self::Request(err),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SeriesPage {
    #[serde(default)]
    time_series: Vec<Value>,
    next_page_token: Option<String>,
}

/// Reads metrics.
pub struct Monitoring {
    client: GoogleClient,
}

impl Monitoring {
    pub fn new() -> Result<Self, MonitoringError> {
        Ok(Self {
            client: GoogleClient::new(Duration::from_secs(30))?,
        })
    }

    pub async fn sums(
        &self,
        project: &str,
        filter: &str,
        start: &str,
        end: &str,
        period_secs: i64,
        group_by: &[&str],
    ) -> Result<Vec<Value>, MonitoringError> {
        let mut series = Vec::new();
        let mut page_token: Option<String> = None;
        loop {
            let period = format!("{period_secs}s");
            let mut query = vec![
                ("filter", filter),
                ("interval.startTime", start),
                ("interval.endTime", end),
                ("aggregation.alignmentPeriod", period.as_str()),
                ("aggregation.perSeriesAligner", "ALIGN_SUM"),
                ("aggregation.crossSeriesReducer", "REDUCE_SUM"),
            ];
            query.extend(
                group_by
                    .iter()
                    .map(|field| ("aggregation.groupByFields", *field)),
            );
            if let Some(token) = &page_token {
                query.push(("pageToken", token));
            }
            let request = self
                .client
                .http
                .get(format!(
                    "https://monitoring.googleapis.com/v3/projects/{project}/timeSeries"
                ))
                .query(&query);
            let response = self.client.send(request).await?;
            let status = response.status();
            if !status.is_success() {
                let body = response.text().await.unwrap_or_default();
                return Err(MonitoringError::Status { status, body });
            }
            let page: SeriesPage = response.json().await.map_err(MonitoringError::Request)?;
            series.extend(page.time_series);
            page_token = page.next_page_token.filter(|token| !token.is_empty());
            if page_token.is_none() {
                return Ok(series);
            }
        }
    }
}
