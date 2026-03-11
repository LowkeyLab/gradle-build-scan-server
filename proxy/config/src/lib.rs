use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub upstream_url: String,
    pub database_url: String,
}

impl Config {
    pub fn from_env() -> Self {
        let port = env::var("PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(8080);

        let upstream_url =
            env::var("UPSTREAM_URL").expect("UPSTREAM_URL environment variable is required");
        let upstream_url = upstream_url.trim_end_matches('/').to_string();

        let database_url =
            env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:proxy.db".to_string());

        Config {
            port,
            upstream_url,
            database_url,
        }
    }
}
