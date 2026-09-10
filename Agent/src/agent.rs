use std::{
    io::{self, BufRead, BufReader, Write},
    net::TcpStream,
    sync::mpsc::{self, Receiver, Sender, TryRecvError},
    thread,
    time::Duration,
};

use wyn_protocol::{
    AgentCommand, AgentCommandKind, AgentHello, AgentMessage, AgentResponse, AgentResponseKind,
    MachineIdentity, PROTOCOL_VERSION,
};

use crate::{config::AgentConfig, inspection, runtime::TelemetryRuntime};

const RECONNECT_DELAY: Duration = Duration::from_secs(1);

const POLL_INTERVAL: Duration = Duration::from_millis(25);

enum SessionEvent {
    Response(AgentResponse),

    ReaderClosed(String),
}

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
    let read_stream = stream.try_clone()?;

    let (event_tx, event_rx) = mpsc::channel();

    let command_runtime = runtime.clone();

    thread::spawn(move || {
        command_reader(read_stream, command_runtime, event_tx);
    });

    send_hello(&mut stream, config)?;

    let mut last_sequence: Option<u64> = None;

    loop {
        drain_session_events(&mut stream, &event_rx)?;

        if let Some(published) = runtime.latest() {
            if last_sequence != Some(published.sequence) {
                let message = AgentMessage::Snapshot {
                    machine_id: config.machine_id.clone(),

                    sequence: published.sequence,

                    snapshot: published.snapshot,
                };

                send_message(&mut stream, &message)?;

                last_sequence = Some(published.sequence);
            }
        }

        thread::sleep(POLL_INTERVAL);
    }
}

fn command_reader(stream: TcpStream, runtime: TelemetryRuntime, event_tx: Sender<SessionEvent>) {
    let reader = BufReader::new(stream);

    for line in reader.lines() {
        let line = match line {
            Ok(line) => line,

            Err(error) => {
                let _ = event_tx.send(SessionEvent::ReaderClosed(error.to_string()));

                return;
            }
        };

        if line.trim().is_empty() {
            continue;
        }

        let command: AgentCommand = match serde_json::from_str(&line) {
            Ok(command) => command,

            Err(error) => {
                eprintln!("Agent // invalid Hub command: {error}");

                continue;
            }
        };

        let response = handle_command(command, &runtime);

        if event_tx.send(SessionEvent::Response(response)).is_err() {
            return;
        }
    }

    let _ = event_tx.send(SessionEvent::ReaderClosed(
        "Hub closed command stream".to_string(),
    ));
}

fn handle_command(command: AgentCommand, runtime: &TelemetryRuntime) -> AgentResponse {
    if command.protocol_version != PROTOCOL_VERSION {
        return error_response(
            command.request_id,
            "protocol_mismatch",
            format!(
                "Agent protocol {} cannot handle Hub protocol {}",
                PROTOCOL_VERSION, command.protocol_version,
            ),
        );
    }

    match command.command {
        AgentCommandKind::ProcessMemoryMap {
            pid,
            expected_started_at_unix_ms,
        } => handle_process_memory_map(
            command.request_id,
            pid,
            expected_started_at_unix_ms,
            runtime,
        ),
        AgentCommandKind::InstructionVeinSample {
            pid,
            expected_started_at_unix_ms,
        } => handle_instruction_vein_sample(
            command.request_id,
            pid,
            expected_started_at_unix_ms,
            runtime,
        ),
    }
}

