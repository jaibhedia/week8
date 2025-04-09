use crate::config::Config;
use crate::models::rune_pool::{DbInterval, DbMeta, DbRunePoolResponse};
use sqlx::PgPool;
use std::error::Error;
use sqlx::Row;
pub struct PsqlClient {
    pool: PgPool,
}

impl PsqlClient {
    pub async fn new(config: &Config) -> Result<Self, Box<dyn Error>> {
        let pool = PgPool::connect(&config.psql_conn).await?;
        Ok(PsqlClient { pool })
    }

    pub async fn update_rune_pool(&self, response: &DbRunePoolResponse) -> Result<(), Box<dyn Error>> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS meta (
                id SERIAL PRIMARY KEY,
                start_time BIGINT NOT NULL,
                end_time BIGINT NOT NULL,
                start_count BIGINT NOT NULL,
                end_count BIGINT NOT NULL,
                start_units BIGINT NOT NULL,
                end_units BIGINT NOT NULL
            )"
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS intervals (
                id SERIAL PRIMARY KEY,
                start_time BIGINT NOT NULL,
                end_time BIGINT NOT NULL,
                count BIGINT NOT NULL,
                units BIGINT NOT NULL
            )"
        )
        .execute(&self.pool)
        .await?;

        sqlx::query("DELETE FROM meta").execute(&self.pool).await?;
        sqlx::query("DELETE FROM intervals").execute(&self.pool).await?;

        sqlx::query(
            "INSERT INTO meta (start_time, end_time, start_count, end_count, start_units, end_units) 
             VALUES ($1, $2, $3, $4, $5, $6)"
        )
        .bind(response.meta.start_time as i64)
        .bind(response.meta.end_time as i64)
        .bind(response.meta.start_count as i64)
        .bind(response.meta.end_count as i64)
        .bind(response.meta.start_units as i64)
        .bind(response.meta.end_units as i64)
        .execute(&self.pool)
        .await?;

        for interval in &response.intervals {
            sqlx::query(
                "INSERT INTO intervals (start_time, end_time, count, units) 
                 VALUES ($1, $2, $3, $4)"
            )
            .bind(interval.start_time as i64)
            .bind(interval.end_time as i64)
            .bind(interval.count as i64)
            .bind(interval.units as i64)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    pub async fn get_rune_pool(&self) -> Result<DbRunePoolResponse, Box<dyn Error>> {
        let meta_row = sqlx::query(
            "SELECT start_time, end_time, start_count, end_count, start_units, end_units 
             FROM meta LIMIT 1"
        )
        .fetch_one(&self.pool)
        .await?;
        let meta = DbMeta {
            start_time: meta_row.get::<i64, _>("start_time") as u64,
            end_time: meta_row.get::<i64, _>("end_time") as u64,
            start_count: meta_row.get::<i64, _>("start_count") as u64,
            end_count: meta_row.get::<i64, _>("end_count") as u64,
            start_units: meta_row.get::<i64, _>("start_units") as u64,
            end_units: meta_row.get::<i64, _>("end_units") as u64,
        };

    
        let interval_rows = sqlx::query(
            "SELECT start_time, end_time, count, units 
             FROM intervals ORDER BY start_time ASC"
        )
        .fetch_all(&self.pool)
        .await?;
        let intervals = interval_rows
            .into_iter()
            .map(|row| DbInterval {
                start_time: row.get::<i64, _>("start_time") as u64,
                end_time: row.get::<i64, _>("end_time") as u64,
                count: row.get::<i64, _>("count") as u64,
                units: row.get::<i64, _>("units") as u64,
            })
            .collect();

        Ok(DbRunePoolResponse { meta, intervals })
    }

    pub async fn clear(&self) -> Result<(), Box<dyn Error>> {
        sqlx::query("DELETE FROM meta").execute(&self.pool).await?;
        sqlx::query("DELETE FROM intervals").execute(&self.pool).await?;
        Ok(())
    }
}