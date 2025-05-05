use crate::config::Config;
use crate::models::rune_pool::{DbInterval, DbMeta, DbRunePoolResponse};
use leveldb::database::Database;
use leveldb::kv::KV;
use leveldb::options::{Options, ReadOptions, WriteOptions};
use serde_json;
use std::error::Error;
use std::path::Path;
use tracing::warn;

pub struct LevelDBClient {
    db: Database<i32>,
}

impl LevelDBClient {
    pub fn new(config: &Config) -> Result<Self, Box<dyn Error>> {
        // Try to create directory if it doesn't exist
        if let Err(err) = std::fs::create_dir_all(&config.leveldb_path) {
            warn!("Failed to create LevelDB directory: {}", err);
        }
        
        let mut options = Options::new();
        options.create_if_missing = true;
        let db = Database::open(Path::new(&config.leveldb_path), options)?;
        
        Ok(LevelDBClient { db })
    }

    pub fn update_rune_pool(&self, response: &DbRunePoolResponse) -> Result<(), Box<dyn Error>> {
        let write_options = WriteOptions::new();
        let meta_key = 0; // Use 0 as the key for meta
        let meta_value = serde_json::to_vec(&response.meta)?;
        self.db.put(write_options, meta_key, &meta_value)?;

        for (index, interval) in response.intervals.iter().enumerate() {
            let key = (index + 1) as i32; // Use index + 1 as key for intervals, to avoid collision with meta
            let value = serde_json::to_vec(interval)?;
            self.db.put(WriteOptions::new(), key, &value)?;
        }
        Ok(())
    }

    pub fn get_rune_pool(&self) -> Result<DbRunePoolResponse, Box<dyn Error>> {
        let read_options = ReadOptions::new();
        let meta_key = 0;
        
        // Get meta data
        let meta_value = match self.db.get(read_options, meta_key)? {
            Some(value) => value,
            None => {
                // Return empty response with default values if no data exists
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

        // Get intervals
        let mut intervals = Vec::new();
        let mut index = 1; // Start from 1 since 0 is used for meta
        loop {
            let key = index as i32;
            match self.db.get(ReadOptions::new(), key)? {
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

    pub fn clear(&self) -> Result<(), Box<dyn Error>> {
        let write_options = WriteOptions::new();
        
        // Delete meta
        if self.db.get(ReadOptions::new(), 0)?.is_some() {
            self.db.delete(write_options, 0)?;
        }
        
        // Delete intervals
        let mut index = 1;
        loop {
            let key = index as i32;
            if self.db.get(ReadOptions::new(), key)?.is_some() {
                self.db.delete(WriteOptions::new(), key)?;
                index += 1;
            } else {
                break;
            }
        }
        
        Ok(())
    }
}