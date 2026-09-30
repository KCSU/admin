//! Send alerts to the `alerts` pub/sub topic.

use google_cloud_pubsub::client::Publisher;
use google_cloud_pubsub::model::Message;
use prost::Message as _;

pub use proto::alerts::v1::Alert;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Couldn't create the pub/sub publisher.
    #[error("alert publisher setup failed")]
    Setup(#[source] google_cloud_gax::client_builder::Error),

    /// A required field was empty.
    #[error("alert {0} must not be empty")]
    Empty(&'static str),

    /// An error occurred while publishing the alert.
    #[error("publishing alert failed")]
    Publish(#[source] google_cloud_pubsub::error::PublishError),
}

/// Publishes alerts on behalf of one service.
#[derive(Clone)]
pub struct AlertPublisher {
    publisher: Publisher,
    source: String,
}

impl AlertPublisher {
    pub async fn new(topic: &str, source: impl Into<String>) -> Result<Self, Error> {
        let publisher = Publisher::builder(topic)
            .build()
            .await
            .map_err(Error::Setup)?;
        Ok(Self {
            publisher,
            source: source.into(),
        })
    }

    /// Start an alert.
    pub fn fire(
        &self,
        dedup_key: impl Into<String>,
        summary: impl Into<String>,
    ) -> AlertBuilder<'_> {
        AlertBuilder {
            publisher: self,
            alert: Alert {
                source: self.source.clone(),
                dedup_key: dedup_key.into(),
                occurred_at_unix: common::unix_now(),
                summary: summary.into(),
                ..Default::default()
            },
        }
    }
}

/// Build optional fields for an alert.
#[must_use = "an alert does nothing until `.send().await`"]
pub struct AlertBuilder<'a> {
    publisher: &'a AlertPublisher,
    alert: Alert,
}

impl AlertBuilder<'_> {
    /// Optional longer text, e.g. an error message.
    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.alert.detail = Some(detail.into());
        self
    }

    /// Include extra context in the alert, e.g. "group_id" -> "105777".
    pub fn extra(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.alert.extra.insert(key.into(), value.into());
        self
    }

    /// Publish the alert.
    pub async fn send(self) -> Result<(), Error> {
        let message = message(&self.alert)?;
        self.publisher
            .publisher
            .publish(message)
            .await
            .map_err(Error::Publish)?;
        Ok(())
    }
}

pub fn message(alert: &Alert) -> Result<Message, Error> {
    for (field, value) in [
        ("source", &alert.source),
        ("dedup_key", &alert.dedup_key),
        ("summary", &alert.summary),
    ] {
        if value.trim().is_empty() {
            return Err(Error::Empty(field));
        }
    }
    Ok(Message::new()
        .set_data(alert.encode_to_vec())
        .set_attributes([("source", alert.source.clone())]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alert() -> Alert {
        Alert {
            source: "admin".into(),
            dedup_key: "lookup-sync-failed".into(),
            occurred_at_unix: 1,
            summary: "Lookup sync failed".into(),
            detail: Some("HTTP 503".into()),
            extra: [("group_id".into(), "105777".into())].into(),
        }
    }

    #[test]
    fn message_round_trips_and_sets_source_attribute() {
        let message = message(&alert()).unwrap();
        assert_eq!(
            message.attributes.get("source").map(String::as_str),
            Some("admin")
        );
        assert_eq!(Alert::decode(message.data).unwrap(), alert());
    }

    #[test]
    fn rejects_empty_required_fields() {
        let mut missing_key = alert();
        missing_key.dedup_key = " ".into();
        assert!(matches!(
            message(&missing_key),
            Err(Error::Empty("dedup_key"))
        ));

        let mut missing_summary = alert();
        missing_summary.summary.clear();
        assert!(matches!(
            message(&missing_summary),
            Err(Error::Empty("summary"))
        ));
    }
}
