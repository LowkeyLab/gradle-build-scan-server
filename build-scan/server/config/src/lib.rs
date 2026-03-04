use std::path::PathBuf;

use runfiles::{Runfiles, rlocation};

#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub db_url: String,
    pub spa_dir: Option<PathBuf>,
    pub base_url: String,
}

/// The known runfiles path where Bazel places the Angular build output.
const RUNFILES_SPA_PATH: &str = "_main/angular/projects/build-scan-web/dist/browser";

/// Resolve the SPA directory for serving the Angular frontend.
///
/// Priority:
/// 1. `SPA_DIR` env var — explicit override
/// 2. Bazel runfiles — auto-discovered when running via `bazel run`
///
/// Returns `None` if neither source is available.
fn resolve_spa_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("SPA_DIR") {
        return Some(PathBuf::from(dir));
    }

    let r = Runfiles::create().ok()?;
    let path = rlocation!(r, RUNFILES_SPA_PATH)?;
    if path.is_dir() {
        return Some(path);
    }

    None
}

impl Config {
    pub fn from_env() -> Self {
        let port: u16 = std::env::var("PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(3000);
        Self {
            port,
            db_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:build-scans.db?mode=rwc".to_string()),
            spa_dir: resolve_spa_dir(),
            base_url: std::env::var("BASE_URL")
                .unwrap_or_else(|_| format!("http://localhost:{}", port)),
        }
    }
}
