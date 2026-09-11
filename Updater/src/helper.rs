mod checksum;

use std::{
    env,
    error::Error,
    fs::{self, File, OpenOptions},
    io::{self, Write},
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{self, Command, ExitCode},
};

use checksum::calculate_sha256;

const UPDATE_ROOT: &str = "/tmp/wynobserve-update";

const STAGING_ROOT: &str = "/var/lib/wynobserve/updater";

struct PackageRequest {
    expected_sha256: String,
    source: PathBuf,
}

struct StagedPackage {
    path: PathBuf,
    package_name: String,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,

        Err(error) => {
            eprintln!("WynObserve // privileged update failed // {error}");

            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    require_root()?;

    let mut arguments = env::args().skip(1);

    let action = arguments
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing helper action"))?;

    if action != "install" {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unknown helper action `{action}`"),
        )
        .into());
    }

    let mut reinstall = false;

    let mut packages = Vec::new();

    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--reinstall" => {
                reinstall = true;
            }

            "--package" => {
                let expected_sha256 = arguments.next().ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--package requires a SHA-256 digest",
                    )
                })?;

                let source = arguments.next().ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--package requires a package path",
                    )
                })?;

                packages.push(PackageRequest {
                    expected_sha256: validate_hash(&expected_sha256)?,

                    source: PathBuf::from(source),
                });
            }

            unknown => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("unknown helper argument `{unknown}`"),
                )
                .into());
            }
        }
    }

    if packages.is_empty() {
        return Err(
            io::Error::new(io::ErrorKind::InvalidInput, "no update packages supplied").into(),
        );
    }

    install_requests(&packages, reinstall)
}

fn require_root() -> io::Result<()> {
    let uid = unsafe { libc::geteuid() };

    if uid != 0 {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "wyn-update-helper must run as root",
        ));
    }

    Ok(())
}

fn validate_hash(hash: &str) -> io::Result<String> {
    if hash.len() != 64 || !hash.chars().all(|character| character.is_ascii_hexdigit()) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid SHA-256 digest",
        ));
    }

    Ok(hash.to_ascii_lowercase())
}

fn install_requests(requests: &[PackageRequest], reinstall: bool) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(STAGING_ROOT)?;

    fs::set_permissions(STAGING_ROOT, fs::Permissions::from_mode(0o755))?;

    let staging_directory = PathBuf::from(STAGING_ROOT).join(format!("staging-{}", process::id(),));

    fs::create_dir(&staging_directory)?;

    fs::set_permissions(&staging_directory, fs::Permissions::from_mode(0o755))?;

    let result = stage_and_install(requests, &staging_directory, reinstall);

    /*
     * Packages are no longer needed after apt
     * has consumed them.
     */
    let _ = fs::remove_dir_all(&staging_directory);

    result
}

fn stage_and_install(
    requests: &[PackageRequest],
    staging_directory: &Path,
    reinstall: bool,
) -> Result<(), Box<dyn Error>> {
    let mut staged = Vec::new();

    for request in requests {
        staged.push(stage_package(request, staging_directory)?);
    }

    install_packages(&staged, reinstall)?;

    refresh_systemd()?;

    if staged
        .iter()
        .any(|package| package.package_name == "wyncommand-agent")
    {
        restart_agent()?;
    }

    Ok(())
}

