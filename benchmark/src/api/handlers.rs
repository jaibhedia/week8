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
use tracing::{info, warn};

/// Helper function to measure database operation time and handle errors gracefully
async fn measure_db_op<T, F, Fut>(name: &str, timings: &mut HashMap<String, u128>, errors: &mut HashMap<String, String>, op: F) -> Option<T> 
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<T, Box<dyn std::error::Error>>>
{
    let start = Instant::now();
    match op().await {
        Ok(result) => {
            timings.insert(name.to_string(), start.elapsed().as_millis());
            Some(result)
        },
        Err(e) => {
            errors.insert(name.to_string(), e.to_string());
            None
        }
    }
}

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

    // Fetch data from Midgard API
    info!("Fetching data from Midgard API: {}", url);
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
    let mut errors = HashMap::new();

    // Perform database operations with timing metrics
    info!("Migrating data to databases with {} records", response.intervals.len());

    // LevelDB
    if let Some(client) = &state.leveldb {
        measure_db_op("leveldb", &mut timings, &mut errors, || async {
            client.update_rune_pool(&db_response)
        }).await;
    } else {
        errors.insert("leveldb".to_string(), "Client not initialized".to_string());
    }

    // RocksDB
    if let Some(client) = &state.rocksdb {
        measure_db_op("rocksdb", &mut timings, &mut errors, || async {
            client.update_rune_pool(&db_response)
        }).await;
    } else {
        errors.insert("rocksdb".to_string(), "Client not initialized".to_string());
    }

    // SurrealDB
    if let Some(client) = &state.surrealdb {
        let client_lock = client.lock().await;
        measure_db_op("surrealdb", &mut timings, &mut errors, || async {
            client_lock.update_rune_pool(&db_response).await
        }).await;
    } else {
        errors.insert("surrealdb".to_string(), "Client not initialized".to_string());
    }

    // PostgreSQL
    if let Some(client) = &state.psql {
        let client_lock = client.lock().await;
        measure_db_op("psql", &mut timings, &mut errors, || async {
            client_lock.update_rune_pool(&db_response).await
        }).await;
    } else {
        errors.insert("psql".to_string(), "Client not initialized".to_string());
    }

    // MongoDB
    if let Some(client) = &state.mongodb {
        let client_lock = client.lock().await;
        measure_db_op("mongodb", &mut timings, &mut errors, || async {
            client_lock.update_rune_pool(&db_response).await
        }).await;
    } else {
        errors.insert("mongodb".to_string(), "Client not initialized".to_string());
    }

    // Return performance metrics
    let perf_metrics = json!({
        "data": {
            "count": response.intervals.len(),
            "time_range": format!("{} to {}", 
                                response.meta.start_time,
                                response.meta.end_time)
        },
        "timings": timings,
        "errors": errors
    });

    Ok((StatusCode::OK, Json(perf_metrics)))
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

/// Structure for benchmarking request
#[derive(Debug, Deserialize)]
pub struct BenchmarkRequest {
    /// Number of records to fetch for testing (optional, default 100)
    count: Option<usize>,
    
    /// Number of iterations for more accurate benchmarking (optional, default 3)
    iterations: Option<usize>,
}

