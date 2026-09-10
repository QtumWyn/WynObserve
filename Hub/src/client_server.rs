use std::{
    io::{self, BufRead, BufReader, ErrorKind, Write},
    net::{TcpListener, TcpStream},
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use wyn_protocol::{
    AgentCommandKind, AgentResponseKind, HubMessage, ObservatoryRequest, ObservatoryRequestKind,
    ObservatoryResponseKind, PROTOCOL_VERSION,
};

use crate::{control_panel::ControlPlane, state::FleetRegistry};

const FLEET_INTERVAL: Duration = Duration::from_millis(500);

pub fn run(
    registry: FleetRegistry,
    control_plane: ControlPlane,
    listen_address: &str,
) -> io::Result<()> {
    let listener = TcpListener::bind(listen_address)?;

    println!("Hub // Observatory egress listening on {listen_address}");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let registry = registry.clone();

                let control_plane = control_plane.clone();

                thread::spawn(move || {
                    if let Err(error) = handle_client(stream, registry, control_plane) {
                        eprintln!("Hub // Observatory client disconnected: {error}");
                    }
                });
            }

            Err(error) => {
                eprintln!("Hub // failed to accept Observatory client: {error}");
            }
        }
    }

    Ok(())
}

fn handle_client(
    stream: TcpStream,
    registry: FleetRegistry,
    control_plane: ControlPlane,
) -> io::Result<()> {
    let peer = stream.peer_addr()?;

    println!("Hub // Observatory client connected // {peer}");

    /*
     * Reader gets one clone.
     *
     * Writer owns the original socket.
     */
    let read_stream = stream.try_clone()?;

    let (outgoing_tx, outgoing_rx) = mpsc::channel::<HubMessage>();

    /*
     * Exactly ONE thread writes to the
     * Observatory TCP stream.
     */
    thread::spawn(move || {
        if let Err(error) = message_writer(stream, outgoing_rx) {
            eprintln!("Hub // Observatory writer ended // {peer} // {error}");
        }
    });

    /*
     * Fleet snapshots remain periodic,
     * but now they enter the same outgoing
     * channel as request responses.
     */
    {
        let fleet_tx = outgoing_tx.clone();

        let fleet_registry = registry.clone();

        thread::spawn(move || {
            fleet_publisher(fleet_registry, fleet_tx);
        });
    }

    /*
     * The connection's read side handles
     * commands coming FROM Observatory.
     */
    let reader = BufReader::new(read_stream);

    for line in reader.lines() {
        let line = line?;

        if line.trim().is_empty() {
            continue;
        }

        let request: ObservatoryRequest = serde_json::from_str(&line)
            .map_err(|error| io::Error::new(ErrorKind::InvalidData, error))?;

        /*
         * Don't block the socket reader
         * for up to five seconds while
         * ControlPlane waits on an Agent.
         */
        let request_plane = control_plane.clone();

        let response_tx = outgoing_tx.clone();

        thread::spawn(move || {
            let response = handle_request(request, request_plane);

            let _ = response_tx.send(response);
        });
    }

    println!("Hub // Observatory client closed // {peer}");

    Ok(())
}

fn fleet_publisher(registry: FleetRegistry, outgoing: Sender<HubMessage>) {
    loop {
        let message = HubMessage::FleetSnapshot {
            protocol_version: PROTOCOL_VERSION,

            generated_at_unix_ms: unix_timestamp_ms(),

            machines: registry.fleet_snapshot(),
        };

        if outgoing.send(message).is_err() {
            return;
        }

        thread::sleep(FLEET_INTERVAL);
    }
}

fn handle_request(request: ObservatoryRequest, control_plane: ControlPlane) -> HubMessage {
    let request_id = request.request_id;

    if request.protocol_version != PROTOCOL_VERSION {
        return error_message(
            request_id,
            "protocol_mismatch",
            format!(
                "Observatory protocol {} does not match Hub protocol {}",
                request.protocol_version, PROTOCOL_VERSION,
            ),
        );
    }

    match request.request {
        ObservatoryRequestKind::ProcessMemoryMap {
            machine_id,
            pid,
            expected_started_at_unix_ms,
        } => route_process_memory_map(
            request_id,
            machine_id,
            pid,
            expected_started_at_unix_ms,
            control_plane,
        ),

        ObservatoryRequestKind::InstructionVeinSample {
            machine_id,
            pid,
            expected_started_at_unix_ms,
        } => route_instruction_vein_sample(
            request_id,
            machine_id,
            pid,
            expected_started_at_unix_ms,
            control_plane,
        ),
    }
}