fn stage_package(
    request: &PackageRequest,
    staging_directory: &Path,
) -> Result<StagedPackage, Box<dyn Error>> {
    let source = validate_source(&request.source)?;

    let file_name = source
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "package has no filename"))?;

    let destination = staging_directory.join(file_name);

    /*
     * Open the user-controlled source and copy it
     * into a root-owned directory.
     *
     * All security checks below operate on the
     * root-owned copy, not the original /tmp file.
     */
    let mut input = File::open(&source)?;

    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&destination)?;

    io::copy(&mut input, &mut output)?;

    output.flush()?;
    output.sync_all()?;

    fs::set_permissions(&destination, fs::Permissions::from_mode(0o644))?;

    let actual_sha256 = calculate_sha256(&destination)?;

    if actual_sha256 != request.expected_sha256 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("root-side checksum mismatch for {}", destination.display(),),
        )
        .into());
    }

    let package_name = package_field(&destination, "Package")?;

    ensure_allowed_package(&package_name)?;

    /*
     * Make sure a file called
     * wynobserve_something.deb really contains
     * the wynobserve package.
     */
    validate_filename(&destination, &package_name)?;

    validate_architecture(&destination)?;

    println!("WynObserve // verified {} as root", package_name,);

    Ok(StagedPackage {
        path: destination,
        package_name,
    })
}

fn validate_source(source: &Path) -> io::Result<PathBuf> {
    let canonical_source = fs::canonicalize(source)?;

    let canonical_root = fs::canonicalize(UPDATE_ROOT)?;

    if !canonical_source.starts_with(&canonical_root) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("package is outside {UPDATE_ROOT}",),
        ));
    }

    if canonical_source.extension() != Some(std::ffi::OsStr::new("deb")) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "update package is not a .deb",
        ));
    }

    Ok(canonical_source)
}

fn ensure_allowed_package(package_name: &str) -> io::Result<()> {
    match package_name {
        "wyncommand-agent" | "wynobserve" => Ok(()),

        _ => Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("package `{package_name}` is not an allowed WynObserve component",),
        )),
    }
}

fn validate_filename(path: &Path, package_name: &str) -> io::Result<()> {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid package filename"))?;

    let expected_prefix = format!("{package_name}_");

    if !file_name.starts_with(&expected_prefix) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("filename `{file_name}` does not match package `{package_name}`",),
        ));
    }

    Ok(())
}

fn validate_architecture(package: &Path) -> io::Result<()> {
    let package_architecture = package_field(package, "Architecture")?;

    let output = Command::new("/usr/bin/dpkg")
        .arg("--print-architecture")
        .output()?;

    if !output.status.success() {
        return Err(io::Error::other("could not determine system architecture"));
    }

    let system_architecture = String::from_utf8_lossy(&output.stdout).trim().to_string();

    if package_architecture != system_architecture {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "package architecture `{package_architecture}` does not match system `{system_architecture}`",
            ),
        ));
    }

    Ok(())
}

fn package_field(package: &Path, field: &str) -> io::Result<String> {
    let output = Command::new("/usr/bin/dpkg-deb")
        .arg("-f")
        .arg(package)
        .arg(field)
        .output()?;

    if !output.status.success() {
        return Err(io::Error::other(format!(
            "could not read Debian field `{field}`",
        )));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn install_packages(packages: &[StagedPackage], reinstall: bool) -> io::Result<()> {
    println!("WynObserve // installing {} package(s)", packages.len(),);

    let mut command = Command::new("/usr/bin/apt-get");

    command
        .env("DEBIAN_FRONTEND", "noninteractive")
        .arg("install")
        .arg("-y")
        .arg("--no-remove");

    if reinstall {
        command.arg("--reinstall");
    }

    command.arg("--");

    for package in packages {
        command.arg(&package.path);
    }

    let status = command.status()?;

    if !status.success() {
        return Err(io::Error::other(format!("apt-get exited with {status}",)));
    }

    Ok(())
}

fn refresh_systemd() -> io::Result<()> {
    let status = Command::new("/usr/bin/systemctl")
        .arg("daemon-reload")
        .status()?;

    if !status.success() {
        return Err(io::Error::other("systemctl daemon-reload failed"));
    }

    Ok(())
}

fn restart_agent() -> io::Result<()> {
    println!("WynObserve // restarting Agent",);

    let status = Command::new("/usr/bin/systemctl")
        .args(["restart", "wyn-agent"])
        .status()?;

    if !status.success() {
        return Err(io::Error::other("could not restart wyn-agent"));
    }

    Ok(())
}
