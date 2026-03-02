DROP INDEX IF EXISTS idx_build_scans_created_at;
CREATE INDEX IF NOT EXISTS idx_build_scans_created_at_id ON build_scans(created_at, id);
