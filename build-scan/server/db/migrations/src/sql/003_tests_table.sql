CREATE TABLE IF NOT EXISTS tests (
    id TEXT PRIMARY KEY,
    scan_id TEXT NOT NULL REFERENCES build_scans(id),
    class_name TEXT NOT NULL,
    method_name TEXT,
    executor_name TEXT,
    outcome TEXT
);

CREATE INDEX IF NOT EXISTS idx_tests_scan_id ON tests(scan_id);
