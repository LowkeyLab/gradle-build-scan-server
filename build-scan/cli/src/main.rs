use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use base64::Engine as _;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "build-scan-cli")]
#[command(about = "Parse Gradle build scan payloads captured by the echo-server")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Parse an echo-server payload file and extract the build scan data
    Parse {
        /// Path to the input echo-server JSON payload file
        #[arg(short, long)]
        input: PathBuf,

        /// Path to write the parsed build scan JSON output
        #[arg(short, long)]
        output: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Parse { input, output } => run_parse(&input, &output),
    }
}

fn run_parse(input: &Path, output: &Path) -> Result<()> {
    // 1. Read the input file
    let contents = std::fs::read_to_string(input)
        .with_context(|| format!("Failed to read input file: {}", input.display()))?;

    // 2. Deserialize into echo-server Payload
    let payload: format::Payload = serde_json::from_str(&contents)
        .context("Failed to parse input as echo-server Payload JSON")?;

    // 3. Extract base64 body
    let b64_str = payload
        .request
        .body
        .get("base64")
        .and_then(|v| v.as_str())
        .context("Payload request body does not contain a \"base64\" string field")?;

    // 4. Decode base64 — these are the full raw bytes (outer header + gzip payload)
    let raw_bytes = base64::engine::general_purpose::STANDARD
        .decode(b64_str)
        .context("Failed to decode base64 body")?;

    // 5. Parse build scan (handles outer header + decompression + framing + decode internally)
    let build_scan = lib::parse(&raw_bytes).context("Failed to parse build scan payload")?;

    // 6. Serialize to JSON
    let json_output =
        serde_json::to_string_pretty(&build_scan).context("Failed to serialize build scan")?;

    // 7. Write output
    std::fs::write(output, &json_output)
        .with_context(|| format!("Failed to write output file: {}", output.display()))?;

    println!("Parsed build scan written to {}", output.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: return a unique temp file path.
    fn temp_path(name: &str) -> std::path::PathBuf {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("cli_test_{name}_{ts}"))
    }

    #[test]
    fn error_when_base64_field_missing() {
        let payload = serde_json::json!({
            "request_id": "test-002",
            "timestamp": "2025-01-01T00:00:00Z",
            "request": {
                "method": "POST",
                "uri": "/scan",
                "headers": [],
                "body": "just a plain string, not an object"
            },
            "response": {
                "status": 200
            }
        });

        let input_path = temp_path("missing_b64_in.json");
        let output_path = temp_path("missing_b64_out.json");

        std::fs::write(&input_path, serde_json::to_string_pretty(&payload).unwrap()).unwrap();

        let result = run_parse(&input_path, &output_path);
        assert!(result.is_err());

        let err_msg = format!("{:#}", result.unwrap_err());
        assert!(
            err_msg.contains("base64"),
            "error should mention 'base64', got: {err_msg}"
        );

        // Output should NOT have been created
        assert!(
            !output_path.exists(),
            "output file should not exist on error"
        );

        // Cleanup
        let _ = std::fs::remove_file(&input_path);
    }

    #[test]
    fn error_when_input_file_missing() {
        let input_path = temp_path("nonexistent_input.json");
        let output_path = temp_path("nonexistent_output.json");

        // Ensure input does not exist
        let _ = std::fs::remove_file(&input_path);

        let result = run_parse(&input_path, &output_path);
        assert!(result.is_err());

        let err_msg = format!("{:#}", result.unwrap_err());
        assert!(
            err_msg.contains("Failed to read input file"),
            "error should mention reading input file, got: {err_msg}"
        );
    }
}
