//! Lookup API client and pub/sub publishing.

mod client;
mod error;
mod response;

pub use error::Error;

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use google_cloud_pubsub::client::Publisher;
use google_cloud_pubsub::model::Message;
use prost::Message as _;
use proto::lookup::v1::GroupSnapshot;

use crate::client::{fetch_access_token, fetch_group, fetch_group_members};

/// List of group IDs to pull from Lookup.
/// todo(khm39): get this dynamically from somewhere instead
const GROUP_IDS: &[&str] = &[
    "105777", // kings-sis-ug
    "105776", // kings-sis-pg
];

/// Fetch each group in `GROUP_IDS` from Lookup and publish it to `topic`.
/// `client_id` / `client_secret` are UIS API Gateway app credentials.
pub async fn sync(client_id: &str, client_secret: &str, topic: &str) -> Result<(), Error> {
    let http_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(Error::HttpClient)?;

    let token = fetch_access_token(&http_client, client_id, client_secret).await?;
    let publisher = Publisher::builder(topic)
        .build()
        .await
        .map_err(Error::PubSubSetup)?;

    for group_id in GROUP_IDS {
        // Fetch from Lookup
        let group = fetch_group(&http_client, &token, group_id).await?;
        let members = fetch_group_members(&http_client, &token, group_id).await?;

        let snapshot = GroupSnapshot {
            id: (*group_id).to_owned(),
            name: group.name,
            description: group.title,
            fetched_at_unix: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock is before 1970")
                .as_secs() as i64,
            members,
        };

        // Publish to pub/sub
        let message = Message::new()
            .set_data(snapshot.encode_to_vec())
            .set_ordering_key(snapshot.id.clone())
            .set_attributes([("group_id", snapshot.id.clone())]);

        publisher
            .publish(message)
            .await
            .map_err(|source| Error::Publish {
                group_id: (*group_id).to_owned(),
                source,
            })?;
    }

    Ok(())
}
