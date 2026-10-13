//! Serve the admin API.

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = common::require_env("PORT")?;
    let project = common::require_env("GOOGLE_CLOUD_PROJECT")?;
    let firestore = common::Firestore::new(&project)?;
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;

    let app = axum::Router::new().nest("/api", api::router(firestore));
    axum::serve(listener, app).await?;
    Ok(())
}
