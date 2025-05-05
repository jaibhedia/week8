use crate::config::Config;
use crate::db::leveldb::LevelDBClient;
use crate::db::mongodb::MongoDBClient;
use crate::db::psql::PsqlClient;
use crate::db::rocksdb::RocksDBClient;
use crate::db::surrealdb::SurrealDBClient;
use crate::models::rune_pool::{ApiRunePoolResponse, DbRunePoolResponse};
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use reqwest::Client as HttpClient;
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct AppState {
    config: Config,
    leveldb: Arc<LevelDBClient>,
    rocksdb: Arc<RocksDBClient>,
    surrealdb: Arc<Mutex<SurrealDBClient>>,
    psql: Arc<Mutex<PsqlClient>>,
    mongodb: Arc<Mutex<MongoDBClient>>,
    http_client: HttpClient,
}

impl AppState {
    pub async fn new(config: Config) -> Result<Self, String> {
        let leveldb = Arc::new(LevelDBClient::new(&config).map_err(|e| e.to_string())?);
        let rocksdb = Arc::new(RocksDBClient::new(&config).map_err(|e| e.to_string())?);
        let surrealdb = Arc::new(Mutex::new(SurrealDBClient::new(&config).await.map_err(|e| e.to_string())?));
        let psql = Arc::new(Mutex::new(PsqlClient::new(&config).await.map_err(|e| e.to_string())?));
        let mongodb = Arc::new(Mutex::new(MongoDBClient::new(&config).await.map_err(|e| e.to_string())?));
        let http_client = HttpClient::new();

        Ok(AppState {
            config,
            leveldb,
            rocksdb,
            surrealdb,
            psql,
            mongodb,
            http_client,
        })
    }
}

// Update request type
#[derive(Debug, Deserialize)]
pub struct UpdateRequest {
    count: Option<usize>,
}

pub async fn update_rune_pool(
    State(state): State<AppState>,
    Json(payload): Json<ApiRunePoolResponse>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let db_response: DbRunePoolResponse = payload.clone().into();
    let mut timings = HashMap::new();

    // LevelDB
    let start = Instant::now();
    state.leveldb.update_rune_pool(&db_response).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    timings.insert("leveldb", start.elapsed().as_millis());

    // RocksDB
    let start = Instant::now();
    state.rocksdb.update_rune_pool(&db_response).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    timings.insert("rocksdb", start.elapsed().as_millis());

    // SurrealDB
    let start = Instant::now();
    state.surrealdb.lock().await.update_rune_pool(&db_response).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    timings.insert("surrealdb", start.elapsed().as_millis());

    // PostgreSQL
    let start = Instant::now();
    state.psql.lock().await.update_rune_pool(&db_response).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    timings.insert("psql", start.elapsed().as_millis());

    // MongoDB
    let start = Instant::now();
    state.mongodb.lock().await.update_rune_pool(&db_response).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    timings.insert("mongodb", start.elapsed().as_millis());

    Ok((StatusCode::OK, Json(json!({
        "data": payload,
        "timings": timings
    }))))
}

pub async fn get_rune_pool(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let db = params.get("db").ok_or((
        StatusCode::BAD_REQUEST,
        "Missing 'db' query parameter".to_string(),
    ))?;

    let (retrieved_api, timing): (ApiRunePoolResponse, u128) = match db.as_str() {
        "leveldb" => {
            let start = Instant::now();
            let retrieved_db = state.leveldb.get_rune_pool().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            (retrieved_db.into(), start.elapsed().as_millis())
        }
        "rocksdb" => {
            let start = Instant::now();
            let retrieved_db = state.rocksdb.get_rune_pool().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            (retrieved_db.into(), start.elapsed().as_millis())
        }
        "surrealdb" => {
            let start = Instant::now();
            let retrieved_db = state.surrealdb.lock().await.get_rune_pool().await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            (retrieved_db.into(), start.elapsed().as_millis())
        }
        "psql" => {
            let start = Instant::now();
            let retrieved_db = state.psql.lock().await.get_rune_pool().await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            (retrieved_db.into(), start.elapsed().as_millis())
        }
        "mongodb" => {
            let start = Instant::now();
            let retrieved_db = state.mongodb.lock().await.get_rune_pool().await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            (retrieved_db.into(), start.elapsed().as_millis())
        }
        _ => return Err((StatusCode::BAD_REQUEST, format!("Unknown database: {}", db))),
    };

    Ok((StatusCode::OK, Json(json!({
        "data": retrieved_api,
        "timing": timing
    }))))
}

pub async fn fetch_and_update_rune_pool(
    State(state): State<AppState>,
    Json(req): Json<UpdateRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Allow custom count or use default
    let count = req.count.unwrap_or(400);
    
    let url = format!(
        "{}?interval={}&from={}&count={}",
        state.config.api_url,
        state.config.interval,
        state.config.initial_from,
        count
    );

    let response = state
        .http_client
        .get(&url)
        .send()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to fetch from Midgard: {}", e)))?
        .json::<ApiRunePoolResponse>()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to parse Midgard response: {}", e)))?;

    let db_response: DbRunePoolResponse = response.clone().into();
    let mut timings = HashMap::new();

    // LevelDB
    let start = Instant::now();
    state.leveldb.update_rune_pool(&db_response).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    timings.insert("leveldb", start.elapsed().as_millis());

    // RocksDB
    let start = Instant::now();
    state.rocksdb.update_rune_pool(&db_response).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    timings.insert("rocksdb", start.elapsed().as_millis());

    // SurrealDB
    let start = Instant::now();
    state.surrealdb.lock().await.update_rune_pool(&db_response).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    timings.insert("surrealdb", start.elapsed().as_millis());

    // PostgreSQL
    let start = Instant::now();
    state.psql.lock().await.update_rune_pool(&db_response).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    timings.insert("psql", start.elapsed().as_millis());

    // MongoDB
    let start = Instant::now();
    state.mongodb.lock().await.update_rune_pool(&db_response).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    timings.insert("mongodb", start.elapsed().as_millis());

    Ok((StatusCode::OK, Json(json!({
        "data": response,
        "timings": timings
    }))))
}

pub async fn clear_databases(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let mut timings = HashMap::new();

    // LevelDB
    let start = Instant::now();
    state.leveldb.clear().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    timings.insert("leveldb", start.elapsed().as_millis());

    // RocksDB
    let start = Instant::now();
    state.rocksdb.clear().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    timings.insert("rocksdb", start.elapsed().as_millis());

    // SurrealDB
    let start = Instant::now();
    state.surrealdb.lock().await.clear().await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    timings.insert("surrealdb", start.elapsed().as_millis());

    // PostgreSQL
    let start = Instant::now();
    state.psql.lock().await.clear().await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    timings.insert("psql", start.elapsed().as_millis());

    // MongoDB
    let start = Instant::now();
    state.mongodb.lock().await.clear().await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    timings.insert("mongodb", start.elapsed().as_millis());

    Ok((StatusCode::OK, Json(json!({ "timings": timings }))))
}