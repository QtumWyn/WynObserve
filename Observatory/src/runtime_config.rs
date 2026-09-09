use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ObservatoryConfig {
    #[serde(default = "default_environment")]
    pub environment: String,

    pub hub: HubConfig,

    #[serde(default)]
    pub local: LocalTelemetryConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HubConfig {
    pub endpoint: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LocalTelemetryConfig {
    #[serde(default)]
    pub enabled: bool,

    #[serde(default = "default_local_endpoint")]
    pub endpoint: String,
}

impl Default for LocalTelemetryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            endpoint: default_local_endpoint(),
        }
    }
}

impl ObservatoryConfig {
    pub fn load() -> io::Result<Self> {
        let path = config_path_from_args();

        println!("Observatory // config {}", path.display());

        let contents = fs::read_to_string(&path)?;

        let config: Self = toml::from_str(&contents).map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid Observatory config {}: {error}", path.display()),
            )
        })?;

        config.validate()?;

        Ok(config)
    }

    fn validate(&self) -> io::Result<()> {
        require_value("hub.endpoint", &self.hub.endpoint)?;

        if self.local.enabled {
            require_value("local.endpoint", &self.local.endpoint)?;
        }

        Ok(())
    }
}

fn default_environment() -> String {
    "development".to_string()
}

fn default_local_endpoint() -> String {
    "127.0.0.1:4767".to_string()
}

fn require_value(field: &str, value: &str) -> io::Result<()> {
    if value.trim().is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Observatory config field `{field}` cannot be empty"),
        ));
    }

    Ok(())
}

fn config_path_from_args() -> PathBuf {
    let mut args = env::args().skip(1);

    while let Some(argument) = args.next() {
        if argument == "--config" {
            if let Some(path) = args.next() {
                return PathBuf::from(path);
            }
        }
    }

    if let Some(path) = env::var_os("WYNCOMMAND_OBSERVATORY_CONFIG") {
        return PathBuf::from(path);
    }

    Path::new("wyncommand-observatory.toml").to_path_buf()
}
