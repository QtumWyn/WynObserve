use std::{
    process::Command,
    sync::{Arc, Mutex},
    thread,
};

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ComponentUpdate {
    pub name: String,
    pub installed: Option<String>,
    pub latest: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateSnapshot {
    pub components: Vec<ComponentUpdate>,
    pub updates_available: usize,
}

#[derive(Debug, Clone)]
pub enum UpdateState {
    Idle,
    Checking,
    Current(UpdateSnapshot),
    Available(UpdateSnapshot),
    Installing,
    Installed(UpdateSnapshot),
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
            let next = match query_update_status() {
                Ok(snapshot) => {
                    if snapshot.updates_available > 0 {
                        UpdateState::Available(snapshot)
                    } else {
                        UpdateState::Current(snapshot)
                    }
                }

                Err(error) => UpdateState::Error(error),
            };

            set_state(&state, next);
        });
    }

    pub fn install_all(&self) {
        set_state(&self.state, UpdateState::Installing);

        let state = Arc::clone(&self.state);

        thread::spawn(move || match run_install() {
            Ok(()) => match query_update_status() {
                Ok(snapshot) => {
                    set_state(&state, UpdateState::Installed(snapshot));
                }

                Err(error) => {
                    set_state(
                        &state,
                        UpdateState::Error(format!(
                            "update installed, but refresh failed: {error}"
                        )),
                    );
                }
            },

            Err(error) => {
                set_state(&state, UpdateState::Error(error));
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

fn query_update_status() -> Result<UpdateSnapshot, String> {
    let output = Command::new("wyn-update")
        .arg("--json")
        .output()
        .map_err(|error| format!("could not launch wyn-update: {error}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);

        return Err(format!("wyn-update --json failed: {}", stderr.trim(),));
    }

    serde_json::from_slice::<UpdateSnapshot>(&output.stdout)
        .map_err(|error| format!("could not parse updater JSON: {error}"))
}

fn run_install() -> Result<(), String> {
    let status = Command::new("wyn-update")
        .args(["--install", "--yes"])
        .status()
        .map_err(|error| format!("could not launch wyn-update: {error}"))?;

    if !status.success() {
        return Err(format!("wyn-update exited with {status}"));
    }

    Ok(())
}

pub fn restart_installed() -> Result<(), String> {
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;

    Command::new(executable)
        .spawn()
        .map_err(|error| error.to_string())?;

    Ok(())
}
