use crate::config::Config;
use crate::models::rune_pool::{DbInterval, DbMeta, DbRunePoolResponse};
use mongodb::{bson::doc, Client, Collection};
use std::error::Error;

pub struct MongoDBClient {
    meta_coll: Collection<DbMeta>,
    intervals_coll: Collection<DbInterval>,
}

impl MongoDBClient{
    pub async fn new(config : &Config) -> Result<Self, Box<dyn Error>>{
        let client = Client::with_uri_str(&config.mongodb_uri).await?;
        let db   = client.database(&config.db_name);
        let meta_coll = db.collection::<DbMeta>("meta");
        let intervals_coll = db.collection::<DbInterval>("intervals");
        Ok(MongoDBClient {
            meta_coll,
            intervals_coll,
        })
    }

    pub async fn update_rune_pool(&self , response : &DbRunePoolResponse)->Result<(),Box<dyn Error>>{
        self.meta_coll.delete_many(doc! {}).await?;
        self.intervals_coll.delete_many(doc! {}).await?;

        self.meta_coll.insert_one(&response.meta).await?;

        self.intervals_coll.insert_many(response.intervals.clone())
        .await?;
    
        Ok(())
    }

    pub async fn get_rune_pool(&self) -> Result<DbRunePoolResponse, Box<dyn Error>>{
        let meta = self.meta_coll.find_one(doc!{}).await?.ok_or("Meta not found")?;

        let mut intervals_cursor = self.intervals_coll.find(doc! {})
        .await?;
        
        let mut intervals = Vec::new();

        while intervals_cursor.advance().await?{
            intervals.push(intervals_cursor.deserialize_current()?);
        }

        intervals.sort_by_key(|i| i.start_time);

        Ok(DbRunePoolResponse { meta, intervals })
    }

    pub async fn clear(&self) -> Result<(), Box<dyn Error>> {
        self.meta_coll.delete_many(doc! {}).await?;
        self.intervals_coll.delete_many(doc! {}).await?;
        Ok(())
    }
}