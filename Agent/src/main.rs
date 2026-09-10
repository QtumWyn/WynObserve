mod agent;
mod config;
mod cpu_identity;
mod format;
mod inspection;
mod instruction_vein;
mod metrics;
mod protocol;
mod runtime;
mod snapshot;
mod telemetry;
mod transport;

use std::thread;

fn main() -> std::io::Result<()> {
    println!("WynCommand // Agent :3");

    println!();

    let config = config::AgentConfig::load()?;

    println!(
        "Machine // {} // {} // {}",
        config.machine_name, config.machine_id, config.role,
    );

    println!("Hub // {}", config.hub.endpoint);

    println!();

    let runtime = runtime::TelemetryRuntime::start();

    if config.local.observatory_enabled {
        let local_runtime = runtime.clone();

        let listen_address = config.local.listen_address.clone();

        thread::spawn(move || {
            if let Err(error) = transport::run_server(local_runtime, &listen_address) {
                eprintln!("Agent // local Observatory server failed: {error}");
            }
        });
    }

    if cpu_identity::supports_rdtscp() {
        let first = instruction_vein::read_tsc();

        std::thread::sleep(std::time::Duration::from_millis(10));

        let second = instruction_vein::read_tsc();

        println!(
            "Instruction Vein // TSC // {first} -> {second} // delta {}",
            second.saturating_sub(first),
        );
    } else {
        println!("Instruction Vein // RDTSCP unsupported");
    }

    agent::run(runtime, config)
}