/// Comprehensive benchmarking endpoint that runs multiple tests for more accurate results
pub async fn run_benchmark(
    State(state): State<AppState>,
    Json(req): Json<BenchmarkRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let count = req.count.unwrap_or(100);
    let iterations = req.iterations.unwrap_or(3);
    let mut results = HashMap::new();
    
    // Initialize result structures
    for db in &["leveldb", "rocksdb", "surrealdb", "psql", "mongodb"] {
        results.insert(db.to_string(), json!({
            "write": {
                "times": Vec::<u128>::new(),
                "errors": Vec::<String>::new(),
            },
            "read": {
                "times": Vec::<u128>::new(),
                "errors": Vec::<String>::new(),
            },
        }));
    }
    
    // Fetch data once to reuse across iterations
    info!("Fetching {} records from Midgard API for benchmark", count);
    let url = format!(
        "{}?interval={}&from={}&count={}",
        state.config.api_url,
        state.config.interval,
        state.config.initial_from,
        count
    );
    
    let response = match state
        .http_client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch from Midgard: {}", e))
        .and_then(|resp| async move {
            resp.json::<ApiRunePoolResponse>()
                .await
                .map_err(|e| format!("Failed to parse Midgard response: {}", e))
        })
        .await {
            Ok(resp) => resp,
            Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, e)),
        };
    
    let db_response: DbRunePoolResponse = response.into();
    
    info!("Running benchmark with {} iterations", iterations);
    // Run tests for each database
    for _ in 0..iterations {
        // Clear all databases first
        if let Err(e) = clear_all_databases(&state).await {
            warn!("Failed to clear databases: {}", e);
        }
        
        // Run write tests
        if let Some(client) = &state.leveldb {
            let start = Instant::now();
            match client.update_rune_pool(&db_response) {
                Ok(_) => add_result(&mut results, "leveldb", "write", "times", start.elapsed().as_millis()),
                Err(e) => add_result(&mut results, "leveldb", "write", "errors", e.to_string()),
            }
        }
        
        if let Some(client) = &state.rocksdb {
            let start = Instant::now();
            match client.update_rune_pool(&db_response) {
                Ok(_) => add_result(&mut results, "rocksdb", "write", "times", start.elapsed().as_millis()),
                Err(e) => add_result(&mut results, "rocksdb", "write", "errors", e.to_string()),
            }
        }
        
        if let Some(client) = &state.surrealdb {
            let start = Instant::now();
            match client.lock().await.update_rune_pool(&db_response).await {
                Ok(_) => add_result(&mut results, "surrealdb", "write", "times", start.elapsed().as_millis()),
                Err(e) => add_result(&mut results, "surrealdb", "write", "errors", e.to_string()),
            }
        }
        
        if let Some(client) = &state.psql {
            let start = Instant::now();
            match client.lock().await.update_rune_pool(&db_response).await {
                Ok(_) => add_result(&mut results, "psql", "write", "times", start.elapsed().as_millis()),
                Err(e) => add_result(&mut results, "psql", "write", "errors", e.to_string()),
            }
        }
        
        if let Some(client) = &state.mongodb {
            let start = Instant::now();
            match client.lock().await.update_rune_pool(&db_response).await {
                Ok(_) => add_result(&mut results, "mongodb", "write", "times", start.elapsed().as_millis()),
                Err(e) => add_result(&mut results, "mongodb", "write", "errors", e.to_string()),
            }
        }
        
        // Run read tests
        if let Some(client) = &state.leveldb {
            let start = Instant::now();
            match client.get_rune_pool() {
                Ok(_) => add_result(&mut results, "leveldb", "read", "times", start.elapsed().as_millis()),
                Err(e) => add_result(&mut results, "leveldb", "read", "errors", e.to_string()),
            }
        }
        
        if let Some(client) = &state.rocksdb {
            let start = Instant::now();
            match client.get_rune_pool() {
                Ok(_) => add_result(&mut results, "rocksdb", "read", "times", start.elapsed().as_millis()),
                Err(e) => add_result(&mut results, "rocksdb", "read", "errors", e.to_string()),
            }
        }
        
        if let Some(client) = &state.surrealdb {
            let start = Instant::now();
            match client.lock().await.get_rune_pool().await {
                Ok(_) => add_result(&mut results, "surrealdb", "read", "times", start.elapsed().as_millis()),
                Err(e) => add_result(&mut results, "surrealdb", "read", "errors", e.to_string()),
            }
        }
        
        if let Some(client) = &state.psql {
            let start = Instant::now();
            match client.lock().await.get_rune_pool().await {
                Ok(_) => add_result(&mut results, "psql", "read", "times", start.elapsed().as_millis()),
                Err(e) => add_result(&mut results, "psql", "read", "errors", e.to_string()),
            }
        }
        
        if let Some(client) = &state.mongodb {
            let start = Instant::now();
            match client.lock().await.get_rune_pool().await {
                Ok(_) => add_result(&mut results, "mongodb", "read", "times", start.elapsed().as_millis()),
                Err(e) => add_result(&mut results, "mongodb", "read", "errors", e.to_string()),
            }
        }
    }
    
    // Calculate averages and prepare final report
    let mut summary = HashMap::new();
    for (db, metrics) in results.iter() {
        let write_times: Vec<u128> = serde_json::from_value(metrics["write"]["times"].clone()).unwrap_or_default();
        let read_times: Vec<u128> = serde_json::from_value(metrics["read"]["times"].clone()).unwrap_or_default();
        
        let write_errors: Vec<String> = serde_json::from_value(metrics["write"]["errors"].clone()).unwrap_or_default();
        let read_errors: Vec<String> = serde_json::from_value(metrics["read"]["errors"].clone()).unwrap_or_default();
        
        let write_avg = if !write_times.is_empty() {
            write_times.iter().sum::<u128>() as f64 / write_times.len() as f64
        } else {
            0.0
        };
        
        let read_avg = if !read_times.is_empty() {
            read_times.iter().sum::<u128>() as f64 / read_times.len() as f64
        } else {
            0.0
        };
        
        summary.insert(db.clone(), json!({
            "write": {
                "avg_ms": write_avg,
                "times": write_times,
                "errors": write_errors,
            },
            "read": {
                "avg_ms": read_avg,
                "times": read_times,
                "errors": read_errors,
            }
        }));
    }
    
    // Determine performance ranking
    let mut write_ranking = create_ranking(&summary, "write");
    let mut read_ranking = create_ranking(&summary, "read");
    
    Ok((StatusCode::OK, Json(json!({
        "benchmark_info": {
            "iterations": iterations,
            "record_count": count,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        },
        "results": summary,
        "ranking": {
            "write": write_ranking,
            "read": read_ranking
        }
    }))))
}

