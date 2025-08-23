CREATE TABLE
    IF NOT EXISTS photos (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        path TEXT NOT NULL,
        name TEXT NOT NULL,
        thumbnail_path TEXT NOT NULL,
        mtime INTEGER NOT NULL, -- file modification time
        ctime INTEGER NOT NULL, -- file creation time
        size INTEGER NOT NULL, -- file size in bytes
        created_at INTEGER DEFAULT (unixepoch ()) -- time when the photo was added to Lixa
    );

CREATE UNIQUE INDEX IF NOT EXISTS idx_photos_path ON photos (path);

CREATE INDEX IF NOT EXISTS idx_photos_size ON photos (size);

CREATE INDEX IF NOT EXISTS idx_photos_mtime ON photos (mtime);

CREATE INDEX IF NOT EXISTS idx_photos_created_at ON photos (created_at);