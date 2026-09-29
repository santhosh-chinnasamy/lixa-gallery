use async_trait::async_trait;
use gallery_core::models::{Favourite, FileMetadata, PhotoMetadata, Result, Workspace};
use gallery_core::repos::{
    CachedPhotoRecord, FavouriteRepository, PhotoRepository, WorkspaceRepository,
};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqliteSynchronous},
    QueryBuilder, Row, Sqlite, SqlitePool,
};
use std::{path::PathBuf, str::FromStr};

pub async fn setup_db(app_data_dir: PathBuf, pkg_name: &str) -> SqlitePool {
    let mut db_path = app_data_dir.clone();
    db_path.push(format!("{}.db", pkg_name));

    let opts: SqliteConnectOptions =
        SqliteConnectOptions::from_str(db_path.to_str().expect("valid path"))
            .expect("valid connection options")
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

    sqlx::migrate!().run(&pool).await.expect("migrations");
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

#[async_trait]
impl PhotoRepository for SqlitePhotoRepository {
    async fn get_cached_photos_for_path(&self, prefix: &str) -> Result<Vec<CachedPhotoRecord>> {
        let rows = sqlx::query(
            "SELECT path, thumbnail_path, mtime, size
             FROM photos
             WHERE path LIKE ?1 || '%'",
        )
        .bind(prefix)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| gallery_core::models::GalleryError::Db(e.to_string()))?;

        let result = rows
            .into_iter()
            .map(|row| CachedPhotoRecord {
                path: row.get("path"),
                thumbnail_path: row.get("thumbnail_path"),
                mtime: row.get("mtime"),
                size: row.get("size"),
            })
            .collect();

        Ok(result)
    }

    async fn get_photos_by_paths(&self, paths: &[String]) -> Result<Vec<PhotoMetadata>> {
        if paths.is_empty() {
            return Ok(Vec::new());
        }

        let mut results = Vec::new();
        const CHUNK: usize = 500;
        for chunk in paths.chunks(CHUNK) {
            let mut qb: QueryBuilder<Sqlite> = QueryBuilder::new(
                "SELECT path, name, thumbnail_path, mtime, ctime, size FROM photos WHERE path IN (",
            );
            let mut separated = qb.separated(", ");
            for p in chunk {
                separated.push_bind(p);
            }
            separated.push_unseparated(")");

            let rows = qb
                .build()
                .fetch_all(&self.pool)
                .await
                .map_err(|e| gallery_core::models::GalleryError::Db(e.to_string()))?;

            for row in rows {
                results.push(PhotoMetadata {
                    metadata: FileMetadata {
                        name: row.get("name"),
                        modified: row.get::<i64, _>("mtime") as u64,
                        created: row.get::<i64, _>("ctime") as u64,
                        size: row.get::<i64, _>("size") as u64,
                    },
                    thumbnail_path: row.get("thumbnail_path"),
                    path: row.get("path"),
                });
            }
        }

        Ok(results)
    }

    async fn batch_insert_photos(&self, photos: &[PhotoMetadata]) -> Result<()> {
        if photos.is_empty() {
            return Ok(());
        }

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| gallery_core::models::GalleryError::Db(e.to_string()))?;

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

            qb.build()
                .execute(&mut *tx)
                .await
                .map_err(|e| gallery_core::models::GalleryError::Db(e.to_string()))?;
        }

        tx.commit()
            .await
            .map_err(|e| gallery_core::models::GalleryError::Db(e.to_string()))?;
        Ok(())
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

#[async_trait]
impl FavouriteRepository for SqliteFavouriteRepository {
    async fn add_favourite(&self, path: String) -> Result<()> {
        sqlx::query("INSERT INTO favourites (path) VALUES (?1) ON CONFLICT(path) DO NOTHING")
            .bind(path)
            .execute(&self.pool)
            .await
            .map_err(|e| gallery_core::models::GalleryError::Db(e.to_string()))?;
        Ok(())
    }

    async fn get_favourites(&self) -> Result<Vec<Favourite>> {
        let rows = sqlx::query("SELECT path FROM favourites")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| gallery_core::models::GalleryError::Db(e.to_string()))?;

        let favourites = rows
            .into_iter()
            .map(|row| Favourite {
                path: row.get("path"),
            })
            .collect();

        Ok(favourites)
    }

    async fn remove_favourite(&self, path: String) -> Result<()> {
        sqlx::query("DELETE FROM favourites WHERE path = ?1")
            .bind(path)
            .execute(&self.pool)
            .await
            .map_err(|e| gallery_core::models::GalleryError::Db(e.to_string()))?;
        Ok(())
    }

    async fn get_favourites_by_prefix(&self, prefix: &str) -> Result<Vec<Favourite>> {
        let rows = sqlx::query("SELECT path FROM favourites WHERE path LIKE ?1 || '%'")
            .bind(prefix)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| gallery_core::models::GalleryError::Db(e.to_string()))?;

        let favourites = rows
            .into_iter()
            .map(|row| Favourite {
                path: row.get("path"),
            })
            .collect();

        Ok(favourites)
    }

    async fn clear_favourites(&self) -> Result<()> {
        sqlx::query("DELETE FROM favourites")
            .execute(&self.pool)
            .await
            .map_err(|e| gallery_core::models::GalleryError::Db(e.to_string()))?;
        Ok(())
    }

    async fn clear_favourites_by_prefix(&self, prefix: &str) -> Result<()> {
        sqlx::query("DELETE FROM favourites WHERE path LIKE ?1 || '%'")
            .bind(prefix)
            .execute(&self.pool)
            .await
            .map_err(|e| gallery_core::models::GalleryError::Db(e.to_string()))?;
        Ok(())
    }
}

