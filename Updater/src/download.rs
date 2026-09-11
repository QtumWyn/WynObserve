use std::{
    error::Error,
    fs::{self, File},
    io,
    path::{Path, PathBuf},
};

use crate::{checksum::calculate_sha256, github::LatestRelease};

#[derive(Debug, Clone)]
pub struct VerifiedPackage {
    pub path: PathBuf,
    pub sha256: String,
}

pub fn download_and_verify(release: &LatestRelease) -> Result<VerifiedPackage, Box<dyn Error>> {
    let directory = std::env::temp_dir()
        .join("wynobserve-update")
        .join(release.component.package_name())
        .join(&release.tag);

    fs::create_dir_all(&directory)?;

    let deb_path = directory.join(&release.deb_name);

    let checksum_path = directory.join("SHA256SUMS");

    let client = reqwest::blocking::Client::builder()
        .user_agent("WynObserve-Updater")
        .build()?;

    println!("  downloading {}", release.deb_name,);

    download_file(&client, &release.deb_url, &deb_path)?;

    println!("  downloading SHA256SUMS");

    download_file(&client, &release.checksum_url, &checksum_path)?;

    println!("  verifying SHA-256");

    let sha256 = verify_checksum(&deb_path, &checksum_path, &release.deb_name)?;

    println!("  checksum verified ✓");

    Ok(VerifiedPackage {
        path: deb_path,
        sha256,
    })
}

fn download_file(
    client: &reqwest::blocking::Client,
    url: &str,
    destination: &Path,
) -> Result<(), Box<dyn Error>> {
    let file_name = destination
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "download destination has no filename",
            )
        })?;

    /*
     * Download into a temporary sibling first.
     *
     * That prevents an interrupted download from
     * leaving a file that looks complete.
     */
    let temporary = destination.with_file_name(format!("{file_name}.part"));

    let mut response = client.get(url).send()?.error_for_status()?;

    let mut output = File::create(&temporary)?;

    io::copy(&mut response, &mut output)?;

    output.sync_all()?;

    fs::rename(temporary, destination)?;

    Ok(())
}

fn verify_checksum(
    package_path: &Path,
    checksum_path: &Path,
    package_name: &str,
) -> Result<String, Box<dyn Error>> {
    let checksum_contents = fs::read_to_string(checksum_path)?;

    let expected = checksum_contents
        .lines()
        .find_map(|line| {
            let mut fields = line.split_whitespace();

            let hash = fields.next()?;

            let filename = fields.next()?.trim_start_matches('*');

            if filename == package_name {
                Some(hash.to_ascii_lowercase())
            } else {
                None
            }
        })
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("SHA256SUMS does not contain {package_name}"),
            )
        })?;

    if expected.len() != 64 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid SHA-256 digest for {package_name}"),
        )
        .into());
    }

    let actual = calculate_sha256(package_path)?;

    if actual != expected {
        /*
         * Never leave a package that failed
         * verification sitting around looking usable.
         */
        let _ = fs::remove_file(package_path);

        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "checksum mismatch for {package_name}\n\
                     expected: {expected}\n\
                     actual:   {actual}"
            ),
        )
        .into());
    }

    Ok(actual)
}
