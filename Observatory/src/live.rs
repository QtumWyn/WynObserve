use std::{
    io::{BufRead, BufReader},
    net::TcpStream,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use crate::model::{
    CpuCoreSnapshot, CpuSnapshot, GpuSnapshot, MemorySnapshot, NetworkSnapshot, NpuSnapshot,
    ProcessSnapshot, StorageSnapshot, SystemEvent, SystemSnapshot, TelemetrySource, TruthLevel,
};
use crate::normalize::telemetry_to_system;

use wyn_protocol::{TELEMETRY_SCHEMA_VERSION, TelemetrySnapshot};

const SUPPORTED_SCHEMA_VERSION: u16 = TELEMETRY_SCHEMA_VERSION;
const RECONNECT_DELAY: Duration = Duration::from_secs(1);

pub struct LiveTelemetry {
    latest_wire: Arc<Mutex<Option<TelemetrySnapshot>>>,

    latest_system: SystemSnapshot,
}

impl LiveTelemetry {
    pub fn connect(endpoint: impl Into<String>) -> Self {
        let endpoint = endpoint.into();

        let latest_wire = Arc::new(Mutex::new(None::<TelemetrySnapshot>));

        let reader_state = Arc::clone(&latest_wire);

        thread::spawn(move || {
            reader_loop(endpoint, reader_state);
        });

        Self {
            latest_wire,
            latest_system: disconnected_snapshot(),
        }
    }
}

impl TelemetrySource for LiveTelemetry {
    fn poll(&mut self, _elapsed_seconds: f64) -> SystemSnapshot {
        let newest = {
            let mut guard = self
                .latest_wire
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());

            guard.take()
        };

        if let Some(snapshot) = newest {
            self.latest_system = telemetry_to_system(snapshot);
        }

        self.latest_system.clone()
    }
}

fn reader_loop(endpoint: String, latest: Arc<Mutex<Option<TelemetrySnapshot>>>) {
    loop {
        match TcpStream::connect(&endpoint) {
            Ok(stream) => {
                eprintln!("Observatory // connected to M6 at {endpoint}");

                if let Err(error) = read_stream(stream, &latest) {
                    eprintln!("Observatory // M6 connection lost: {error}");
                }
            }

            Err(_) => {
                // Backend may simply not be running yet.
                //
                // Retry quietly instead of flooding the terminal once
                // per second with connection-refused messages.
            }
        }

        thread::sleep(RECONNECT_DELAY);
    }
}

fn read_stream(
    stream: TcpStream,
    latest: &Arc<Mutex<Option<TelemetrySnapshot>>>,
) -> std::io::Result<()> {
    let reader = BufReader::new(stream);

    for line in reader.lines() {
        let line = line?;

        if line.trim().is_empty() {
            continue;
        }

        match serde_json::from_str::<TelemetrySnapshot>(&line) {
            Ok(snapshot) => {
                if snapshot.schema_version != SUPPORTED_SCHEMA_VERSION {
                    eprintln!(
                        "Observatory // unsupported telemetry schema {}",
                        snapshot.schema_version
                    );

                    continue;
                }

                let mut guard = latest
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());

                // Deliberately replace the old unread snapshot.
                //
                // Live mode cares about the newest truth, not about
                // rendering every historical telemetry frame.
                *guard = Some(snapshot);
            }

            Err(error) => {
                eprintln!("Observatory // invalid telemetry packet: {error}");
            }
        }
    }

    Ok(())
}

fn disconnected_snapshot() -> SystemSnapshot {
    SystemSnapshot {
        cpu: CpuSnapshot {
            model: "awaiting M6 telemetry".to_string(),
            package_usage: 0.0,
            package_temperature_c: 0.0,
            logical_cpus: Vec::new(),
            context_switches_per_second: 0,
            migrations_per_second: 0,
        },

        memory: MemorySnapshot {
            total_bytes: 0,
            used_bytes: 0,
            available_bytes: 0,
            cached_bytes: 0,
            active_bytes: 0,
            dirty_bytes: 0,
            page_faults_per_second: 0,
        },

        gpu: GpuSnapshot {
            available: false,
            model: "No GPU telemetry".to_string(),
            uuid: String::new(),
            pci_address: String::new(),
            utilization: 0.0,
            memory_activity_percent: 0.0,
            vram_total_bytes: 0,
            vram_used_bytes: 0,
            temperature_c: 0.0,
            power_watts: 0.0,
            graphics_clock_mhz: None,
            sm_clock_mhz: None,
            memory_clock_mhz: None,
            video_clock_mhz: None,
            performance_state: None,
            fan_percent: None,
            power_limit_watts: None,
            pcie_rx_mib_s: None,
            pcie_tx_mib_s: None,
        },

        npu: NpuSnapshot {
            available: false,
            model: "No NPU telemetry".to_string(),
            pci_address: String::new(),
            utilization: 0.0,
            current_frequency_mhz: 0,
            max_frequency_mhz: 0,
            memory_used_bytes: 0,
            power_state: "unavailable".to_string(),
        },

        storage: StorageSnapshot {
            available: false,
            device: String::new(),
            model: "No storage telemetry".to_string(),
            read_mib_s: 0.0,
            write_mib_s: 0.0,
            read_iops: 0.0,
            write_iops: 0.0,
            utilization: 0.0,
            io_in_progress: 0,
            average_queue_depth: 0.0,
            rates_available: false,
        },

        network: NetworkSnapshot {
            available: false,
            interface: "No network telemetry".to_string(),
            rx_mib_s: 0.0,
            tx_mib_s: 0.0,
            rx_packets_per_second: 0,
            tx_packets_per_second: 0,
            rx_errors_per_second: 0,
            tx_errors_per_second: 0,
            rx_drops_per_second: 0,
            tx_drops_per_second: 0,
            connections: 0,
            rates_available: false,
            connections_available: false,
        },

        processes_available: false,
        process_count: 0,
        thread_count: 0,
        processes: Vec::new(),

        events: vec![SystemEvent {
            age_seconds: 0.0,
            truth: TruthLevel::Observed,
            message: "waiting for M6 telemetry // 127.0.0.1:4767".to_string(),
        }],

        instruction_samples: Vec::new(),
    }
}
