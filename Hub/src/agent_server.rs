use std::{
    io::{self, BufRead, BufReader, ErrorKind},
    net::{TcpListener, TcpStream},
    thread,
};

use wyn_protocol::{AgentMessage, PROTOCOL_VERSION, TELEMETRY_SCHEMA_VERSION};

use crate::state::{FleetRegistry, SnapshotUpdate};

pub fn run(registry: FleetRegistry, listen_address: &str) -> io::Result<()> {
    let listener = TcpListener::bind(listen_address)?;

    println!("Hub // agent ingress listening on {listen_address}");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let registry = registry.clone();

                thread::spawn(move || {
                    if let Err(error) = handle_agent(stream, registry) {
                        eprintln!("Hub // agent connection ended: {error}");
                    }
                });
            }

            Err(error) => {
                eprintln!("Hub // failed to accept agent: {error}");
            }
        }
    }

    Ok(())
}

fn handle_agent(stream: TcpStream, registry: FleetRegistry) -> io::Result<()> {
    let peer = stream.peer_addr()?;

    println!("Hub // incoming agent connection from {peer}");

    let reader = BufReader::new(stream);

    let mut bound_machine_id: Option<String> = None;

    for line in reader.lines() {
        let line = line?;

        if line.trim().is_empty() {
            continue;
        }

        let message: AgentMessage = serde_json::from_str(&line)
            .map_err(|error| io::Error::new(ErrorKind::InvalidData, error))?;

        match message {
            AgentMessage::Hello { hello } => {
                if hello.protocol_version != PROTOCOL_VERSION {
                    return Err(io::Error::new(
                        ErrorKind::InvalidData,
                        format!(
                            "protocol mismatch: agent={} hub={}",
                            hello.protocol_version, PROTOCOL_VERSION,
                        ),
                    ));
                }

                let machine_id = hello.machine.machine_id.clone();

                println!(
                    "Hub // registered {} // {} // agent {}",
                    hello.machine.machine_name, machine_id, hello.machine.agent_version,
                );

                registry.register(hello.machine);

                bound_machine_id = Some(machine_id);

                println!(
                    "Hub // fleet now contains {} machine(s)",
                    registry.machine_count(),
                );
            }

            AgentMessage::Snapshot {
                machine_id,
                sequence,
                snapshot,
            } => {
                let Some(bound_id) = bound_machine_id.as_deref() else {
                    return Err(io::Error::new(
                        ErrorKind::InvalidData,
                        "agent sent snapshot before hello",
                    ));
                };

                if bound_id != machine_id {
                    return Err(io::Error::new(
                        ErrorKind::InvalidData,
                        format!(
                            "machine identity changed: connection={} packet={}",
                            bound_id, machine_id,
                        ),
                    ));
                }

                if snapshot.schema_version != TELEMETRY_SCHEMA_VERSION {
                    return Err(io::Error::new(
                        ErrorKind::InvalidData,
                        format!(
                            "telemetry schema mismatch: agent={} hub={}",
                            snapshot.schema_version, TELEMETRY_SCHEMA_VERSION,
                        ),
                    ));
                }

                let cpu = snapshot.cpu.global_usage_percent;

                let process_count = snapshot.process_count;

                match registry.update_snapshot(&machine_id, sequence, snapshot) {
                    SnapshotUpdate::Accepted => {
                        // Don't vomit 2 log lines
                        // every second forever.
                        if sequence % 20 == 0 {
                            println!(
                                "Hub // {} // seq {} // CPU {:.1}% // {} processes",
                                machine_id, sequence, cpu, process_count,
                            );
                        }
                    }

                    SnapshotUpdate::UnknownMachine => {
                        return Err(io::Error::new(
                            ErrorKind::InvalidData,
                            "snapshot references unregistered machine",
                        ));
                    }

                    SnapshotUpdate::StaleSequence { previous } => {
                        eprintln!(
                            "Hub // {} ignored stale sequence {} // newest {}",
                            machine_id, sequence, previous,
                        );
                    }
                }
            }
        }
    }

    println!("Hub // agent disconnected // {peer}");

    Ok(())
}
