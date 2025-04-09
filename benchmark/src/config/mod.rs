use chrono::{Duration, Utc};
use std::env;

#[derive(Clone)]
pub struct Config {
    pub api_url: String,
    pub interval: String,
    pub initial_from: u64,
    pub rocksdb_path: String,
    pub leveldb_path: String,
    pub surrealdb_url: String,
    pub psql_conn: String,
    pub mongodb_uri: String,
    pub db_name: String,
    pub host: String,
    pub port: u16,
}

impl Config {
    pub fn load() -> Self {
        dotenvy::dotenv().ok();

        let api_url = env::var("API_URL")
            .unwrap_or_else(|_| "https://midgard.ninerealms.com/v2/history/runepool".to_string());
        let interval = env::var("INTERVAL").unwrap_or_else(|_| "hour".to_string());

        let six_months_ago = Utc::now() - Duration::days(6 * 30); 
        let initial_from = six_months_ago.timestamp() as u64;

        let rocksdb_path =
            env::var("ROCKSDB_PATH").unwrap_or_else(|_| "/tmp/data/rocksdb".to_string());
        let leveldb_path =
            env::var("LEVELDB_PATH").unwrap_or_else(|_| "/tmp/data/leveldb".to_string());
        let surrealdb_url =
            env::var("SURREALDB_URL").unwrap_or_else(|_| "127.0.0.1:8000".to_string());
        let psql_conn = env::var("PSQL_CONN")
            .unwrap_or_else(|_| "postgres://user:password@localhost:5432/runepool".to_string());
        let mongodb_uri = env::var("MONGODB_URI")
            .unwrap_or_else(|_| "mongodb://localhost:27017/runepool".to_string());
        let db_name = env::var("DB_NAME")
            .unwrap_or_else(|_| "runepool".to_string());

        let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string()).parse::<u16>().expect("Invalid PORT value");

        Config {
            api_url,
            interval,
            initial_from,
            rocksdb_path,
            leveldb_path,
            surrealdb_url,
            psql_conn,
            mongodb_uri,
            db_name,
            host,
            port,
        }
    }
}
