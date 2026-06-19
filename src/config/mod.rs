use std::env;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppConfig {
    pub host: IpAddr,
    pub port: u16,
    pub database_url: Option<String>,
    pub jwt_secret: String,
    pub run_migrations: bool,
    pub files_backend: String,
    pub files_bucket: String,
    pub files_local_dir: String,
    pub files_public_base_url: String,
    pub files_max_byte_size: u64,
    pub s3_region: Option<String>,
    pub s3_endpoint: Option<String>,
    pub jobs_enabled: bool,
    pub jobs_reminder_interval_seconds: u64,
    pub jobs_reminder_window_minutes: u32,
    pub jobs_dispatch_interval_seconds: u64,
    pub jobs_live_interval_seconds: u64,
}

impl AppConfig {
    pub fn from_env() -> Self {
        let host = env::var("HTTP_HOST")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST));
        let port = env::var("HTTP_PORT")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(3001);
        let database_url = env::var("DATABASE_URL").ok();
        let jwt_secret =
            env::var("JWT_SECRET").unwrap_or_else(|_| "development-only-secret".to_owned());
        let run_migrations = env::var("RUN_MIGRATIONS")
            .map(|value| !matches!(value.as_str(), "0" | "false" | "FALSE" | "no" | "NO"))
            .unwrap_or(true);
        let files_backend =
            env::var("FILES_BACKEND").unwrap_or_else(|_| "local".to_owned());
        let files_bucket =
            env::var("FILES_BUCKET").unwrap_or_else(|_| "palpites-local".to_owned());
        let files_local_dir =
            env::var("FILES_LOCAL_DIR").unwrap_or_else(|_| "./storage".to_owned());
        let files_public_base_url = env::var("FILES_PUBLIC_BASE_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:3001/files-local".to_owned());
        let files_max_byte_size = env::var("FILES_MAX_BYTE_SIZE")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(5 * 1024 * 1024);
        let s3_region = env::var("S3_REGION").ok();
        let s3_endpoint = env::var("S3_ENDPOINT").ok();
        let jobs_enabled = env::var("JOBS_ENABLED")
            .map(|value| !matches!(value.as_str(), "0" | "false" | "FALSE" | "no" | "NO"))
            .unwrap_or(true);
        let jobs_reminder_interval_seconds = env::var("JOBS_REMINDER_INTERVAL_SECONDS")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(300);
        let jobs_reminder_window_minutes = env::var("JOBS_REMINDER_WINDOW_MINUTES")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(60);
        let jobs_dispatch_interval_seconds = env::var("JOBS_DISPATCH_INTERVAL_SECONDS")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(60);
        let jobs_live_interval_seconds = env::var("JOBS_LIVE_INTERVAL_SECONDS")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(60);

        Self {
            host,
            port,
            database_url,
            jwt_secret,
            run_migrations,
            files_backend,
            files_bucket,
            files_local_dir,
            files_public_base_url,
            files_max_byte_size,
            s3_region,
            s3_endpoint,
            jobs_enabled,
            jobs_reminder_interval_seconds,
            jobs_reminder_window_minutes,
            jobs_dispatch_interval_seconds,
            jobs_live_interval_seconds,
        }
    }

    pub fn http_addr(&self) -> SocketAddr {
        SocketAddr::new(self.host, self.port)
    }
}
