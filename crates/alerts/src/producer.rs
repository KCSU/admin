use google_cloud_pubsub::client::Publisher;
use google_cloud_pubsub::model::Message;
use prost::Message as _;

use crate::{Alert, SendError, empty_field};

/// Publishes alerts to the alerts pub/sub topic.
///
/// # Examples
///
/// ```no_run
/// use alerts::AlertPublisher;
///
/// # async fn run() -> Result<(), alerts::SendError> {
/// let alerts = AlertPublisher::new("projects/kcsu-admin/topics/alerts", "admin/lookup-sync").await?;
///
/// alerts
///     .alert("sync-failed", "Lookup sync failed")
///     .detail("lookup request for group 105777 failed: timed out")
///     .extra("group_id", "105777")
///     .send()
///     .await?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct AlertPublisher {
    publisher: Publisher,
    source: String,
}

impl AlertPublisher {
    /// Creates a publisher that sends alerts to `topic` as `source`.
    ///
    /// `topic` is the full pub/sub topic name, e.g. `"projects/kcsu-admin/topics/alerts"`.
    /// `source` names the service raising the alert, e.g. `"admin/lookup-sync"`.
    pub async fn new(topic: &str, source: impl Into<String>) -> Result<Self, SendError> {
        let source = source.into();
        if source.trim().is_empty() {
            return Err(SendError::Empty("source"));
        }
        let publisher = Publisher::builder(topic)
            .build()
            .await
            .map_err(SendError::Setup)?;
        Ok(Self { publisher, source })
    }

    /// Start an alert. To fire the alert, see [`AlertBuilder::send`].
    pub fn alert(
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

/// Builder for [`AlertPublisher`]. See [`AlertPublisher`] for more details and usage.
#[must_use = "an alert does nothing until `.send().await`"]
pub struct AlertBuilder<'a> {
    publisher: &'a AlertPublisher,
    alert: Alert,
}

impl AlertBuilder<'_> {
    /// Include optional longer text in the alert.
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
    pub async fn send(self) -> Result<(), SendError> {
        let message = to_message(&self.alert)?;
        self.publisher
            .publisher
            .publish(message)
            .await
            .map_err(SendError::Publish)?;
        Ok(())
    }
}

/// Encode an [`Alert`] into a pub/sub [`Message`].
///
/// Fails with [`SendError::Empty`] if a required field is blank.
pub fn to_message(alert: &Alert) -> Result<Message, SendError> {
    if let Some(field) = empty_field(alert) {
        return Err(SendError::Empty(field));
    }
    Ok(Message::new()
        .set_data(alert.encode_to_vec())
        .set_attributes([("source", alert.source.clone())]))
}
