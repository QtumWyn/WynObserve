use std::{
    io::{self, Write},
    net::TcpStream,
    thread,
    time::Duration,
};

use wyn_protocol::{AgentHello, AgentMessage, MachineIdentity};

use crate::{config::AgentConfig, runtime::TelemetryRuntime};

const RECONNECT_DELAY: Duration = Duration::from_secs(1);

const POLL_INTERVAL: Duration = Duration::from_millis(25);

pub fn run(runtime: TelemetryRuntime, config: AgentConfig) -> ! {
    connection_loop(runtime, config)
}

fn connection_loop(runtime: TelemetryRuntime, config: AgentConfig) -> ! {
    loop {
        match TcpStream::connect(&config.hub.endpoint) {
            Ok(stream) => {
                println!("Agent // connected to Hub at {}", config.hub.endpoint);

                if let Err(error) = run_session(stream, &runtime, &config) {
                    eprintln!("Agent // Hub connection lost: {error}");
                }
            }

            Err(_) => {
                // Hub may not be running.
                // Retry quietly.
            }
        }

        thread::sleep(RECONNECT_DELAY);
    }
}

fn run_session(
    mut stream: TcpStream,
    runtime: &TelemetryRuntime,
    config: &AgentConfig,
) -> io::Result<()> {
    send_hello(&mut stream, config)?;

    let mut last_sequence: Option<u64> = None;

    loop {
        let Some(published) = runtime.latest() else {
            thread::sleep(POLL_INTERVAL);

            continue;
        };

        if last_sequence == Some(published.sequence) {
            thread::sleep(POLL_INTERVAL);

            continue;
        }

        let message = AgentMessage::Snapshot {
            machine_id: config.machine_id.clone(),

            sequence: published.sequence,

            snapshot: published.snapshot,
        };

        send_message(&mut stream, &message)?;

        last_sequence = Some(published.sequence);
    }
}

fn send_hello(stream: &mut TcpStream, config: &AgentConfig) -> io::Result<()> {
    let machine = MachineIdentity {
        machine_id: config.machine_id.clone(),

        machine_name: config.machine_name.clone(),

        role: config.role.clone(),

        os: std::env::consts::OS.to_string(),

        architecture: std::env::consts::ARCH.to_string(),

        agent_version: env!("CARGO_PKG_VERSION").to_string(),
    };

    let hello = AgentHello::new(machine);

    let message = AgentMessage::Hello { hello };

    send_message(stream, &message)
}

fn send_message(stream: &mut TcpStream, message: &AgentMessage) -> io::Result<()> {
    let json = serde_json::to_string(message).map_err(io::Error::other)?;

    writeln!(stream, "{json}")?;

    stream.flush()
}
