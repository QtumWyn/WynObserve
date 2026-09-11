use std::{env, error::Error, io, path::PathBuf, process::Command};

use crate::{component::Component, download::VerifiedPackage};

pub fn install(
    packages: &[(Component, VerifiedPackage)],
    reinstall: bool,
) -> Result<(), Box<dyn Error>> {
    let helper = find_helper()?;

    println!("WynObserve // requesting administrator authorization");

    let mut command = Command::new("pkexec");

    command.arg(helper).arg("install");

    if reinstall {
        command.arg("--reinstall");
    }

    for (_, package) in packages {
        command
            .arg("--package")
            .arg(&package.sha256)
            .arg(&package.path);
    }

    let status = command.status()?;

    if !status.success() {
        return Err(io::Error::other(format!("privileged updater exited with {status}")).into());
    }

    refresh_kde(packages);

    Ok(())
}

fn find_helper() -> io::Result<PathBuf> {
    /*
     * Development:
     *
     * target/debug/wyn-update
     * target/debug/wyn-update-helper
     */
    let current = env::current_exe()?;

    if let Some(directory) = current.parent() {
        let sibling = directory.join("wyn-update-helper");

        if sibling.is_file() {
            return Ok(sibling);
        }
    }

    /*
     * Installed package.
     */
    let installed = PathBuf::from("/usr/libexec/wynobserve/wyn-update-helper");

    if installed.is_file() {
        return Ok(installed);
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "could not locate wyn-update-helper",
    ))
}

fn refresh_kde(packages: &[(Component, VerifiedPackage)]) {
    let observatory_updated = packages
        .iter()
        .any(|(component, _)| matches!(component, Component::Observatory));

    if !observatory_updated {
        return;
    }

    let _ = Command::new("kbuildsycoca6")
        .arg("--noincremental")
        .status();
}
