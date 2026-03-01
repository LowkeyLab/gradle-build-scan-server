CREATE TABLE IF NOT EXISTS build_scans (
    id TEXT PRIMARY KEY,
    build_tool_type TEXT NOT NULL,
    build_tool_version TEXT NOT NULL,
    plugin_version TEXT NOT NULL,
    started_at TEXT,
    finished_at TEXT,
    outcome TEXT,
    requested_tasks TEXT,
    hostname TEXT,
    os_name TEXT,
    os_version TEXT,
    jvm_vendor TEXT,
    jvm_version TEXT,
    raw_payload BLOB,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS tasks (
    id TEXT PRIMARY KEY,
    scan_id TEXT NOT NULL REFERENCES build_scans(id),
    task_path TEXT NOT NULL,
    class_name TEXT,
    outcome TEXT,
    cacheable INTEGER,
    start_timestamp INTEGER,
    finish_timestamp INTEGER,
    cache_key TEXT,
    origin_execution_time INTEGER
);

CREATE INDEX IF NOT EXISTS idx_tasks_scan_id ON tasks(scan_id);
CREATE INDEX IF NOT EXISTS idx_build_scans_created_at ON build_scans(created_at);
