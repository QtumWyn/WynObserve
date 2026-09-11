use semver::Version;

use crate::{component::Component, github::LatestRelease, installed::InstalledComponent};

#[derive(Debug)]
pub enum UpdateState {
    NotInstalled,

    Current { version: Version },

    UpdateAvailable { installed: Version, latest: Version },
}

#[derive(Debug)]
pub struct ComponentPlan {
    pub component: Component,
    pub state: UpdateState,
    pub release: Option<LatestRelease>,
}

pub fn build(
    component: Component,
    installed: Option<InstalledComponent>,
    release: Option<LatestRelease>,
) -> ComponentPlan {
    let state = match (&installed, &release) {
        (None, _) => UpdateState::NotInstalled,

        (Some(installed), Some(release)) if release.version > installed.version => {
            UpdateState::UpdateAvailable {
                installed: installed.version.clone(),
                latest: release.version.clone(),
            }
        }

        (Some(installed), _) => UpdateState::Current {
            version: installed.version.clone(),
        },
    };

    ComponentPlan {
        component,
        state,
        release,
    }
}
