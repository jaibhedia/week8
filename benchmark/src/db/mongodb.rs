use crate::config::Config;
use crate::models::rune_pool::{DbInterval, DbMeta, DbRunePoolResponse};
use mongodb::{
    bson::doc, 
    Client, 
    Collection,
    options::{ClientOptions, ServerApi, ServerApiVersion}
};
use std::error::Error;
use std::time::Duration;
use tracing::{info, warn};

pub struct MongoDBClient {
    meta_coll: Collection<DbMeta>,
    intervals_coll: Collection<DbInterval>,
}

impl MongoDBClient {
    pub async fn new(config: &Config) -> Result<Self, Box<dyn Error>> {
        // Parse connection string
        let mut client_options = match ClientOptions::parse(&config.mongodb_uri).await {
            Ok(options) => {
                info!("MongoDB connection string parsed successfully");
                options
            },
            Err(e) => {
                warn!("Failed to parse MongoDB connection string: {}", e);
                return Err(Box::new(e));
            }
        };

        // Configure connection options for Atlas
        client_options.server_selection_timeout = Some(Duration::from_secs(5));
        client_options.connect_timeout = Some(Duration::from_secs(10));
        
        // Set server API version
        let server_api = ServerApi::builder()
            .version(ServerApiVersion::V1)
            .build();
        client_options.server_api = Some(server_api);
        
        // Create client
        let client = match Client::with_options(client_options) {
            Ok(client) => client,
            Err(e) => {
                warn!("Failed to create MongoDB client: {}", e);
                return Err(Box::new(e));
            }
        };
        
        // Test connection with timeout
        match tokio::time::timeout(
            Duration::from_secs(5),
            client.database("admin").run_command(doc! {"ping": 1}, None)
        ).await {
            Ok(Ok(_)) => info!("Successfully connected to MongoDB"),
            Ok(Err(e)) => warn!("MongoDB ping failed, but continuing: {}", e),
            Err(_) => warn!("MongoDB ping timed out after 5 seconds"),
        }
        
        // Access database and collections
        let db = client.database(&config.db_name);
        let meta_coll = db.collection::<DbMeta>("meta");
        let intervals_coll = db.collection::<DbInterval>("intervals");
        
        Ok(MongoDBClient {
            meta_coll,
            intervals_coll,
        })
    }

    pub async fn update_rune_pool(&self, response: &DbRunePoolResponse) -> Result<(), Box<dyn Error>> {
        // Clear existing data
        self.meta_coll.delete_many(doc! {}, None).await?;
        self.intervals_coll.delete_many(doc! {}, None).await?;

        // Insert new meta data
        match self.meta_coll.insert_one(&response.meta, None).await {
            Ok(_) => info!("Successfully inserted meta data"),
            Err(e) => return Err(Box::new(e)),
        }

        // Insert intervals data if we have any
        if !response.intervals.is_empty() {
            match self.intervals_coll.insert_many(response.intervals.clone(), None).await {
                Ok(result) => info!("Successfully inserted {} interval records", result.inserted_ids.len()),
                Err(e) => return Err(Box::new(e)),
            }
        }
    
        Ok(())
    }

    pub async fn get_rune_pool(&self) -> Result<DbRunePoolResponse, Box<dyn Error>> {
        let meta = match self.meta_coll.find_one(doc!{}, None).await? {
            Some(meta) => meta,
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

        let mut intervals_cursor = self.intervals_coll.find(doc! {}, None).await?;
        
        let mut intervals = Vec::new();

        while intervals_cursor.advance().await? {
            intervals.push(intervals_cursor.deserialize_current()?);
        }

        intervals.sort_by_key(|i| i.start_time);

        Ok(DbRunePoolResponse { meta, intervals })
    }

    pub async fn clear(&self) -> Result<(), Box<dyn Error>> {
        match self.meta_coll.delete_many(doc! {}, None).await {
            Ok(result) => info!("Cleared {} meta documents", result.deleted_count),
            Err(e) => return Err(Box::new(e)),
        }
        
        match self.intervals_coll.delete_many(doc! {}, None).await {
            Ok(result) => info!("Cleared {} interval documents", result.deleted_count),
            Err(e) => return Err(Box::new(e)),
        }
        
        Ok(())
    }
}