use std::{
    io::{self, BufRead, BufReader, ErrorKind, Write},
    net::{TcpListener, TcpStream},
    sync::mpsc::{self, Receiver},
    thread,
};

use wyn_protocol::{AgentCommand, AgentMessage, PROTOCOL_VERSION, TELEMETRY_SCHEMA_VERSION};

use crate::{
    control_panel::ControlPlane,
    state::{FleetRegistry, SnapshotUpdate},
};

pub fn run(
    registry: FleetRegistry,
    control_plane: ControlPlane,
    listen_address: &str,
) -> io::Result<()> {
    let listener = TcpListener::bind(listen_address)?;

    println!("Hub // agent ingress listening on {listen_address}");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let registry = registry.clone();

                let control_plane = control_plane.clone();

                thread::spawn(move || {
                    if let Err(error) = handle_agent(stream, registry, control_plane) {
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

fn handle_agent(
    stream: TcpStream,
    registry: FleetRegistry,
    control_plane: ControlPlane,
) -> io::Result<()> {
    let peer = stream.peer_addr()?;

    println!("Hub // incoming agent connection from {peer}");

    /*
     * The original socket belongs to the
     * Agent -> Hub reader.
     *
     * The clone belongs to exactly one
     * Hub -> Agent command writer.
     */
    let command_stream = stream.try_clone()?;

    let reader = BufReader::new(stream);

    let (command_tx, command_rx) = mpsc::channel::<AgentCommand>();

    thread::spawn(move || {
        if let Err(error) = command_writer(command_stream, command_rx) {
            eprintln!("Hub // Agent command writer ended // {peer} // {error}");
        }
    });

    /*
     * The machine becomes bound only after
     * a valid AgentHello.
     */
    let mut bound_machine_id: Option<String> = None;

    /*
     * The session ID protects a new
     * connection from cleanup belonging
     * to an older connection with the
     * same machine ID.
     */
    let mut active_session: Option<(String, u64)> = None;

    /*
     * Wrap the read loop so cleanup below
     * runs even when protocol validation
     * returns an error.
     */
    let result = (|| -> io::Result<()> {
        for line in reader.lines() {
            let line = line?;

            if line.trim().is_empty() {
                continue;
            }

            let message: AgentMessage = serde_json::from_str(&line)
                .map_err(|error| io::Error::new(ErrorKind::InvalidData, error))?;

            match message {
                AgentMessage::Hello { hello } => {
                    if bound_machine_id.is_some() {
                        return Err(io::Error::new(
                            ErrorKind::InvalidData,
                            "agent sent hello more than once",
                        ));
                    }

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

                    /*
                     * FleetRegistry owns telemetry
                     * and online/stale/offline state.
                     */
                    registry.register(hello.machine);

                    /*
                     * ControlPlane owns the live
                     * command path back TO Agent.
                     */
                    let session_id =
                        control_plane.register_agent(machine_id.clone(), command_tx.clone());

                    active_session = Some((machine_id.clone(), session_id));

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

                AgentMessage::Response { response } => {
                    let Some(bound_id) = bound_machine_id.as_deref() else {
                        return Err(io::Error::new(
                            ErrorKind::InvalidData,
                            "agent sent response before hello",
                        ));
                    };

                    if response.protocol_version != PROTOCOL_VERSION {
                        return Err(io::Error::new(
                            ErrorKind::InvalidData,
                            format!(
                                "response protocol mismatch: agent={} hub={}",
                                response.protocol_version, PROTOCOL_VERSION,
                            ),
                        ));
                    }

                    let request_id = response.request_id;

                    if control_plane.complete(bound_id, response) {
                        println!("Hub // {} completed Agent request {}", bound_id, request_id,);
                    } else {
                        eprintln!(
                            "Hub // {} returned unknown or mismatched request {}",
                            bound_id, request_id,
                        );
                    }
                }
            }
        }

        Ok(())
    })();

    /*
     * This cleanup runs on BOTH:
     *
     * clean EOF
     * protocol/read errors
     */
    if let Some((machine_id, session_id)) = active_session {
        control_plane.unregister_agent(&machine_id, session_id);
    }

    println!("Hub // agent disconnected // {peer}");

    result
}

fn command_writer(mut stream: TcpStream, command_rx: Receiver<AgentCommand>) -> io::Result<()> {
    for command in command_rx {
        let json = serde_json::to_string(&command).map_err(io::Error::other)?;

        writeln!(stream, "{json}")?;

        stream.flush()?;
    }

    Ok(())
}