/// Clear all databases for testing
async fn clear_all_databases(state: &AppState) -> Result<(), String> {
    if let Some(client) = &state.leveldb {
        client.clear().map_err(|e| format!("Failed to clear LevelDB: {}", e))?;
    }
    
    if let Some(client) = &state.rocksdb {
        client.clear().map_err(|e| format!("Failed to clear RocksDB: {}", e))?;
    }
    
    if let Some(client) = &state.surrealdb {
        client.lock().await.clear().await.map_err(|e| format!("Failed to clear SurrealDB: {}", e))?;
    }
    
    if let Some(client) = &state.psql {
        client.lock().await.clear().await.map_err(|e| format!("Failed to clear PostgreSQL: {}", e))?;
    }
    
    if let Some(client) = &state.mongodb {
        client.lock().await.clear().await.map_err(|e| format!("Failed to clear MongoDB: {}", e))?;
    }
    
    Ok(())
}

/// Helper to add a result to our metrics collection
fn add_result<T: serde::Serialize>(
    results: &mut HashMap<String, serde_json::Value>,
    db: &str, 
    op_type: &str, 
    metric: &str,
    value: T
) {
    if let Some(db_entry) = results.get_mut(db) {
        if let Some(op_entry) = db_entry.get_mut(op_type) {
            if let Some(metric_entry) = op_entry.get_mut(metric) {
                if let Some(array) = metric_entry.as_array_mut() {
                    array.push(json!(value));
                }
            }
        }
    }
}

/// Create performance ranking from results
fn create_ranking(
    summary: &HashMap<String, serde_json::Value>,
    op_type: &str
) -> Vec<(String, f64)> {
    let mut ranking = Vec::new();
    
    for (db, metrics) in summary {
        if let Some(op_metrics) = metrics.get(op_type) {
            if let Some(avg) = op_metrics.get("avg_ms") {
                if let Some(avg_val) = avg.as_f64() {
                    if avg_val > 0.0 {  // Only include if there were valid measurements
                        ranking.push((db.clone(), avg_val));
                    }
                }
            }
        }
    }
    
    // Sort by performance (lowest time first)
    ranking.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    
    ranking
}