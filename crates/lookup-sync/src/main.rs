#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client_id = std::env::var("CAMBRIDGE_UIS_API_KEY")?;
    let client_secret = std::env::var("CAMBRIDGE_UIS_API_SECRET")?;
    let topic = std::env::var("PUBSUB_TOPIC")?;
    lookup_sync::sync(&client_id, &client_secret, &topic).await?;
    Ok(())
}
