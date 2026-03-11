use std::path::PathBuf;

use runfiles::{Runfiles, rlocation};
use tracing::{info, warn};

#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub db_url: String,
    pub spa_dir: Option<PathBuf>,
    pub base_url: String,
}

/// The known runfiles path where Bazel places the Angular build output.
const RUNFILES_SPA_PATH: &str = "_main/angular/projects/build-scan-web/dist";

/// Resolve the SPA directory for serving the Angular frontend.
///
/// Priority:
/// 1. `SPA_DIR` env var — explicit override
/// 2. Bazel runfiles — auto-discovered when running via `bazel run`
///
/// Returns `None` if neither source is available.
fn resolve_spa_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("SPA_DIR") {
        let path = PathBuf::from(dir);
        if !path.is_dir() {
            warn!(path = %path.display(), "SPA_DIR is not a directory; falling back to runfiles");
        } else if !is_spa_dir(&path) {
            warn!(path = %path.display(), "SPA_DIR does not contain index.html; falling back to runfiles");
        } else {
            info!(path = %path.display(), "Serving SPA from SPA_DIR");
            return Some(path);
        }
    }

    if let Some(path) = resolve_from_runfiles() {
        info!(path = %path.display(), "Serving SPA from Bazel runfiles");
        return Some(path);
    }

    warn!("SPA directory not found; /web/ will not be available");
    None
}

fn resolve_from_runfiles() -> Option<PathBuf> {
    let r = Runfiles::create().ok()?;
    let dist_path = rlocation!(r, RUNFILES_SPA_PATH)?;
    let path = dist_path.join("browser");
    is_spa_dir(&path).then_some(path)
}

fn is_spa_dir(path: &PathBuf) -> bool {
    path.join("index.html").is_file()
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
