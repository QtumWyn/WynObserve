use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AgentConfig {
    pub machine_id: String,
    pub machine_name: String,

    #[serde(default = "default_role")]
    pub role: String,

    pub hub: HubConfig,

    #[serde(default)]
    pub local: LocalConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HubConfig {
    pub endpoint: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LocalConfig {
    #[serde(default)]
    pub observatory_enabled: bool,

    #[serde(default = "default_local_listen_address")]
    pub listen_address: String,
}

impl Default for LocalConfig {
    fn default() -> Self {
        Self {
            observatory_enabled: false,
            listen_address: default_local_listen_address(),
        }
    }
}

fn default_role() -> String {
    "workstation".to_string()
}

fn default_local_listen_address() -> String {
    "127.0.0.1:4767".to_string()
}

impl AgentConfig {
    pub fn load() -> io::Result<Self> {
        let path = config_path_from_args();

        println!("Agent // config {}", path.display());

        let contents = fs::read_to_string(&path)?;

        let config: Self = toml::from_str(&contents).map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid agent config {}: {error}", path.display()),
            )
        })?;

        config.validate()?;

        Ok(config)
    }

    fn validate(&self) -> io::Result<()> {
        require_value("machine_id", &self.machine_id)?;

        require_value("machine_name", &self.machine_name)?;

        require_value("role", &self.role)?;

        require_value("hub.endpoint", &self.hub.endpoint)?;

        if self.local.observatory_enabled {
            require_value("local.listen_address", &self.local.listen_address)?;
        }

        Ok(())
    }
}

fn require_value(field: &str, value: &str) -> io::Result<()> {
    if value.trim().is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("agent config field `{field}` cannot be empty"),
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

    if let Some(path) = env::var_os("WYNCOMMAND_CONFIG") {
        return PathBuf::from(path);
    }

    Path::new("wyncommand-agent.toml").to_path_buf()
}
