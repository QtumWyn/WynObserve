mod agent_server;
mod client_server;
mod config;
mod control_panel;
mod state;

use std::thread;

use control_panel::ControlPlane;
use state::FleetRegistry;

fn main() -> std::io::Result<()> {
    println!("WynCommand // Fleet Hub :3");

    println!();

    let config = config::HubConfig::load()?;

    println!("Environment // {}", config.environment.to_uppercase());

    println!();

    /*
     * FleetRegistry:
     * telemetry/state database in memory.
     */
    let registry = FleetRegistry::default();

    let control_plane = ControlPlane::default();

    let client_registry = registry.clone();

    let client_control_plane = control_plane.clone();

    let observatory_address = config.observatory.listen_address.clone();

    thread::spawn(move || {
        if let Err(error) =
            client_server::run(client_registry, client_control_plane, &observatory_address)
        {
            eprintln!("Hub // Observatory egress failed: {error}");
        }
    });

    agent_server::run(registry, control_plane, &config.agent.listen_address)
}
