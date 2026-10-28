//! Startup configuration with redacted secrets.
use crate::{
    contracts::utf16_len,
    error::{AppError, ErrorCode},
};
use std::{
    collections::HashMap,
    fmt,
    net::{IpAddr, SocketAddr},
    path::{Path, PathBuf},
    time::Duration,
};

#[derive(Clone, Default)]
pub struct Secret(String);
impl Secret {
    pub fn expose(&self) -> &str {
        &self.0
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<redacted>")
    }
}

#[derive(Debug, Clone)]
pub struct Limits {
    pub body_bytes: usize,
    pub body_read: Duration,
    pub concurrent: usize,
    pub per_minute: usize,
    pub research: Duration,
    pub search: Duration,
    pub connect: Duration,
    pub provider_bytes: usize,
    pub connections: usize,
    pub waiters: usize,
    pub headers: Duration,
    pub shutdown_grace: Duration,
    pub shutdown_total: Duration,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            body_bytes: 65536,
            body_read: Duration::from_secs(10),
            concurrent: 2,
            per_minute: 10,
            research: Duration::from_secs(120),
            search: Duration::from_secs(20),
            connect: Duration::from_secs(5),
            provider_bytes: 2 * 1024 * 1024,
            connections: 128,
            waiters: 32,
            headers: Duration::from_secs(10),
            shutdown_grace: Duration::from_secs(125),
            shutdown_total: Duration::from_secs(130),
        }
    }
}
#[derive(Debug, Clone)]
pub struct Config {
    pub storage: crate::storage::StorageConfig,
    pub origin: url::Url,
    pub bind: SocketAddr,
    pub access_key: Secret,
    pub openai_key: Secret,
    pub tavily_key: Secret,
    pub model: String,
    pub asset_dir: PathBuf,
    pub log_filter: String,
    pub limits: Limits,
}
impl Config {
    pub fn from_values(
        values: &HashMap<String, String>,
        executable: &Path,
    ) -> Result<Self, AppError> {
        let invalid = || AppError::new(ErrorCode::InvalidConfig);
        let origin = url::Url::parse(values.get("APP_ORIGIN").ok_or_else(invalid)?)
            .map_err(|_| invalid())?;
        let loopback_origin =
            matches!(origin.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
        if !matches!(origin.scheme(), "https" | "http")
            || (origin.scheme() == "http" && !loopback_origin)
            || origin.host_str().is_none()
            || !origin.username().is_empty()
            || origin.password().is_some()
            || origin.query().is_some()
            || origin.fragment().is_some()
            || origin.path() != "/"
        {
            return Err(invalid());
        }
        let host = values
            .get("HOST")
            .map(String::as_str)
            .unwrap_or("127.0.0.1");
        let ip = if host == "localhost" {
            IpAddr::from([127, 0, 0, 1])
        } else {
            host.trim_matches(['[', ']'])
                .parse::<IpAddr>()
                .map_err(|_| invalid())?
        };
        let port = values
            .get("PORT")
            .map(|s| s.parse::<u16>())
            .transpose()
            .map_err(|_| invalid())?
            .unwrap_or(3000);
        if !ip.is_loopback() || port == 0 {
            return Err(invalid());
        }
        let secret = |name: &str| Secret(values.get(name).cloned().unwrap_or_default());
        let access_key = secret("APP_ACCESS_KEY");
        if !access_key.is_empty() && utf16_len(access_key.expose()) < 32 {
            return Err(invalid());
        }
        let asset_dir = values
            .get("FRONTEND_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                executable
                    .parent()
                    .unwrap_or_else(|| Path::new("."))
                    .join("public")
            });
        Ok(Self {
            storage: crate::storage::StorageConfig::from_values(values, executable)?,
            origin,
            bind: SocketAddr::new(ip, port),
            access_key,
            openai_key: secret("OPENAI_API_KEY"),
            tavily_key: secret("TAVILY_API_KEY"),
            model: values
                .get("OPENAI_MODEL")
                .filter(|s| !s.is_empty())
                .cloned()
                .unwrap_or_else(|| "gpt-5.6-terra".into()),
            asset_dir,
            log_filter: values
                .get("RUST_LOG")
                .cloned()
                .unwrap_or_else(|| "info".into()),
            limits: Limits::default(),
        })
    }
    pub fn research_configured(&self) -> bool {
        !self.access_key.is_empty() && !self.openai_key.is_empty() && !self.tavily_key.is_empty()
    }
    pub fn authority(&self) -> &str {
        &self.origin[url::Position::BeforeHost..url::Position::AfterPort]
    }
}

/// Load into owned values; never mutate the process environment after threads start.
pub fn load_values() -> Result<HashMap<String, String>, AppError> {
    let environment: HashMap<String, String> = std::env::vars_os()
        .filter_map(|(key, value)| Some((key.into_string().ok()?, value.into_string().ok()?)))
        .collect();
    let selected = environment.get("APP_ENV_FILE");
    let path = selected.map(String::as_str).unwrap_or(".env");
    let mut values = HashMap::new();
    if !path.is_empty() {
        match dotenvy::from_path_iter(path) {
            Ok(entries) => {
                for entry in entries {
                    let (key, value) =
                        entry.map_err(|_| AppError::new(ErrorCode::InvalidConfig))?;
                    values.insert(key, value);
                }
            }
            Err(dotenvy::Error::Io(error))
                if selected.is_none() && error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err(AppError::new(ErrorCode::InvalidConfig)),
        }
    }
    values.extend(environment);
    Ok(values)
}
