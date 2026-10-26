//! Serve the public admin API.

#[tokio::main]
async fn main() -> std::process::ExitCode {
    common::log::init();
    common::exit_code(start().await)
}

async fn start() -> Result<(), Box<dyn std::error::Error>> {
    let port = common::require_env("PORT")?;
    let project = common::require_env("GOOGLE_CLOUD_PROJECT")?;
    let firestore = common::Firestore::new(&project)?;
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;

    axum::serve(listener, api::router(firestore)).await?;
    Ok(())
}
