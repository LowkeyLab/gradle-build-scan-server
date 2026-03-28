CREATE TABLE IF NOT EXISTS task_cache_operations (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL REFERENCES tasks(id),
    operation_type TEXT NOT NULL,
    succeeded INTEGER NOT NULL,
    archive_size INTEGER,
    cache_key TEXT
);

CREATE INDEX IF NOT EXISTS idx_task_cache_operations_task_id ON task_cache_operations(task_id);
