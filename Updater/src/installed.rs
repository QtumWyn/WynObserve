use std::{io, process::Command};

use semver::Version;

use crate::component::Component;

#[derive(Debug, Clone)]
pub struct InstalledComponent {
    pub version: Version,
}

pub fn detect(component: Component) -> io::Result<Option<InstalledComponent>> {
    let output = Command::new("dpkg-query")
        .args(["-W", "-f=${Status}\t${Version}", component.package_name()])
        .output()?;

    if !output.status.success() {
        return Ok(None);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    let Some((status, raw_version)) = stdout.trim().split_once('\t') else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("unexpected dpkg-query output for {}", component),
        ));
    };

    if status != "install ok installed" {
        return Ok(None);
    }

    let version = parse_debian_version(raw_version).map_err(io::Error::other)?;

    Ok(Some(InstalledComponent { version }))
}

fn parse_debian_version(value: &str) -> Result<Version, semver::Error> {
    /*
     * Examples:
     *
     * 0.2.0
     * 0.2.0-1
     *
     * semver doesn't understand Debian's
     * packaging revision, so strip it.
     */
    let semver_part = value.split('-').next().unwrap_or(value);

    Version::parse(semver_part)
}
