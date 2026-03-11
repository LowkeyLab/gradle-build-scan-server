CREATE TABLE IF NOT EXISTS payloads (
    id TEXT PRIMARY KEY,
    request_id TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    method TEXT NOT NULL,
    uri TEXT NOT NULL,
    request_headers TEXT NOT NULL,
    request_body TEXT,
    response_status INTEGER,
    response_headers TEXT,
    response_body TEXT,
    response_error TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
