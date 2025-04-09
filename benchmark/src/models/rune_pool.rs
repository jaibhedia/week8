use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use sqlx::FromRow;

#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ApiMeta {
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "startTime")]
    pub start_time: u64,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "endTime")]
    pub end_time: u64,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "startCount")]
    pub start_count: u64,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "endCount")]
    pub end_count: u64,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "startUnits")]
    pub start_units: u64,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "endUnits")]
    pub end_units: u64,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ApiInterval {
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "startTime")]
    pub start_time: u64,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "endTime")]
    pub end_time: u64,
    #[serde_as(as = "DisplayFromStr")]
    pub count: u64,
    #[serde_as(as = "DisplayFromStr")]
    pub units: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ApiRunePoolResponse {
    pub meta: ApiMeta,
    pub intervals: Vec<ApiInterval>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, FromRow)]
pub struct DbMeta {
    pub start_time: u64,
    pub end_time: u64,
    pub start_count: u64,
    pub end_count: u64,
    pub start_units: u64,
    pub end_units: u64,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, FromRow)]
pub struct DbInterval {
    pub start_time: u64,
    pub end_time: u64,
    pub count: u64,
    pub units: u64,
}

#[derive(Debug, Clone)]
pub struct DbRunePoolResponse {
    pub meta: DbMeta,
    pub intervals: Vec<DbInterval>,
}


impl From<ApiRunePoolResponse> for DbRunePoolResponse {
    fn from(api: ApiRunePoolResponse) -> Self {
        DbRunePoolResponse {
            meta: DbMeta {
                start_time: api.meta.start_time,
                end_time: api.meta.end_time,
                start_count: api.meta.start_count,
                end_count: api.meta.end_count,
                start_units: api.meta.start_units,
                end_units: api.meta.end_units,
            },
            intervals: api.intervals.into_iter().map(|i| DbInterval {
                start_time: i.start_time,
                end_time: i.end_time,
                count: i.count,
                units: i.units,
            }).collect(),
        }
    }
}

impl From<DbRunePoolResponse> for ApiRunePoolResponse {
    fn from(db: DbRunePoolResponse) -> Self {
        ApiRunePoolResponse {
            meta: ApiMeta {
                start_time: db.meta.start_time,
                end_time: db.meta.end_time,
                start_count: db.meta.start_count,
                end_count: db.meta.end_count,
                start_units: db.meta.start_units,
                end_units: db.meta.end_units,
            },
            intervals: db.intervals.into_iter().map(|i| ApiInterval {
                start_time: i.start_time,
                end_time: i.end_time,
                count: i.count,
                units: i.units,
            }).collect(),
        }
    }
}