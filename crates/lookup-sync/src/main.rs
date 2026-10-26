use alerts::AlertPublisher;

#[tokio::main]
async fn main() -> std::process::ExitCode {
    common::log::init();
    common::exit_code(start().await)
}

async fn start() -> Result<(), Box<dyn std::error::Error>> {
    let alerts_topic = common::require_env("ALERTS_TOPIC")?;
    let alerts = AlertPublisher::new(&alerts_topic, "admin/lookup-sync").await?;

    if let Err(err) = run().await {
        alerts
            .alert("sync-failed", "Lookup sync failed")
            .detail(common::report(&*err))
            .send()
            .await;
        return Err(err);
    }
    Ok(())
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let client_id = common::require_env("CAMBRIDGE_UIS_API_KEY")?;
    let client_secret = common::require_env("CAMBRIDGE_UIS_API_SECRET")?;
    let topic = common::require_env("PUBSUB_TOPIC")?;
    let project = common::require_env("GOOGLE_CLOUD_PROJECT")?;
    lookup_sync::sync(&client_id, &client_secret, &topic, &project).await?;
    Ok(())
}
