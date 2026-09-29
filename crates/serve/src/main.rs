//! Serve the admin API.

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_owned());
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;

    let app = axum::Router::new().nest("/api", api::router());
    axum::serve(listener, app).await
}
