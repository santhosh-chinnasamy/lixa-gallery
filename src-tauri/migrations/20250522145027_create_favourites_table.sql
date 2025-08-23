CREATE TABLE IF NOT EXISTS favourites (
    path TEXT PRIMARY KEY NOT NULL
);

CREATE INDEX IF NOT EXISTS ix_favourites_path ON favourites (path);