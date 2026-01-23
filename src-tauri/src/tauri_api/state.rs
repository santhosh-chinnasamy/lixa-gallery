use sqlx::{Pool, Sqlite};

pub type Db = Pool<Sqlite>;

pub struct AppState {
    pub db: Db,
}
