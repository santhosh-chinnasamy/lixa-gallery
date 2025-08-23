use serde::{Deserialize, Serialize};
use sqlx::{prelude::FromRow, Pool, Sqlite};

pub type Db = Pool<Sqlite>;

pub struct AppState {
    pub db: Db,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Favourite {
    pub path: String,
}
