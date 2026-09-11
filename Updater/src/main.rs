mod checksum;
mod component;
mod download;
mod github;
mod install;
mod installed;
mod plan;

use std::{env, error::Error, io, process};

use component::Component;
use download::VerifiedPackage;
use plan::{ComponentPlan, UpdateState};
use serde::Serialize;

#[derive(Serialize)]
struct JsonComponent {
    name: String,
    installed: Option<String>,
    latest: Option<String>,
    status: String,
}

#[derive(Serialize)]
struct JsonStatus {
    components: Vec<JsonComponent>,
    updates_available: usize,
}

#[derive(Debug, Default)]
struct Options {
    download: bool,
    install: bool,
    force: bool,
    yes: bool,
    json: bool,
}

fn main() {
    if let Err(error) = run() {
        eprintln!();

        eprintln!("WynObserve // update failed // {error}");

        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let options = parse_options()?;

    println!("WynObserve // Updater :3");
    println!();

    let mut plans = Vec::new();

    for component in Component::ALL {
        let plan = check_component(component)?;

        print_plan(&plan);

        println!();

        plans.push(plan);
    }

    if options.json {
        print_json(&plans)?;

        return Ok(());
    }

    print_summary(&plans);

    let packages = if options.download {
        println!();

        download_updates(&plans, options.force)?
    } else {
        Vec::new()
    };

    if options.install && !packages.is_empty() {
        println!();

        if !options.yes && !confirm_install(packages.len())? {
            println!("Installation cancelled.");

            return Ok(());
        }

        install::install(&packages, options.force)?;

        println!();

        println!("WynObserve // update complete ✓");
    }

    Ok(())
}

fn check_component(component: Component) -> Result<ComponentPlan, Box<dyn Error>> {
    let installed = installed::detect(component)?;

    let latest = github::latest_release(component)?;

    Ok(plan::build(component, installed, latest))
}

fn print_plan(plan: &ComponentPlan) {
    println!("{}", plan.component);

    match &plan.state {
        UpdateState::NotInstalled => {
            println!("  Installed   NOT INSTALLED");

            if let Some(release) = &plan.release {
                println!("  Latest      {}", release.version,);
            }

            println!("  Status      NOT INSTALLED");
        }

        UpdateState::Current { version } => {
            println!("  Installed   {version}");

            if let Some(release) = &plan.release {
                println!("  Latest      {}", release.version,);
            }

            println!("  Status      CURRENT");
        }

        UpdateState::UpdateAvailable { installed, latest } => {
            println!("  Installed   {installed}");

            println!("  Latest      {latest}");

            println!("  Status      UPDATE AVAILABLE");
        }
    }
}

fn print_summary(plans: &[ComponentPlan]) {
    let update_count = plans
        .iter()
        .filter(|plan| matches!(plan.state, UpdateState::UpdateAvailable { .. }))
        .count();

    println!("System");

    if update_count == 0 {
        println!("  Everything is up to date. ✓");
    } else {
        println!("  {update_count} update(s) available.");
    }
}

fn download_updates(
    plans: &[ComponentPlan],
    force: bool,
) -> Result<Vec<(Component, VerifiedPackage)>, Box<dyn Error>> {
    println!("Downloads");

    let mut packages = Vec::new();

    let mut downloaded = 0_usize;

    for plan in plans {
        let should_download = force || matches!(plan.state, UpdateState::UpdateAvailable { .. });

        if !should_download {
            continue;
        }

        let Some(release) = &plan.release else {
            println!("  {} // no release available", plan.component,);

            continue;
        };

        println!();

        println!("{} {}", plan.component, release.version,);

        let package = download::download_and_verify(release)?;

        packages.push((plan.component, package.clone()));

        println!("  verified package // {}", package.path.display(),);

        downloaded += 1;
    }

    if downloaded == 0 {
        println!("  No downloads needed.");
    } else {
        println!();

        println!("  {downloaded} verified package(s) ready.");
    }

    Ok(packages)
}

fn parse_options() -> Result<Options, Box<dyn Error>> {
    let mut options = Options::default();

    for argument in env::args().skip(1) {
        match argument.as_str() {
            "--download" => {
                options.download = true;
            }

            "--force" => {
                options.force = true;
            }

            "--help" | "-h" => {
                print_help();

                process::exit(0);
            }

            "--install" => {
                options.install = true;
                options.download = true;
            }

            "--yes" | "-y" => {
                options.yes = true;
            }

            "--version" | "-V" => {
                println!("wyn-update {}", env!("CARGO_PKG_VERSION"),);

                process::exit(0);
            }

            "--json" => {
                options.json = true;
            }

            unknown => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("unknown argument `{unknown}`"),
                )
                .into());
            }
        }
    }

    if options.force && !options.download {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "--force requires --download or --install",
        )
        .into());
    }

    if options.yes && !options.install {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "--yes requires --install").into());
    }

    Ok(options)
}

fn print_help() {
    println!(
        "WynObserve Updater\n\
         \n\
         Usage:\n\
         \x20 wyn-update\n\
         \x20 wyn-update --download\n\
         \x20 wyn-update --install\n\
         \x20 wyn-update --install --yes\n\
         \x20 wyn-update --install --force\n\
         \n\
         Options:\n\
         \x20 --download  Download and verify available updates\n\
         \x20 --install   Download, verify, and install updates\n\
         \x20 --force     Include already-current releases\n\
         \x20 --json        Output machine-readable JSON\n\
         \x20 -V, --version  Show updater version\n\
         \x20 -y, --yes   Install without confirmation\n\
         \x20 -h, --help  Show this help"
    );
}

fn confirm_install(package_count: usize) -> Result<bool, Box<dyn Error>> {
    use std::io::Write;

    print!("Install {package_count} verified package(s)? [y/N]: ");

    std::io::stdout().flush()?;

    let mut response = String::new();

    std::io::stdin().read_line(&mut response)?;

    Ok(matches!(
        response.trim().to_ascii_lowercase().as_str(),
        "y" | "yes"
    ))
}

fn print_json(plans: &[ComponentPlan]) -> Result<(), Box<dyn Error>> {
    let components = plans
        .iter()
        .map(|plan| {
            let (installed, latest, status) = match &plan.state {
                UpdateState::NotInstalled => (
                    None,
                    plan.release
                        .as_ref()
                        .map(|release| release.version.to_string()),
                    "not_installed",
                ),

                UpdateState::Current { version } => (
                    Some(version.to_string()),
                    plan.release
                        .as_ref()
                        .map(|release| release.version.to_string()),
                    "current",
                ),

                UpdateState::UpdateAvailable { installed, latest } => (
                    Some(installed.to_string()),
                    Some(latest.to_string()),
                    "update_available",
                ),
            };

            JsonComponent {
                name: plan.component.display_name().to_string(),

                installed,

                latest,

                status: status.to_string(),
            }
        })
        .collect::<Vec<_>>();

    let updates_available = components
        .iter()
        .filter(|component| component.status == "update_available")
        .count();

    let output = JsonStatus {
        components,
        updates_available,
    };

    println!("{}", serde_json::to_string_pretty(&output,)?);

    Ok(())
}
