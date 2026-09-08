use std::{
    io::{BufRead, BufReader},
    net::TcpStream,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use wyn_protocol::{FleetMachineState, HubMessage, MachineStatus, PROTOCOL_VERSION};

use crate::{
    fleet::{CpuPackageTopology, FleetMachine, FleetState, MachineOrigin},
    normalize::telemetry_to_system,
};

const RECONNECT_DELAY: Duration = Duration::from_secs(1);

pub struct HubFleetClient {
    latest: Arc<Mutex<Option<FleetState>>>,
    current: FleetState,
}

impl HubFleetClient {
    pub fn connect(endpoint: impl Into<String>) -> Self {
        let endpoint = endpoint.into();

        let latest = Arc::new(Mutex::new(None::<FleetState>));

        let reader_state = Arc::clone(&latest);

        thread::spawn(move || {
            reader_loop(endpoint, reader_state);
        });

        Self {
            latest,

            current: FleetState {
                machines: Vec::new(),
            },
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
}

fn reader_loop(endpoint: String, latest: Arc<Mutex<Option<FleetState>>>) {
    loop {
        match TcpStream::connect(&endpoint) {
            Ok(stream) => {
                eprintln!("Observatory // connected to Fleet Hub at {endpoint}");

                if let Err(error) = read_stream(stream, &latest, &endpoint) {
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

fn read_stream(
    stream: TcpStream,
    latest: &Arc<Mutex<Option<FleetState>>>,
    endpoint: &str,
) -> std::io::Result<()> {
    let reader = BufReader::new(stream);

    for line in reader.lines() {
        let line = line?;

        if line.trim().is_empty() {
            continue;
        }

        match serde_json::from_str::<HubMessage>(&line) {
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

                    continue;
                }

                let fleet = convert_fleet(machines, endpoint);

                let mut guard = latest
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());

                *guard = Some(fleet);
            }

            Err(error) => {
                eprintln!("Observatory // invalid Hub packet: {error}");
            }
        }
    }

    Ok(())
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
