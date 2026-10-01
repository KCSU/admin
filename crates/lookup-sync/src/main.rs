use alerts::AlertPublisher;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let alerts_topic = common::require_env("ALERTS_TOPIC")?;
    let alerts = AlertPublisher::new(&alerts_topic, "admin/lookup-sync").await?;

    if let Err(err) = run().await {
        let sent = alerts
            .alert("sync-failed", "Lookup sync failed")
            .detail(common::report(&*err))
            .send()
            .await;

        // If sending the alert itself failed
        if let Err(alert_err) = sent {
            eprintln!("sending alert failed: {}", common::report(&alert_err));
        }
        return Err(err);
    }
    Ok(())
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let client_id = common::require_env("CAMBRIDGE_UIS_API_KEY")?;
    let client_secret = common::require_env("CAMBRIDGE_UIS_API_SECRET")?;
    let topic = common::require_env("PUBSUB_TOPIC")?;
    lookup_sync::sync(&client_id, &client_secret, &topic).await?;
    Ok(())
}
