//! Posts alerts into a Google Chat space through an incoming webhook.

use std::time::Duration;

use serde_json::json;

use crate::Alert;

/// Posts alerts to a Google Chat space.
pub struct GoogleChat {
    /// Secret Webhook URL for the Google Chat space.
    webhook: String,
    http_client: reqwest::Client,
}

impl GoogleChat {
    /// Creates a handler that posts to `webhook`, the space's incoming webhook URL.
    ///
    /// Fails if the HTTP client can't be built.
    pub fn new(webhook: impl Into<String>) -> reqwest::Result<Self> {
        Ok(Self {
            webhook: webhook.into(),
            http_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()?,
        })
    }

    /// Posts `alert` to the space, formatted as a card.
    pub async fn handle(&self, alert: &Alert) -> reqwest::Result<()> {
        let body = chat_message(alert);
        self.http_client
            .post(&self.webhook)
            .json(&body)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }
}

const ADMIN_URL: &str = "https://admin.kcsu.org.uk/alerts";
const ICON_URL: &str = "https://www.gstatic.com/images/icons/material/system/2x/error_red_48dp.png";
const SUBTITLE: &str = "Please investigate this issue urgently. This alert is sent automatically \
                        when a serious issue is detected and needs attention.";

/// Format the alert into a Google Chat card.
fn chat_message(alert: &Alert) -> serde_json::Value {
    let title = format!("Service Disruption: {}: {}", alert.source, alert.summary);

    // Two columns of key-value labels
    let mut left = vec![
        key_value_label("Service", &alert.source),
        key_value_label("Deduplication key", &alert.dedup_key),
    ];
    let mut right = vec![key_value_label(
        "Occurred",
        &common::fmt_unix_timestamp(alert.occurred_at_unix),
    )];

    // Alternate between the two columns for each key-value pair.
    let mut extra: Vec<_> = alert.extra.iter().collect();
    extra.sort();
    for (key, value) in extra {
        let column = if right.len() < left.len() {
            &mut right
        } else {
            &mut left
        };
        column.push(key_value_label(key, value));
    }

    let mut body = vec![json!({ "columns": { "columnItems": [
        { "widgets": left },
        { "widgets": right },
    ]}})];

    // If alert has detail, include it in the card body
    if let Some(detail) = &alert.detail {
        body.push(json!({ "textParagraph": { "text": format!("<i>{}</i>", escape(detail)) } }));
    }

    json!({
        "fallbackText": title,
        "cardsV2": [{
            "cardId": "alert",
            "card": {
                "header": {
                    "title": title,
                    "subtitle": SUBTITLE,
                    "imageUrl": ICON_URL,
                    "imageType": "CIRCLE",
                    "imageAltText": "Alert",
                },
                "sections": [
                    { "widgets": body },
                    {
                        "widgets": [{
                            "buttonList": {
                                "buttons": [{
                                    "text": "Open in admin",
                                    "type": "FILLED",
                                    "onClick": {
                                        "openLink": { "url": ADMIN_URL }
                                    },
                                }]
                            }
                        }]
                    },
                ],
            }
        }],
    })
}

/// A labelled value. Card text is HTML, so the value has to be escaped.
fn key_value_label(label: &str, value: &str) -> serde_json::Value {
    json!({ "decoratedText": { "topLabel": label, "text": escape(value) } })
}

/// Escapes `text`.
fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
