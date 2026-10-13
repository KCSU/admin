use alerts::AlertPublisher;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let alerts_topic = common::require_env("ALERTS_TOPIC")?;
    let alerts = AlertPublisher::new(&alerts_topic, "admin/uptime").await?;

    if let Err(err) = run(&alerts).await {
        let sent = alerts
            .alert("uptime-failed", "Uptime check failed")
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

async fn run(alerts: &AlertPublisher) -> Result<(), Box<dyn std::error::Error>> {
    let project = common::require_env("GOOGLE_CLOUD_PROJECT")?;
    let firestore = common::Firestore::new(&project)?;
    uptime::run(&firestore, alerts).await?;
    Ok(())
}
