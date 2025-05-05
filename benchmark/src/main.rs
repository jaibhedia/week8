mod config;
mod models;
mod db;
mod api;

use api::handlers::{clear_databases, fetch_and_update_rune_pool, get_rune_pool, update_rune_pool, health_check, AppState};
use axum::{routing::get, routing::post, routing::delete, Router};
use config::Config;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            "benchmark=info,tower_http=debug,axum::rejection=trace".into()
        }))
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting benchmark application");
    
    let config = Config::load();
    info!("Configuration loaded");
    
    let state = AppState::new(config.clone()).await?;
    info!("Application state initialized");
    
    let app = Router::new()
        .route("/update", post(update_rune_pool))
        .route("/get", get(get_rune_pool))
        .route("/fetch-and-update", post(fetch_and_update_rune_pool))
        .route("/clear", delete(clear_databases))
        .route("/health", get(health_check))
        .with_state(state);

    let addr = format!("{}:{}", &config.host, &config.port);
    info!("Server starting on http://{}", addr);
    
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("Server running at http://{}", addr);
    
    axum::serve(listener, app).await?;

    Ok(())
}
