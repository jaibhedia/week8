mod config;
mod models;
mod db;
mod api;

use api::handlers::{clear_databases, fetch_and_update_rune_pool, get_rune_pool, update_rune_pool, AppState};
use axum::{routing::get, routing::post, routing::delete, Router};
use config::Config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load();
    let state = AppState::new(config.clone()).await?;
    
    let app = Router::new()
    .route("/update", post(update_rune_pool))
    .route("/get", get(get_rune_pool))
    .route("/fetch-and-update", post(fetch_and_update_rune_pool))
    .route("/clear", delete(clear_databases))
    .with_state(state);

    let config = Config::load();
    let addr = format!("{}:{}", &config.host, &config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    println!("Server running at http://{}", addr);
    axum::serve(listener, app).await?;


    Ok(())
}
