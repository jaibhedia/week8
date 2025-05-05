use crate::config::Config;
use crate::models::rune_pool::{DbInterval, DbMeta, DbRunePoolResponse};
use rocksdb::{Options, DB};
use serde_json;
use thiserror::Error;
use tracing::warn;
use std::fs;

#[derive(Error, Debug)]
pub enum RocksDBError {
    #[error("RocksDB error: {0}")]
    DBError(#[from] rocksdb::Error),
    
    #[error("Serialization error: {0}")]
    SerdeError(#[from] serde_json::Error),
    
    #[error("Data not found: {0}")]
    NotFoundError(String),
}

pub struct RocksDBClient {
    db: DB,
}

impl RocksDBClient {
    pub fn new(config: &Config) -> Result<Self, RocksDBError> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        
        // Ensure the directory exists
        if let Err(err) = fs::create_dir_all(&config.rocksdb_path) {
            warn!("Failed to create RocksDB directory: {}", err);
            // Continue anyway, the open call will fail if there's a real problem
        }
        
        let db = DB::open(&opts, &config.rocksdb_path)?;
        Ok(RocksDBClient { db })
    }

    pub fn update_rune_pool(&self, response: &DbRunePoolResponse) -> Result<(), RocksDBError> {
        let meta_key = "meta".as_bytes();
        let meta_value = serde_json::to_vec(&response.meta)?;
        self.db.put(meta_key, meta_value)?;

        for (index, interval) in response.intervals.iter().enumerate() {
            let key = format!("interval_{}", index).into_bytes();
            let value = serde_json::to_vec(interval)?;
            self.db.put(&key, value)?;
        }
        Ok(())
    }

    pub fn get_rune_pool(&self) -> Result<DbRunePoolResponse, RocksDBError> {
        let meta_key = "meta".as_bytes();
        let meta_value = match self.db.get(meta_key)? {
            Some(data) => data,
            None => {
                // Return default empty response if no data exists
                return Ok(DbRunePoolResponse {
                    meta: DbMeta {
                        start_time: 0,
                        end_time: 0,
                        start_count: 0,
                        end_count: 0,
                        start_units: 0,
                        end_units: 0,
                    },
                    intervals: vec![],
                });
            }
        };
            
        let meta: DbMeta = serde_json::from_slice(&meta_value)?;

        let mut intervals = Vec::new();
        let mut index = 0;
        loop {
            let key = format!("interval_{}", index).into_bytes();
            match self.db.get(&key)? {
                Some(value) => {
                    let interval: DbInterval = serde_json::from_slice(&value)?;
                    intervals.push(interval);
                    index += 1;
                }
                None => break,
            }
        }
        Ok(DbRunePoolResponse { meta, intervals })
    }

    pub fn clear(&self) -> Result<(), RocksDBError> {
        // Delete meta if it exists
        if self.db.get("meta".as_bytes())?.is_some() {
            self.db.delete("meta".as_bytes())?;
        }
        
        // Delete all intervals
        let mut index = 0;
        loop {
            let key = format!("interval_{}", index).into_bytes();
            if self.db.get(&key)?.is_some() {
                self.db.delete(&key)?;
                index += 1;
            } else {
                break;
            }
        }
        Ok(())
    }
}