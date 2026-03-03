use futures::future::BoxFuture;
use gallery_core::models::{Favourite, PhotoMetadata};
use gallery_core::repos::{FavouriteRepository, PhotoRepository};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqliteSynchronous},
    QueryBuilder, Row, Sqlite, SqlitePool,
};
use std::{path::PathBuf, str::FromStr};

pub async fn setup_db(app_data_dir: PathBuf, pkg_name: &str) -> SqlitePool {
    let mut db_path = app_data_dir.clone();
    db_path.push(format!("{}.db", pkg_name));

    let opts: SqliteConnectOptions = SqliteConnectOptions::from_str(db_path.to_str().unwrap())
        .unwrap()
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .foreign_keys(true);

    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(4)
        .acquire_timeout(std::time::Duration::from_secs(10))
        .after_connect(|conn, _meta| {
            Box::pin(async move {
                sqlx::query("PRAGMA busy_timeout = 5000;")
                    .execute(&mut *conn)
                    .await?;
                sqlx::query("PRAGMA temp_store = MEMORY;")
                    .execute(&mut *conn)
                    .await?;
                sqlx::query("PRAGMA cache_size = -40000;")
                    .execute(&mut *conn)
                    .await?;
                sqlx::query("PRAGMA wal_autocheckpoint = 1000;")
                    .execute(&mut *conn)
                    .await?;
                sqlx::query("PRAGMA mmap_size = 268435456;")
                    .execute(&mut *conn)
                    .await?;
                Ok::<_, sqlx::Error>(())
            })
        })
        .connect_with(opts)
        .await
        .expect("failed to connect sqlite");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrations");
    let _ = sqlx::query("PRAGMA optimize;").execute(&pool).await;

    pool
}

pub struct SqlitePhotoRepository {
    pool: SqlitePool,
}

impl SqlitePhotoRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

impl PhotoRepository for SqlitePhotoRepository {
    fn get_cached_photos_for_path<'a>(
        &'a self,
        prefix: &'a str,
    ) -> BoxFuture<'a, anyhow::Result<Vec<(String, String, i64, i64)>>> {
        Box::pin(async move {
            let rows = sqlx::query(
                "SELECT path, thumbnail_path, mtime, size
                 FROM photos
                 WHERE path LIKE ?1 || '%'",
            )
            .bind(prefix)
            .fetch_all(&self.pool)
            .await?;

            let result = rows
                .into_iter()
                .map(|row| {
                    (
                        row.get("path"),
                        row.get("thumbnail_path"),
                        row.get("mtime"),
                        row.get("size"),
                    )
                })
                .collect();

            Ok(result)
        })
    }

    fn batch_insert_photos<'a>(
        &'a self,
        photos: &'a [PhotoMetadata],
    ) -> BoxFuture<'a, anyhow::Result<()>> {
        Box::pin(async move {
            if photos.is_empty() {
                return Ok(());
            }

            let mut tx = self.pool.begin().await?;

            const CHUNK: usize = 1_000;
            for chunk in photos.chunks(CHUNK) {
                let mut qb: QueryBuilder<Sqlite> = QueryBuilder::new(
                    "INSERT INTO photos (path, name, thumbnail_path, mtime, ctime, size) ",
                );
                qb.push_values(chunk, |mut b, p| {
                    b.push_bind(&p.path)
                        .push_bind(&p.metadata.name)
                        .push_bind(&p.thumbnail_path)
                        .push_bind(p.metadata.modified as i64)
                        .push_bind(p.metadata.created as i64)
                        .push_bind(p.metadata.size as i64);
                });
                qb.push(
                    " ON CONFLICT(path) DO UPDATE SET
                        name=excluded.name,
                        thumbnail_path=excluded.thumbnail_path,
                        mtime=excluded.mtime,
                        ctime=excluded.ctime,
                        size=excluded.size
                      WHERE photos.mtime <> excluded.mtime
                         OR photos.size  <> excluded.size
                         OR photos.thumbnail_path <> excluded.thumbnail_path
                         OR photos.name  <> excluded.name",
                );

                qb.build().execute(&mut *tx).await?;
            }

            tx.commit().await?;
            Ok(())
        })
    }
}

pub struct SqliteFavouriteRepository {
    pool: SqlitePool,
}

impl SqliteFavouriteRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

impl FavouriteRepository for SqliteFavouriteRepository {
    fn add_favourite(&self, path: String) -> BoxFuture<'_, anyhow::Result<()>> {
        Box::pin(async move {
            sqlx::query("INSERT INTO favourites (path) VALUES (?1) ON CONFLICT(path) DO NOTHING")
                .bind(path)
                .execute(&self.pool)
                .await?;
            Ok(())
        })
    }

    fn get_favourites(&self) -> BoxFuture<'_, anyhow::Result<Vec<Favourite>>> {
        Box::pin(async move {
            let rows = sqlx::query("SELECT path FROM favourites")
                .fetch_all(&self.pool)
                .await?;

            let favourites = rows
                .into_iter()
                .map(|row| Favourite {
                    path: row.get("path"),
                })
                .collect();

            Ok(favourites)
        })
    }

    fn remove_favourite(&self, path: String) -> BoxFuture<'_, anyhow::Result<()>> {
        Box::pin(async move {
            sqlx::query("DELETE FROM favourites WHERE path = ?1")
                .bind(path)
                .execute(&self.pool)
                .await?;
            Ok(())
        })
    }

    fn clear_favourites(&self) -> BoxFuture<'_, anyhow::Result<()>> {
        Box::pin(async move {
            sqlx::query("DELETE FROM favourites")
                .execute(&self.pool)
                .await?;
            Ok(())
        })
    }
}
