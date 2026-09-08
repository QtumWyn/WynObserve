use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct HubConfig {
    #[serde(default = "default_environment")]
    pub environment: String,

    pub agent: ListenerConfig,
    pub observatory: ListenerConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ListenerConfig {
    pub listen_address: String,
}

impl HubConfig {
    pub fn load() -> io::Result<Self> {
        let path = config_path_from_args();

        println!("Hub // config {}", path.display());

        let contents = fs::read_to_string(&path)?;

        let config: Self = toml::from_str(&contents).map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid Hub config {}: {error}", path.display()),
            )
        })?;

        config.validate()?;

        Ok(config)
    }

    fn validate(&self) -> io::Result<()> {
        require_value("environment", &self.environment)?;

        require_value("agent.listen_address", &self.agent.listen_address)?;

        require_value(
            "observatory.listen_address",
            &self.observatory.listen_address,
        )?;

        Ok(())
    }
}

fn default_environment() -> String {
    "development".to_string()
}

fn require_value(field: &str, value: &str) -> io::Result<()> {
    if value.trim().is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Hub config field `{field}` cannot be empty"),
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

    if let Some(path) = env::var_os("WYNCOMMAND_HUB_CONFIG") {
        return PathBuf::from(path);
    }

    Path::new("wyncommand-hub.toml").to_path_buf()
}
