CREATE TABLE IF NOT EXISTS workspaces (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    root_path TEXT UNIQUE NOT NULL,
    created_at INTEGER DEFAULT (unixepoch()),
    last_opened_at INTEGER DEFAULT (unixepoch())
);

CREATE INDEX IF NOT EXISTS idx_workspaces_root_path ON workspaces (root_path);
