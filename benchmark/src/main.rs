mod config;
mod models;
mod db;
mod api;

use api::handlers::{
    clear_databases, 
    fetch_and_update_rune_pool, 
    get_rune_pool, 
    update_rune_pool, 
    health_check, 
    run_benchmark,
    AppState
};
use axum::{routing::get, routing::post, routing::delete, Router};
use config::Config;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use std::fs;

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
    
    // Create necessary directories for databases
    ensure_directories(&config)?;
    
    let state = AppState::new(config.clone()).await?;
    info!("Application state initialized");
    
    let app = Router::new()
        .route("/update", post(update_rune_pool))
        .route("/get", get(get_rune_pool))
        .route("/fetch-and-update", post(fetch_and_update_rune_pool))
        .route("/clear", delete(clear_databases))
        .route("/health", get(health_check))
        .route("/benchmark", post(run_benchmark))  // Add the new benchmark endpoint
        .with_state(state);

    let addr = format!("{}:{}", &config.host, &config.port);
    info!("Server starting on http://{}", addr);
    
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("Server running at http://{}", addr);
    
    axum::serve(listener, app).await?;

    Ok(())
}

fn ensure_directories(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    // Ensure LevelDB directory exists
    let leveldb_path = &config.leveldb_path;
    if !leveldb_path.is_empty() {
        info!("Creating LevelDB directory: {}", leveldb_path);
        fs::create_dir_all(leveldb_path)?;
    }
    
    // Ensure RocksDB directory exists
    let rocksdb_path = &config.rocksdb_path;
    if !rocksdb_path.is_empty() {
        info!("Creating RocksDB directory: {}", rocksdb_path);
        fs::create_dir_all(rocksdb_path)?;
    }
    
    Ok(())
}
