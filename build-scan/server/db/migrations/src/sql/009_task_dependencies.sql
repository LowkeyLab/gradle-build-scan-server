CREATE TABLE IF NOT EXISTS task_dependencies (
    task_id TEXT NOT NULL REFERENCES tasks(id),
    dependency_task_id TEXT NOT NULL REFERENCES tasks(id),
    PRIMARY KEY (task_id, dependency_task_id)
);

CREATE INDEX IF NOT EXISTS idx_task_dependencies_task_id
    ON task_dependencies(task_id);
