use crate::config::Config;
use crate::models::rune_pool::{DbInterval, DbMeta, DbRunePoolResponse};
use surrealdb::engine::remote::ws::{Ws, Client};
use surrealdb::opt::auth::Root;
use surrealdb::Surreal;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SurrealDBError {
    #[error("SurrealDB connection error: {0}")]
    ConnectionError(#[from] surrealdb::Error),
    
    #[error("Data not found: {0}")]
    NotFoundError(String),
    
    #[error("Other error: {0}")]
    Other(String),
}

pub struct SurrealDBClient {
    db: Surreal<Client>,
}

impl SurrealDBClient {
    pub async fn new(config: &Config) -> Result<Self, SurrealDBError> {
        let db = Surreal::new::<Ws>(&config.surrealdb_url)
            .await
            .map_err(SurrealDBError::ConnectionError)?;
            
        db.signin(Root {
            username: "root",
            password: "root",
        })
        .await
        .map_err(SurrealDBError::ConnectionError)?;
        
        db.use_ns("runepool_ns").use_db("runepool_db")
            .await
            .map_err(SurrealDBError::ConnectionError)?;
            
        Ok(SurrealDBClient { db })
    }

    pub async fn update_rune_pool(&self, response: &DbRunePoolResponse) -> Result<(), SurrealDBError> {
        self.db
            .query("CREATE meta SET start_time = $start_time, end_time = $end_time, start_count = $start_count, end_count = $end_count, start_units = $start_units, end_units = $end_units")
            .bind(("start_time", response.meta.start_time))
            .bind(("end_time", response.meta.end_time))
            .bind(("start_count", response.meta.start_count))
            .bind(("end_count", response.meta.end_count))
            .bind(("start_units", response.meta.start_units))
            .bind(("end_units", response.meta.end_units))
            .await
            .map_err(SurrealDBError::ConnectionError)?
            .check()
            .map_err(|e| SurrealDBError::Other(e.to_string()))?;

        for interval in &response.intervals {
            let query = format!(
                "CREATE interval:{} SET start_time = $start_time, end_time = $end_time, count = $count, units = $units",
                interval.start_time
            );
            self.db
                .query(&query)
                .bind(("start_time", interval.start_time))
                .bind(("end_time", interval.end_time))
                .bind(("count", interval.count))
                .bind(("units", interval.units))
                .await
                .map_err(SurrealDBError::ConnectionError)?
                .check()
                .map_err(|e| SurrealDBError::Other(e.to_string()))?;
        }

        Ok(())
    }

    pub async fn get_rune_pool(&self) -> Result<DbRunePoolResponse, SurrealDBError> {
        let metas: Vec<DbMeta> = self.db
            .query("SELECT start_time, end_time, start_count, end_count, start_units, end_units FROM meta")
            .await
            .map_err(SurrealDBError::ConnectionError)?
            .take(0)
            .map_err(|e| SurrealDBError::Other(e.to_string()))?;
        let meta = metas.into_iter().next().ok_or(SurrealDBError::NotFoundError("Meta not found".to_string()))?;

        let intervals: Vec<DbInterval> = self.db
            .query("SELECT start_time, end_time, count, units FROM interval ORDER BY start_time ASC")
            .await
            .map_err(SurrealDBError::ConnectionError)?
            .take(0)
            .map_err(|e| SurrealDBError::Other(e.to_string()))?;

        Ok(DbRunePoolResponse { meta, intervals })
    }

    pub async fn clear(&self) -> Result<(), SurrealDBError> {
        self.db.query("DELETE meta")
            .await
            .map_err(SurrealDBError::ConnectionError)?
            .check()
            .map_err(|e| SurrealDBError::Other(e.to_string()))?;
        self.db.query("DELETE interval")
            .await
            .map_err(SurrealDBError::ConnectionError)?
            .check()
            .map_err(|e| SurrealDBError::Other(e.to_string()))?;
        Ok(())
    }
}