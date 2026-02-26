use base64::Engine;
use tracing::info;

fn find_reference_payload() -> Option<std::path::PathBuf> {
    const FILENAME: &str = "20260222_115121.815-7df62a0f-bf22-4eb7-9fdc-84c238df73c6.json";

    // When run under Bazel, data files live under:
    //   $TEST_SRCDIR/_main/captured-output/payloads/<file>
    if let Ok(srcdir) = std::env::var("TEST_SRCDIR") {
        let bazel_path = std::path::Path::new(&srcdir)
            .join("_main")
            .join("captured-output")
            .join("payloads")
            .join(FILENAME);
        if bazel_path.exists() {
            return Some(bazel_path);
        }
    }

    // Fallback: workspace-relative path (useful when running with `cargo test`)
    let workspace_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .find_map(|ancestor| {
            let candidate = ancestor
                .join("captured-output")
                .join("payloads")
                .join(FILENAME);
            if candidate.exists() {
                Some(candidate)
            } else {
                None
            }
        });
    workspace_path
}

#[test]
fn test_parse_reference_payload() {
    let payload_path = match find_reference_payload() {
        Some(p) => p,
        None => {
            info!(
                "Skipping integration test: reference payload not found. \
                 Run under Bazel with data dependency or set TEST_SRCDIR."
            );
            return;
        }
    };

    info!("Loading reference payload from: {}", payload_path.display());

    let contents = std::fs::read_to_string(&payload_path)
        .unwrap_or_else(|e| panic!("Failed to read payload file: {e}"));

    let json: serde_json::Value =
        serde_json::from_str(&contents).expect("Reference payload must be valid JSON");

    let b64 = json["request"]["body"]["base64"]
        .as_str()
        .expect("JSON must have request.body.base64 string");

    let raw_bytes = base64::engine::general_purpose::STANDARD
        .decode(b64)
        .expect("base64 must decode cleanly");

    info!("Decoded {} raw bytes", raw_bytes.len());

    let result = lib::parse(&raw_bytes).expect("Parser must succeed on reference payload");

    info!(
        "Parsed {} tasks, {} raw event types",
        result.tasks.len(),
        result.raw_events.len()
    );

    for task in &result.tasks {
        info!(
            "  {} ({}) duration={}ms",
            task.task_path,
            task.outcome
                .as_ref()
                .map(|o| format!("{o:?}"))
                .unwrap_or_default(),
            task.duration_ms.unwrap_or(0)
        );
    }

    // Reference payload should have ~45 tasks (from analysis: "45x TASK_STARTED_v6")
    assert!(
        result.tasks.len() > 40,
        "expected ~45 tasks, got {}",
        result.tasks.len()
    );

    // Check known task paths from the reference build
    let paths: Vec<&str> = result.tasks.iter().map(|t| t.task_path.as_str()).collect();
    assert!(
        paths.contains(&":app:compileKotlin"),
        "missing :app:compileKotlin; found paths: {paths:?}"
    );
    assert!(
        paths.contains(&":app:build"),
        "missing :app:build; found paths: {paths:?}"
    );

    // All tasks should have timing info
    for task in &result.tasks {
        assert!(
            task.started_at.is_some(),
            "task {} missing started_at",
            task.task_path
        );
        assert!(
            task.finished_at.is_some(),
            "task {} missing finished_at",
            task.task_path
        );
        assert!(
            task.duration_ms.is_some(),
            "task {} missing duration_ms",
            task.task_path
        );
    }

    // Raw events should be populated
    assert!(
        !result.raw_events.is_empty(),
        "raw_events must not be empty"
    );
}
