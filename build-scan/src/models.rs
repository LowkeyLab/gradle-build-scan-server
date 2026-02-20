use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct BuildScanPayload {
    pub tasks: Vec<TaskExecution>,
    // Expand as needed
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskExecution {
    pub task_path: String,
    // Add fields as discovered by heuristics
}
