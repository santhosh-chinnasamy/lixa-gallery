-- BEGIN;
CREATE TABLE IF NOT EXISTS favourites (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    path TEXT NOT NULL
);

-- CREATE INDEX IF NOT EXISTS ix_favourites_path ON favourites (path);
-- COMMIT;