fn handle_process_memory_map(
    request_id: u64,
    pid: u32,
    expected_started_at_unix_ms: Option<u64>,
    runtime: &TelemetryRuntime,
) -> AgentResponse {
    if let Some(expected_start) = expected_started_at_unix_ms {
        let Some(published) = runtime.latest() else {
            return error_response(
                request_id,
                "telemetry_unavailable",
                "Agent does not yet have a telemetry snapshot",
            );
        };

        let process = published
            .snapshot
            .processes
            .iter()
            .find(|process| process.pid == pid);

        let Some(process) = process else {
            return error_response(
                request_id,
                "process_not_found",
                format!("PID {pid} is no longer present in the Agent process snapshot"),
            );
        };

        if process.started_at_unix_ms != Some(expected_start) {
            return error_response(
                request_id,
                "process_identity_changed",
                format!("PID {pid} no longer refers to the process selected by Observatory"),
            );
        }
    }

    match inspection::process_memory::read(pid) {
        Ok(map) => AgentResponse {
            protocol_version: PROTOCOL_VERSION,

            request_id,

            result: AgentResponseKind::ProcessMemoryMap { map },
        },

        Err(error) => error_response(
            request_id,
            "memory_map_failed",
            format!("Could not inspect PID {pid}: {error}"),
        ),
    }
}

fn handle_instruction_vein_sample(
    request_id: u64,
    pid: u32,
    expected_started_at_unix_ms: Option<u64>,
    runtime: &TelemetryRuntime,
) -> AgentResponse {
    /*
     * Same PID-reuse protection as the
     * Process Memory Mapper.
     */
    if let Some(expected_start) = expected_started_at_unix_ms {
        let Some(published) = runtime.latest() else {
            return error_response(
                request_id,
                "telemetry_unavailable",
                "Agent does not yet have a telemetry snapshot",
            );
        };

        let process = published
            .snapshot
            .processes
            .iter()
            .find(|process| process.pid == pid);

        let Some(process) = process else {
            return error_response(
                request_id,
                "process_not_found",
                format!("PID {pid} is no longer present in the Agent process snapshot"),
            );
        };

        if process.started_at_unix_ms != Some(expected_start) {
            return error_response(
                request_id,
                "process_identity_changed",
                format!("PID {pid} no longer refers to the process selected by Observatory"),
            );
        }
    }

    let mut sampler = match crate::instruction_vein::ProcessInstructionSampler::start(pid) {
        Ok(sampler) => sampler,

        Err(error) => {
            return error_response(request_id, "instruction_vein_start_failed", error);
        }
    };

    std::thread::sleep(std::time::Duration::from_millis(75));

    let samples = match sampler.poll() {
        Ok(samples) => samples,

        Err(error) => {
            return error_response(request_id, "instruction_vein_poll_failed", error);
        }
    };

    sampler.stop();

    let samples = samples
        .into_iter()
        .take(256)
        .map(
            |sample| wyn_protocol::instruction_vein::InstructionVeinSample {
                ip: sample.ip,
                pid: sample.pid,
                tid: sample.tid,
                cpu: sample.cpu,

                perf_time: sample.timestamp,

                bytes: sample.bytes,

                instruction: sample.instruction,
            },
        )
        .collect();

    AgentResponse {
        protocol_version: PROTOCOL_VERSION,

        request_id,

        result: AgentResponseKind::InstructionVein {
            batch: wyn_protocol::instruction_vein::InstructionVeinBatch { pid, samples },
        },
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

fn drain_session_events(
    stream: &mut TcpStream,
    event_rx: &Receiver<SessionEvent>,
) -> io::Result<()> {
    loop {
        match event_rx.try_recv() {
            Ok(SessionEvent::Response(response)) => {
                let message = AgentMessage::Response { response };

                send_message(stream, &message)?;
            }

            Ok(SessionEvent::ReaderClosed(reason)) => {
                return Err(io::Error::new(io::ErrorKind::ConnectionReset, reason));
            }

            Err(TryRecvError::Empty) => {
                return Ok(());
            }

            Err(TryRecvError::Disconnected) => {
                return Err(io::Error::new(
                    io::ErrorKind::ConnectionReset,
                    "Agent command reader stopped",
                ));
            }
        }
    }
}

fn error_response(
    request_id: u64,
    code: impl Into<String>,
    message: impl Into<String>,
) -> AgentResponse {
    AgentResponse {
        protocol_version: PROTOCOL_VERSION,

        request_id,

        result: AgentResponseKind::Error {
            code: code.into(),

            message: message.into(),
        },
    }
}
