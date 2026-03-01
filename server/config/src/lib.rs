use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub db_url: String,
    pub spa_dir: Option<PathBuf>,
    pub base_url: String,
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
            spa_dir: std::env::var("SPA_DIR").ok().map(PathBuf::from),
            base_url: std::env::var("BASE_URL")
                .unwrap_or_else(|_| format!("http://localhost:{}", port)),
        }
    }
}
