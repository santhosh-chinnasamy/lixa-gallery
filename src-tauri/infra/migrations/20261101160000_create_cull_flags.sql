-- Cull flags use the same shape as favourites: one row per path.
-- No row means unflagged. The flag column is picked or rejected, so a photo
-- cannot be both. Existing favourites become picked.
CREATE TABLE IF NOT EXISTS cull_flags (
    path TEXT PRIMARY KEY NOT NULL,
    flag TEXT NOT NULL CHECK (flag IN ('picked', 'rejected')),
    updated_at INTEGER NOT NULL DEFAULT 0
);

INSERT INTO cull_flags (path, flag, updated_at)
SELECT f.path, 'picked', CAST(strftime('%s', 'now') AS INTEGER)
FROM favourites AS f
WHERE NOT EXISTS (
    SELECT 1 FROM cull_flags AS c WHERE c.path = f.path
);

CREATE INDEX IF NOT EXISTS ix_cull_flags_flag ON cull_flags (flag);
