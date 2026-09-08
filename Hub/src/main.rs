mod agent_server;
mod client_server;
mod config;
mod state;

use std::thread;

use state::FleetRegistry;

fn main() -> std::io::Result<()> {
    println!("WynCommand // Fleet Hub :3");

    println!();

    let config = config::HubConfig::load()?;

    println!("Environment // {}", config.environment.to_uppercase());

    println!();

    let registry = FleetRegistry::default();

    let client_registry = registry.clone();

    let observatory_address = config.observatory.listen_address.clone();

    thread::spawn(move || {
        if let Err(error) = client_server::run(client_registry, &observatory_address) {
            eprintln!("Hub // Observatory egress failed: {error}");
        }
    });

    agent_server::run(registry, &config.agent.listen_address)
}
