use std::{
    collections::HashMap,
    io::{BufRead, BufReader, ErrorKind, Write},
    net::TcpStream,
    sync::{
        Arc, Mutex,
        mpsc::{self, Receiver, Sender, TryRecvError},
    },
    thread,
    time::Duration,
};

use wyn_protocol::{
    FleetMachineState, HubMessage, MachineStatus, ObservatoryRequest, ObservatoryRequestKind,
    ObservatoryResponseKind, PROTOCOL_VERSION,
};

use crate::{
    fleet::{CpuPackageTopology, FleetMachine, FleetState, MachineOrigin},
    normalize::telemetry_to_system,
};

const RECONNECT_DELAY: Duration = Duration::from_secs(1);
const IO_POLL_INTERVAL: Duration = Duration::from_millis(50);

pub struct HubFleetClient {
    latest: Arc<Mutex<Option<FleetState>>>,
    current: FleetState,
    request_tx: Sender<ObservatoryRequest>,
    responses: Arc<Mutex<HashMap<u64, ObservatoryResponseKind>>>,
    next_request_id: u64,
}

impl HubFleetClient {
    pub fn connect(endpoint: impl Into<String>) -> Self {
        let endpoint = endpoint.into();

        let latest = Arc::new(Mutex::new(None::<FleetState>));

        let responses = Arc::new(Mutex::new(HashMap::new()));

        let (request_tx, request_rx) = mpsc::channel();

        let reader_state = Arc::clone(&latest);

        let reader_responses = Arc::clone(&responses);

        thread::spawn(move || {
            connection_loop(endpoint, reader_state, reader_responses, request_rx);
        });

        Self {
            latest,

            current: FleetState {
                machines: Vec::new(),
            },

            request_tx,

            responses,

            next_request_id: 1,
        }
    }

    pub fn poll(&mut self) -> FleetState {
        let newest = {
            let mut guard = self
                .latest
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());

            guard.take()
        };

        if let Some(fleet) = newest {
            self.current = fleet;
        }

        self.current.clone()
    }

    pub fn request_process_memory_map(
        &mut self,
        machine_id: &str,
        pid: u32,
        expected_started_at_unix_ms: Option<u64>,
    ) -> Result<u64, String> {
        let request_id = self.next_request_id;

        self.next_request_id = self.next_request_id.wrapping_add(1).max(1);

        let request = ObservatoryRequest {
            protocol_version: PROTOCOL_VERSION,

            request_id,

            request: ObservatoryRequestKind::ProcessMemoryMap {
                machine_id: machine_id.to_string(),

                pid,

                expected_started_at_unix_ms,
            },
        };

        self.request_tx
            .send(request)
            .map_err(|error| format!("could not queue Hub request: {error}"))?;

        Ok(request_id)
    }

    pub fn request_instruction_vein_sample(
        &mut self,
        machine_id: &str,
        pid: u32,
        expected_started_at_unix_ms: Option<u64>,
    ) -> Result<u64, String> {
        let request_id = self.next_request_id;

        self.next_request_id = self.next_request_id.wrapping_add(1).max(1);

        let request = ObservatoryRequest {
            protocol_version: PROTOCOL_VERSION,

            request_id,

            request: ObservatoryRequestKind::InstructionVeinSample {
                machine_id: machine_id.to_string(),

                pid,

                expected_started_at_unix_ms,
            },
        };

        self.request_tx
            .send(request)
            .map_err(|error| format!("could not queue Hub request: {error}"))?;

        Ok(request_id)
    }

    pub fn take_response(&self, request_id: u64) -> Option<ObservatoryResponseKind> {
        self.responses
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(&request_id)
    }
}

