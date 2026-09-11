use std::{error::Error, io};

use semver::Version;
use serde::Deserialize;

use crate::component::Component;

const RELEASES_URL: &str = "https://api.github.com/repos/QtumWyn/WynObserve/releases";

#[derive(Debug, Deserialize)]
struct GithubAsset {
    name: String,

    browser_download_url: String,
}

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,

    draft: bool,

    prerelease: bool,

    assets: Vec<GithubAsset>,
}

#[derive(Debug, Clone)]
pub struct LatestRelease {
    pub component: Component,

    pub tag: String,

    pub version: Version,

    pub deb_name: String,

    pub deb_url: String,

    pub checksum_url: String,
}

pub fn latest_release(component: Component) -> Result<Option<LatestRelease>, Box<dyn Error>> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("WynObserve-Updater")
        .build()?;

    let releases = client
        .get(RELEASES_URL)
        .send()?
        .error_for_status()?
        .json::<Vec<GithubRelease>>()?;

    let prefix = component.release_prefix();

    /*
     * First choose the newest release by version.
     *
     * We intentionally do this BEFORE checking assets.
     * If our newest release is malformed, we want to
     * report that rather than silently falling back to
     * an older release.
     */
    let latest = releases
        .into_iter()
        .filter(|release| {
            !release.draft && !release.prerelease && release.tag_name.starts_with(prefix)
        })
        .filter_map(|release| {
            let raw = release.tag_name.strip_prefix(prefix)?;

            let version = Version::parse(raw).ok()?;

            Some((version, release))
        })
        .max_by(|left, right| left.0.cmp(&right.0));

    let Some((version, release)) = latest else {
        return Ok(None);
    };

    let deb_asset = release
        .assets
        .iter()
        .find(|asset| {
            asset.name.starts_with(component.package_name()) && asset.name.ends_with(".deb")
        })
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "{} does not contain a Debian package for {}",
                    release.tag_name, component,
                ),
            )
        })?;

    let checksum_asset = release
        .assets
        .iter()
        .find(|asset| asset.name == "SHA256SUMS")
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("{} does not contain SHA256SUMS", release.tag_name,),
            )
        })?;

    Ok(Some(LatestRelease {
        component,

        tag: release.tag_name,

        version,

        deb_name: deb_asset.name.clone(),

        deb_url: deb_asset.browser_download_url.clone(),

        checksum_url: checksum_asset.browser_download_url.clone(),
    }))
}