pub struct SqliteWorkspaceRepository {
    pool: SqlitePool,
}

impl SqliteWorkspaceRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl WorkspaceRepository for SqliteWorkspaceRepository {
    async fn upsert_workspace(&self, workspace: &Workspace) -> Result<()> {
        sqlx::query(
            "INSERT INTO workspaces (id, name, root_path, created_at, last_opened_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(root_path) DO UPDATE SET
                name = excluded.name,
                last_opened_at = excluded.last_opened_at",
        )
        .bind(&workspace.id)
        .bind(&workspace.name)
        .bind(&workspace.root_path)
        .bind(workspace.created_at)
        .bind(workspace.last_opened_at)
        .execute(&self.pool)
        .await
        .map_err(|e| gallery_core::models::GalleryError::Db(e.to_string()))?;

        Ok(())
    }

    async fn get_workspaces(&self) -> Result<Vec<Workspace>> {
        let rows = sqlx::query(
            "SELECT id, name, root_path, created_at, last_opened_at
             FROM workspaces
             ORDER BY last_opened_at DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| gallery_core::models::GalleryError::Db(e.to_string()))?;

        let workspaces = rows
            .into_iter()
            .map(|row| Workspace {
                id: row.get("id"),
                name: row.get("name"),
                root_path: row.get("root_path"),
                created_at: row.get("created_at"),
                last_opened_at: row.get("last_opened_at"),
            })
            .collect();

        Ok(workspaces)
    }

    async fn get_workspace_by_path(&self, root_path: &str) -> Result<Option<Workspace>> {
        let row = sqlx::query(
            "SELECT id, name, root_path, created_at, last_opened_at
             FROM workspaces
             WHERE root_path = ?1",
        )
        .bind(root_path)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| gallery_core::models::GalleryError::Db(e.to_string()))?;

        Ok(row.map(|r| Workspace {
            id: r.get("id"),
            name: r.get("name"),
            root_path: r.get("root_path"),
            created_at: r.get("created_at"),
            last_opened_at: r.get("last_opened_at"),
        }))
    }

    async fn remove_workspace(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM workspaces WHERE id = ?1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| gallery_core::models::GalleryError::Db(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gallery_core::models::FileMetadata;

    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!().run(&pool).await.expect("migrations");
        pool
    }

    #[tokio::test]
    async fn test_favourite_repo() {
        let pool = setup_test_db().await;
        let repo = SqliteFavouriteRepository::new(pool);

        repo.add_favourite("test/path.jpg".to_string())
            .await
            .unwrap();
        let favs = repo.get_favourites().await.unwrap();
        assert_eq!(favs.len(), 1);
        assert_eq!(favs[0].path, "test/path.jpg");

        repo.remove_favourite("test/path.jpg".to_string())
            .await
            .unwrap();
        let favs = repo.get_favourites().await.unwrap();
        assert_eq!(favs.len(), 0);
    }

    #[tokio::test]
    async fn test_photo_repo() {
        let pool = setup_test_db().await;
        let repo = SqlitePhotoRepository::new(pool);

        let photos = vec![PhotoMetadata {
            metadata: FileMetadata {
                name: "img.jpg".to_string(),
                modified: 100,
                created: 100,
                size: 500,
            },
            thumbnail_path: "thumb".to_string(),
            path: "img.jpg".to_string(),
        }];

        repo.batch_insert_photos(&photos).await.unwrap();

        let cached = repo.get_cached_photos_for_path("").await.unwrap();
        assert_eq!(cached.len(), 1);
        assert_eq!(cached[0].path, "img.jpg");

        let fetched = repo
            .get_photos_by_paths(&["img.jpg".to_string()])
            .await
            .unwrap();
        assert_eq!(fetched.len(), 1);
        assert_eq!(fetched[0].metadata.name, "img.jpg");
    }

    #[tokio::test]
    async fn test_favourite_repo_prefix() {
        let pool = setup_test_db().await;
        let repo = SqliteFavouriteRepository::new(pool);

        repo.add_favourite("/a/1.jpg".to_string()).await.unwrap();
        repo.add_favourite("/a/2.jpg".to_string()).await.unwrap();
        repo.add_favourite("/b/1.jpg".to_string()).await.unwrap();

        let a_favs = repo.get_favourites_by_prefix("/a/").await.unwrap();
        assert_eq!(a_favs.len(), 2);

        repo.clear_favourites_by_prefix("/a/").await.unwrap();
        let remaining = repo.get_favourites().await.unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].path, "/b/1.jpg");
    }

    #[tokio::test]
    async fn test_workspace_repo() {
        let pool = setup_test_db().await;
        let repo = SqliteWorkspaceRepository::new(pool);

        let ws = Workspace {
            id: "ws-1".to_string(),
            name: "Folder A".to_string(),
            root_path: "/photos/a".to_string(),
            created_at: 1000,
            last_opened_at: 1000,
        };

        repo.upsert_workspace(&ws).await.unwrap();
        let workspaces = repo.get_workspaces().await.unwrap();
        assert_eq!(workspaces.len(), 1);
        assert_eq!(workspaces[0].name, "Folder A");

        let fetched = repo.get_workspace_by_path("/photos/a").await.unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().id, "ws-1");

        repo.remove_workspace("ws-1").await.unwrap();
        let workspaces_after = repo.get_workspaces().await.unwrap();
        assert_eq!(workspaces_after.len(), 0);
    }
}
