-- EXIF / RAW preview columns on photos. Safe to re-run after cull_flags
-- (20261101160000). SQLite ignores ADD COLUMN when the name already exists
-- only if we gate it; sqlx records this version once, so plain ALTER is enough
-- on a normal upgrade path.
ALTER TABLE photos ADD COLUMN exif_json TEXT;

ALTER TABLE photos ADD COLUMN preview_path TEXT;

ALTER TABLE photos ADD COLUMN sidecar_path TEXT;