fn connection_loop(
    endpoint: String,

    latest: Arc<Mutex<Option<FleetState>>>,

    responses: Arc<Mutex<HashMap<u64, ObservatoryResponseKind>>>,

    request_rx: Receiver<ObservatoryRequest>,
) {
    loop {
        match TcpStream::connect(&endpoint) {
            Ok(stream) => {
                eprintln!("Observatory // connected to Fleet Hub at {endpoint}");

                if let Err(error) = run_session(stream, &latest, &responses, &request_rx, &endpoint)
                {
                    eprintln!("Observatory // Fleet Hub connection lost: {error}");
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
    mut write_stream: TcpStream,

    latest: &Arc<Mutex<Option<FleetState>>>,

    responses: &Arc<Mutex<HashMap<u64, ObservatoryResponseKind>>>,

    request_rx: &Receiver<ObservatoryRequest>,

    endpoint: &str,
) -> std::io::Result<()> {
    let read_stream = write_stream.try_clone()?;

    read_stream.set_read_timeout(Some(IO_POLL_INTERVAL))?;

    let mut reader = BufReader::new(read_stream);

    let mut line = String::new();

    loop {
        drain_requests(&mut write_stream, request_rx)?;

        line.clear();

        match reader.read_line(&mut line) {
            Ok(0) => {
                return Err(std::io::Error::new(
                    ErrorKind::UnexpectedEof,
                    "Fleet Hub closed connection",
                ));
            }

            Ok(_) => {
                if line.trim().is_empty() {
                    continue;
                }

                handle_hub_message(&line, latest, responses, endpoint);
            }

            Err(error) if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {
                continue;
            }

            Err(error) => {
                return Err(error);
            }
        }
    }
}

fn drain_requests(
    stream: &mut TcpStream,

    request_rx: &Receiver<ObservatoryRequest>,
) -> std::io::Result<()> {
    loop {
        match request_rx.try_recv() {
            Ok(request) => {
                let json = serde_json::to_string(&request).map_err(std::io::Error::other)?;

                writeln!(stream, "{json}")?;

                stream.flush()?;
            }

            Err(TryRecvError::Empty) => {
                return Ok(());
            }

            Err(TryRecvError::Disconnected) => {
                return Err(std::io::Error::new(
                    ErrorKind::BrokenPipe,
                    "Observatory request channel closed",
                ));
            }
        }
    }
}

fn handle_hub_message(
    line: &str,

    latest: &Arc<Mutex<Option<FleetState>>>,

    responses: &Arc<Mutex<HashMap<u64, ObservatoryResponseKind>>>,

    endpoint: &str,
) {
    match serde_json::from_str::<HubMessage>(line) {
        Ok(HubMessage::FleetSnapshot {
            protocol_version,
            generated_at_unix_ms: _,
            machines,
        }) => {
            if protocol_version != PROTOCOL_VERSION {
                eprintln!(
                    "Observatory // unsupported Hub protocol {}",
                    protocol_version
                );

                return;
            }

            let fleet = convert_fleet(machines, endpoint);

            let mut guard = latest
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());

            *guard = Some(fleet);
        }

        Ok(HubMessage::Response {
            protocol_version,
            request_id,
            response,
        }) => {
            if protocol_version != PROTOCOL_VERSION {
                eprintln!(
                    "Observatory // unsupported Hub response protocol {}",
                    protocol_version
                );

                return;
            }

            responses
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .insert(request_id, response);
        }

        Err(error) => {
            eprintln!("Observatory // invalid Hub packet: {error}");
        }
    }
}

fn convert_fleet(machines: Vec<FleetMachineState>, endpoint: &str) -> FleetState {
    let machines = machines
        .into_iter()
        .filter_map(|machine| convert_machine(machine, endpoint))
        .collect();

    FleetState { machines }
}

fn convert_machine(machine: FleetMachineState, endpoint: &str) -> Option<FleetMachine> {
    let snapshot = machine.snapshot?;

    let model = snapshot.cpu.brand.clone();

    let physical_cores = snapshot
        .cpu
        .physical_core_count
        .unwrap_or(snapshot.cpu.logical_cpu_count);

    let logical_cpu_ids = snapshot
        .cpu
        .logical_cpus
        .iter()
        .map(|cpu| cpu.logical_id)
        .collect();

    let system = telemetry_to_system(snapshot);

    let online = matches!(machine.status, MachineStatus::Online);

    Some(FleetMachine {
        id: machine.machine.machine_id,

        name: machine.machine.machine_name,

        // Temporary until machine role becomes
        // configurable Agent metadata.
        role: machine.machine.role.clone(),

        online,

        origin: MachineOrigin::Agent {
            endpoint: endpoint.to_string(),
        },

        agent_version: Some(machine.machine.agent_version),

        system,

        cpu_packages: vec![CpuPackageTopology {
            package_id: 0,

            model,

            physical_cores,

            logical_cpu_ids,
        }],
    })
}
