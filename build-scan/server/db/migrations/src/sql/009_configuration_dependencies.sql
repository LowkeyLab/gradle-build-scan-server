CREATE TABLE IF NOT EXISTS configuration_dependencies (
    scan_id TEXT NOT NULL REFERENCES build_scans(id),
    configuration_id INTEGER NOT NULL,
    display_name TEXT NOT NULL,
    details_json TEXT NOT NULL,
    started_labels_json TEXT NOT NULL,
    finished_labels_json TEXT NOT NULL,
    PRIMARY KEY (scan_id, configuration_id)
);

CREATE TABLE IF NOT EXISTS configuration_dependency_artifact_labels (
    scan_id TEXT NOT NULL REFERENCES build_scans(id),
    ordinal INTEGER NOT NULL,
    label TEXT NOT NULL,
    PRIMARY KEY (scan_id, ordinal)
);
