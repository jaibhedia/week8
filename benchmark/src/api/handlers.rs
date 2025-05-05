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
use tracing::{error, info, warn};

#[derive(Clone)]
pub struct AppState {
    config: Config,
    leveldb: Option<Arc<LevelDBClient>>,
    rocksdb: Option<Arc<RocksDBClient>>,
    surrealdb: Option<Arc<Mutex<SurrealDBClient>>>,
    psql: Option<Arc<Mutex<PsqlClient>>>,
    mongodb: Option<Arc<Mutex<MongoDBClient>>>,
    http_client: HttpClient,
}

impl AppState {
    pub async fn new(config: Config) -> Result<Self, String> {
        let leveldb = match LevelDBClient::new(&config) {
            Ok(client) => {
                info!("LevelDB client initialized successfully");
                Some(Arc::new(client))
            },
            Err(e) => {
                warn!("Failed to initialize LevelDB client: {}", e);
                None
            }
        };
        
        let rocksdb = match RocksDBClient::new(&config) {
            Ok(client) => {
                info!("RocksDB client initialized successfully");
                Some(Arc::new(client))
            },
            Err(e) => {
                warn!("Failed to initialize RocksDB client: {}", e);
                None
            }
        };
        
        let surrealdb = match SurrealDBClient::new(&config).await {
            Ok(client) => {
                info!("SurrealDB client initialized successfully");
                Some(Arc::new(Mutex::new(client)))
            },
            Err(e) => {
                warn!("Failed to initialize SurrealDB client: {}", e);
                None
            }
        };
        
        let psql = match PsqlClient::new(&config).await {
            Ok(client) => {
                info!("PostgreSQL client initialized successfully");
                Some(Arc::new(Mutex::new(client)))
            },
            Err(e) => {
                warn!("Failed to initialize PostgreSQL client: {}", e);
                None
            }
        };
        
        let mongodb = match MongoDBClient::new(&config).await {
            Ok(client) => {
                info!("MongoDB client initialized successfully");
                Some(Arc::new(Mutex::new(client)))
            },
            Err(e) => {
                warn!("Failed to initialize MongoDB client: {}", e);
                None
            }
        };
        
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
    let mut errors = HashMap::new();

    // LevelDB
    if let Some(client) = &state.leveldb {
        let start = Instant::now();
        match client.update_rune_pool(&db_response) {
            Ok(_) => {
                timings.insert("leveldb", start.elapsed().as_millis());
            },
            Err(e) => {
                errors.insert("leveldb", e.to_string());
            }
        }
    } else {
        errors.insert("leveldb", "Client not initialized".to_string());
    }

    // RocksDB
    if let Some(client) = &state.rocksdb {
        let start = Instant::now();
        match client.update_rune_pool(&db_response) {
            Ok(_) => {
                timings.insert("rocksdb", start.elapsed().as_millis());
            },
            Err(e) => {
                errors.insert("rocksdb", e.to_string());
            }
        }
    } else {
        errors.insert("rocksdb", "Client not initialized".to_string());
    }

    // SurrealDB
    if let Some(client) = &state.surrealdb {
        let start = Instant::now();
        match client.lock().await.update_rune_pool(&db_response).await {
            Ok(_) => {
                timings.insert("surrealdb", start.elapsed().as_millis());
            },
            Err(e) => {
                errors.insert("surrealdb", e.to_string());
            }
        }
    } else {
        errors.insert("surrealdb", "Client not initialized".to_string());
    }

    // PostgreSQL
    if let Some(client) = &state.psql {
        let start = Instant::now();
        match client.lock().await.update_rune_pool(&db_response).await {
            Ok(_) => {
                timings.insert("psql", start.elapsed().as_millis());
            },
            Err(e) => {
                errors.insert("psql", e.to_string());
            }
        }
    } else {
        errors.insert("psql", "Client not initialized".to_string());
    }

    // MongoDB
    if let Some(client) = &state.mongodb {
        let start = Instant::now();
        match client.lock().await.update_rune_pool(&db_response).await {
            Ok(_) => {
                timings.insert("mongodb", start.elapsed().as_millis());
            },
            Err(e) => {
                errors.insert("mongodb", e.to_string());
            }
        }
    } else {
        errors.insert("mongodb", "Client not initialized".to_string());
    }

    Ok((StatusCode::OK, Json(json!({
        "data": payload,
        "timings": timings,
        "errors": errors
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
            let retrieved_db = state.leveldb.as_ref().ok_or((StatusCode::INTERNAL_SERVER_ERROR, "LevelDB client not initialized".to_string()))?.get_rune_pool().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            (retrieved_db.into(), start.elapsed().as_millis())
        }
        "rocksdb" => {
            let start = Instant::now();
            let retrieved_db = state.rocksdb.as_ref().ok_or((StatusCode::INTERNAL_SERVER_ERROR, "RocksDB client not initialized".to_string()))?.get_rune_pool().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            (retrieved_db.into(), start.elapsed().as_millis())
        }
        "surrealdb" => {
            let start = Instant::now();
            let retrieved_db = state.surrealdb.as_ref().ok_or((StatusCode::INTERNAL_SERVER_ERROR, "SurrealDB client not initialized".to_string()))?.lock().await.get_rune_pool().await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            (retrieved_db.into(), start.elapsed().as_millis())
        }
        "psql" => {
            let start = Instant::now();
            let retrieved_db = state.psql.as_ref().ok_or((StatusCode::INTERNAL_SERVER_ERROR, "PostgreSQL client not initialized".to_string()))?.lock().await.get_rune_pool().await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            (retrieved_db.into(), start.elapsed().as_millis())
        }
        "mongodb" => {
            let start = Instant::now();
            let retrieved_db = state.mongodb.as_ref().ok_or((StatusCode::INTERNAL_SERVER_ERROR, "MongoDB client not initialized".to_string()))?.lock().await.get_rune_pool().await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
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
    if let Some(client) = &state.leveldb {
        let start = Instant::now();
        client.update_rune_pool(&db_response).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        timings.insert("leveldb", start.elapsed().as_millis());
    }

    // RocksDB
    if let Some(client) = &state.rocksdb {
        let start = Instant::now();
        client.update_rune_pool(&db_response).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        timings.insert("rocksdb", start.elapsed().as_millis());
    }

    // SurrealDB
    if let Some(client) = &state.surrealdb {
        let start = Instant::now();
        client.lock().await.update_rune_pool(&db_response).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        timings.insert("surrealdb", start.elapsed().as_millis());
    }

    // PostgreSQL
    if let Some(client) = &state.psql {
        let start = Instant::now();
        client.lock().await.update_rune_pool(&db_response).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        timings.insert("psql", start.elapsed().as_millis());
    }

    // MongoDB
    if let Some(client) = &state.mongodb {
        let start = Instant::now();
        client.lock().await.update_rune_pool(&db_response).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        timings.insert("mongodb", start.elapsed().as_millis());
    }

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
    if let Some(client) = &state.leveldb {
        let start = Instant::now();
        client.clear().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        timings.insert("leveldb", start.elapsed().as_millis());
    }

    // RocksDB
    if let Some(client) = &state.rocksdb {
        let start = Instant::now();
        client.clear().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        timings.insert("rocksdb", start.elapsed().as_millis());
    }

    // SurrealDB
    if let Some(client) = &state.surrealdb {
        let start = Instant::now();
        client.lock().await.clear().await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        timings.insert("surrealdb", start.elapsed().as_millis());
    }

    // PostgreSQL
    if let Some(client) = &state.psql {
        let start = Instant::now();
        client.lock().await.clear().await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        timings.insert("psql", start.elapsed().as_millis());
    }

    // MongoDB
    if let Some(client) = &state.mongodb {
        let start = Instant::now();
        client.lock().await.clear().await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        timings.insert("mongodb", start.elapsed().as_millis());
    }

    Ok((StatusCode::OK, Json(json!({ "timings": timings }))))
}

pub async fn health_check(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let mut status = HashMap::new();
    
    // Check LevelDB
    status.insert("leveldb", state.leveldb.is_some());
    
    // Check RocksDB
    status.insert("rocksdb", state.rocksdb.is_some());
    
    // Check SurrealDB
    status.insert("surrealdb", state.surrealdb.is_some());
    
    // Check PostgreSQL
    status.insert("psql", state.psql.is_some());
    
    // Check MongoDB
    status.insert("mongodb", state.mongodb.is_some());

    // Add server info
    let server_info = json!({
        "host": state.config.host,
        "port": state.config.port,
    });

    Ok((StatusCode::OK, Json(json!({
        "status": "OK",
        "databases": status,
        "server": server_info,
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))))
}