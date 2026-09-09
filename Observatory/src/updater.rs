use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{Arc, Mutex},
    thread,
};

use semver::Version;
use serde::Deserialize;

const RELEASES_API: &str = "https://api.github.com/repos/QtumWyn/WynObserve/releases?per_page=100";

const TAG_PREFIX: &str = "observatory-v";

#[derive(Debug, Clone, Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Debug, Clone, Deserialize)]
struct GithubRelease {
    tag_name: String,
    draft: bool,
    prerelease: bool,
    assets: Vec<GithubAsset>,
}

#[derive(Debug, Clone)]
pub struct AvailableUpdate {
    pub version: Version,
    pub tag: String,
    pub download_url: String,
}

#[derive(Debug, Clone)]
pub enum UpdateState {
    Idle,
    Checking,
    Current,
    Available(AvailableUpdate),
    Downloading,
    Installing,
    Installed(Version),
    Error(String),
}

#[derive(Clone)]
pub struct Updater {
    state: Arc<Mutex<UpdateState>>,
}

impl Default for Updater {
    fn default() -> Self {
        Self {
            state: Arc::new(Mutex::new(UpdateState::Idle)),
        }
    }
}

impl Updater {
    pub fn state(&self) -> UpdateState {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    pub fn check(&self) {
        set_state(&self.state, UpdateState::Checking);

        let state = Arc::clone(&self.state);

        thread::spawn(move || {
            let next = match check_for_update() {
                Ok(Some(update)) => UpdateState::Available(update),

                Ok(None) => UpdateState::Current,

                Err(error) => UpdateState::Error(error),
            };

            set_state(&state, next);
        });
    }

    pub fn install(&self, update: AvailableUpdate) {
        let state = Arc::clone(&self.state);

        thread::spawn(move || {
            set_state(&state, UpdateState::Downloading);

            let path = match download_update(&update) {
                Ok(path) => path,

                Err(error) => {
                    set_state(&state, UpdateState::Error(error));

                    return;
                }
            };

            set_state(&state, UpdateState::Installing);

            match install_package(&path) {
                Ok(()) => {
                    let _ = fs::remove_file(&path);

                    set_state(&state, UpdateState::Installed(update.version));
                }

                Err(error) => {
                    set_state(&state, UpdateState::Error(error));
                }
            }
        });
    }
}

fn set_state(state: &Arc<Mutex<UpdateState>>, value: UpdateState) {
    let mut guard = state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    *guard = value;
}

fn check_for_update() -> Result<Option<AvailableUpdate>, String> {
    let client = github_client()?;

    let releases: Vec<GithubRelease> = client
        .get(RELEASES_API)
        .send()
        .map_err(|error| error.to_string())?
        .error_for_status()
        .map_err(|error| error.to_string())?
        .json()
        .map_err(|error| error.to_string())?;

    let current = Version::parse(env!("CARGO_PKG_VERSION")).map_err(|error| error.to_string())?;

    let latest = releases
        .into_iter()
        .filter_map(release_to_update)
        .filter(|update| update.version > current)
        .max_by(|left, right| left.version.cmp(&right.version));

    Ok(latest)
}

fn release_to_update(release: GithubRelease) -> Option<AvailableUpdate> {
    if release.draft || release.prerelease {
        return None;
    }

    let version = Version::parse(release.tag_name.strip_prefix(TAG_PREFIX)?).ok()?;

    let asset = release.assets.into_iter().find(|asset| {
        asset.name.starts_with("wynobserve_") && asset.name.ends_with("_amd64.deb")
    })?;

    Some(AvailableUpdate {
        version,
        tag: release.tag_name,
        download_url: asset.browser_download_url,
    })
}

fn github_client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .user_agent(concat!("WynObserve/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|error| error.to_string())
}

fn download_update(update: &AvailableUpdate) -> Result<PathBuf, String> {
    let client = github_client()?;

    let response = client
        .get(&update.download_url)
        .send()
        .map_err(|error| error.to_string())?
        .error_for_status()
        .map_err(|error| error.to_string())?;

    let bytes = response.bytes().map_err(|error| error.to_string())?;

    let path = env::temp_dir().join(format!(
        "wynobserve-update-{}-{}.deb",
        update.version,
        std::process::id(),
    ));

    fs::write(&path, &bytes).map_err(|error| error.to_string())?;

    Ok(path)
}

fn install_package(path: &Path) -> Result<(), String> {
    if !Path::new("/usr/bin/pkexec").exists() {
        return Err("pkexec is not installed".to_string());
    }

    let status = Command::new("/usr/bin/pkexec")
        .arg("/usr/bin/apt-get")
        .arg("install")
        .arg("-y")
        .arg(path)
        .stdin(Stdio::null())
        .status()
        .map_err(|error| error.to_string())?;

    match status.code() {
        Some(0) => Ok(()),

        Some(126) => Err("update authorization was cancelled".to_string()),

        Some(127) => Err("update authorization failed".to_string()),

        _ => Err(format!("package installer exited with {status}")),
    }
}

pub fn restart_installed() -> Result<(), String> {
    let installed = Path::new("/usr/bin/wynobserve");

    let executable = if installed.exists() {
        installed.to_path_buf()
    } else {
        env::current_exe().map_err(|error| error.to_string())?
    };

    let args = env::args_os().skip(1).collect::<Vec<_>>();

    Command::new(executable)
        .args(args)
        .spawn()
        .map_err(|error| error.to_string())?;

    Ok(())
}
