use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub payload_dir: PathBuf,
    pub upstream_url: String,
}

impl Config {
    pub fn from_env() -> Self {
        let upstream_url =
            std::env::var("UPSTREAM_URL").expect("UPSTREAM_URL environment variable is required");

        let upstream_url = upstream_url.trim_end_matches('/').to_string();

        Self {
            port: std::env::var("PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(8080),
            payload_dir: std::env::var("PAYLOAD_DIR")
                .ok()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("/tmp/gradle-payloads")),
            upstream_url,
        }
    }
}
