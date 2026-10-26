use alerts::AlertPublisher;

#[tokio::main]
async fn main() -> std::process::ExitCode {
    common::log::init();
    common::exit_code(start().await)
}

async fn start() -> Result<(), Box<dyn std::error::Error>> {
    let alerts_topic = common::require_env("ALERTS_TOPIC")?;
    let alerts = AlertPublisher::new(&alerts_topic, "admin/uptime").await?;

    if let Err(err) = run(&alerts).await {
        alerts
            .alert("uptime-failed", "Uptime check failed")
            .detail(common::report(&*err))
            .send()
            .await;
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