fn route_process_memory_map(
    observatory_request_id: u64,
    machine_id: String,
    pid: u32,
    expected_started_at_unix_ms: Option<u64>,
    control_plane: ControlPlane,
) -> HubMessage {
    let result = control_plane.request(
        &machine_id,
        AgentCommandKind::ProcessMemoryMap {
            pid,
            expected_started_at_unix_ms,
        },
    );

    let agent_response = match result {
        Ok(response) => response,

        Err(error) => {
            return error_message(observatory_request_id, "agent_request_failed", error);
        }
    };

    if agent_response.protocol_version != PROTOCOL_VERSION {
        return error_message(
            observatory_request_id,
            "agent_protocol_mismatch",
            format!(
                "Agent response protocol {} does not match Hub protocol {}",
                agent_response.protocol_version, PROTOCOL_VERSION,
            ),
        );
    }

    match agent_response.result {
        AgentResponseKind::ProcessMemoryMap { map } => HubMessage::Response {
            protocol_version: PROTOCOL_VERSION,

            request_id: observatory_request_id,

            response: ObservatoryResponseKind::ProcessMemoryMap { machine_id, map },
        },

        AgentResponseKind::InstructionVein { .. } => error_message(
            observatory_request_id,
            "unexpected_agent_response",
            "Agent returned Instruction Vein data for a process-memory-map request",
        ),

        AgentResponseKind::Error { code, message } => {
            error_message(observatory_request_id, code, message)
        }
    }
}

fn route_instruction_vein_sample(
    observatory_request_id: u64,
    machine_id: String,
    pid: u32,
    expected_started_at_unix_ms: Option<u64>,
    control_plane: ControlPlane,
) -> HubMessage {
    let result = control_plane.request(
        &machine_id,
        AgentCommandKind::InstructionVeinSample {
            pid,
            expected_started_at_unix_ms,
        },
    );

    let agent_response = match result {
        Ok(response) => response,

        Err(error) => {
            return error_message(observatory_request_id, "agent_request_failed", error);
        }
    };

    if agent_response.protocol_version != PROTOCOL_VERSION {
        return error_message(
            observatory_request_id,
            "agent_protocol_mismatch",
            format!(
                "Agent response protocol {} does not match Hub protocol {}",
                agent_response.protocol_version, PROTOCOL_VERSION,
            ),
        );
    }

    match agent_response.result {
        AgentResponseKind::InstructionVein { batch } => HubMessage::Response {
            protocol_version: PROTOCOL_VERSION,

            request_id: observatory_request_id,

            response: ObservatoryResponseKind::InstructionVein { machine_id, batch },
        },

        AgentResponseKind::ProcessMemoryMap { .. } => error_message(
            observatory_request_id,
            "unexpected_agent_response",
            "Agent returned a process memory map for an Instruction Vein request",
        ),

        AgentResponseKind::Error { code, message } => {
            error_message(observatory_request_id, code, message)
        }
    }
}

fn error_message(
    request_id: u64,
    code: impl Into<String>,
    message: impl Into<String>,
) -> HubMessage {
    HubMessage::Response {
        protocol_version: PROTOCOL_VERSION,

        request_id,

        response: ObservatoryResponseKind::Error {
            code: code.into(),

            message: message.into(),
        },
    }
}

fn message_writer(mut stream: TcpStream, outgoing: Receiver<HubMessage>) -> io::Result<()> {
    for message in outgoing {
        let json = serde_json::to_string(&message).map_err(io::Error::other)?;

        writeln!(stream, "{json}")?;

        stream.flush()?;
    }

    Ok(())
}

fn unix_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u64::MAX as u128) as u64
}
