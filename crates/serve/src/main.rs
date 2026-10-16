//! Serve the public admin API.

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = common::require_env("PORT")?;
    let project = common::require_env("GOOGLE_CLOUD_PROJECT")?;
    let firestore = common::Firestore::new(&project)?;
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;

    let api = api::router(firestore);
    let app = axum::Router::new()
        .nest("/api/public", api.clone())
        // todo(khm39): remove when ready
        .nest("/api", api);
    axum::serve(listener, app).await?;
    Ok(())
}
