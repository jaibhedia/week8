use crate::config::Config;
use crate::models::rune_pool::{DbInterval, DbMeta, DbRunePoolResponse};
use leveldb::database::Database;
use leveldb::kv::KV;
use leveldb::options::{Options, ReadOptions, WriteOptions};
use serde_json;
use std::error::Error;
use std::path::Path;

pub struct LevelDBClient {
    db: Database<i32>, 
}

impl LevelDBClient {
    pub fn new(config: &Config) -> Result<Self, Box<dyn Error>> {
        let mut opts = Options::new();
        opts.create_if_missing = true;
        let db = Database::open(Path::new(&config.leveldb_path), opts)?;
        Ok(LevelDBClient { db })
    }

    pub fn update_rune_pool(&self, response: &DbRunePoolResponse) -> Result<(), Box<dyn Error>> {
        let write_opts = WriteOptions::new();
        let meta_key = 0; 
        let meta_value = serde_json::to_vec(&response.meta)?;
        self.db.put(write_opts, meta_key, &meta_value)?;

        let write_opts = WriteOptions::new(); 
        for (index, interval) in response.intervals.iter().enumerate() {
            let key = index as i32 + 1; 
            let value = serde_json::to_vec(interval)?;
            self.db.put(write_opts, key, &value)?;
        }

        Ok(())
    }

    pub fn get_rune_pool(&self) -> Result<DbRunePoolResponse, Box<dyn Error>> {
        let read_opts = ReadOptions::new();
        let meta_key = 0;
        let meta_value = self.db.get(read_opts, meta_key)?.ok_or("Meta not found")?;
        let meta: DbMeta = serde_json::from_slice(&meta_value)?;


        let mut intervals = Vec::new();
        let mut index = 1;
        loop {
            let read_opts = ReadOptions::new();
            let key = index as i32;
            match self.db.get(read_opts, key)? {
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
    let write_opts = WriteOptions::new();

    self.db.delete(write_opts, 0)?;

   
    let mut index = 1;
    loop {
        let read_opts = ReadOptions::new(); 
        let key = index as i32;
        if self.db.get(read_opts, key)?.is_some() {
            self.db.delete(write_opts, key)?;
            index += 1;
        } else {
            break;
        }
    }

    Ok(())
}

}