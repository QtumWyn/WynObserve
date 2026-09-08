use crate::model::{CpuCoreSnapshot, ProcessSnapshot, SystemSnapshot};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MachineOrigin {
    Local,
    Agent { endpoint: String },
}

impl MachineOrigin {
    pub fn label(&self) -> String {
        match self {
            Self::Local => "LOCAL".to_string(),
            Self::Agent { endpoint } => format!("AGENT // {endpoint}"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CpuPackageTopology {
    pub package_id: usize,
    pub model: String,
    pub physical_cores: usize,
    pub logical_cpu_ids: Vec<usize>,
}

#[derive(Debug, Clone)]
pub struct FleetMachine {
    pub id: String,
    pub name: String,
    pub role: String,
    pub online: bool,
    pub origin: MachineOrigin,
    pub agent_version: Option<String>,

    /// The exact normalized snapshot every existing Observatory tab consumes.
    ///
    /// This is the important architecture boundary: once a machine is selected,
    /// CPU / Memory / GPU / Processes / Network / deep-analysis pages simply
    /// receive this machine's snapshot instead of the local one.
    pub system: SystemSnapshot,

    /// Hardware topology that is not naturally represented by the original
    /// single-package `CpuSnapshot`.
    pub cpu_packages: Vec<CpuPackageTopology>,
}

#[derive(Debug, Clone)]
pub struct FleetState {
    pub machines: Vec<FleetMachine>,
}

impl FleetState {
    pub fn mock(local: &SystemSnapshot, elapsed: f32) -> Self {
        Self {
            machines: build_mock_fleet(local, elapsed),
        }
    }

    pub fn refresh_mock(&mut self, local: &SystemSnapshot, elapsed: f32) {
        self.machines = build_mock_fleet(local, elapsed);
    }

    pub fn machine(&self, id: &str) -> Option<&FleetMachine> {
        self.machines.iter().find(|machine| machine.id == id)
    }

    pub fn first_online(&self) -> Option<&FleetMachine> {
        self.machines.iter().find(|machine| machine.online)
    }

    pub fn refresh_hybrid(&mut self, local: &SystemSnapshot, elapsed: f32, live: &FleetState) {
        let mut machines = build_mock_fleet(local, elapsed);

        for live_machine in &live.machines {
            if let Some(existing) = machines
                .iter_mut()
                .find(|machine| machine.id == live_machine.id)
            {
                *existing = live_machine.clone();
            } else {
                machines.push(live_machine.clone());
            }
        }

        self.machines = machines;
    }
}

/// Future network clients can implement this interface.
///
/// Today the UI uses `FleetState::mock`. Later:
///
/// ```text
/// LocalCollector ----------┐
/// EeveeAgentClient --------┤
/// ServerAgentClient -------┼--> FleetState
/// LaptopAgentClient -------┘
/// ```
///
/// Nothing in the rest of Observatory needs to know which machine came from
/// `/proc`, a TLS-connected remote agent, or a replay file.
pub trait FleetProvider {
    fn poll_fleet(&mut self, elapsed: f32) -> FleetState;
}

fn build_mock_fleet(local: &SystemSnapshot, elapsed: f32) -> Vec<FleetMachine> {
    let local_count = local.cpu.logical_cpus.len().max(1);

    let local_machine = FleetMachine {
        id: "wyn-itpc".into(),
        name: "Wyn-ITPC".into(),
        role: "workstation".into(),
        online: true,
        origin: MachineOrigin::Local,
        agent_version: Some("local collector".into()),
        system: local.clone(),
        cpu_packages: vec![CpuPackageTopology {
            package_id: 0,
            model: local.cpu.model.clone(),
            physical_cores: local_count,
            logical_cpu_ids: (0..local_count).collect(),
        }],
    };

    let eevee_system = make_remote_snapshot(
        local,
        16,
        "Eevee workstation CPU // mock agent",
        64,
        elapsed + 1.7,
        0.54,
    );

    let eevee_machine = FleetMachine {
        id: "eevee-pc".into(),
        name: "Eevee-PC".into(),
        role: "workstation".into(),
        online: true,
        origin: MachineOrigin::Agent {
            endpoint: "eevee-pc.local:7443".into(),
        },
        agent_version: Some("wyn-agent 0.1.0-mock".into()),
        system: eevee_system,
        cpu_packages: vec![CpuPackageTopology {
            package_id: 0,
            model: "Eevee workstation CPU".into(),
            physical_cores: 8,
            logical_cpu_ids: (0..16).collect(),
        }],
    };

    // Two physical packages, each 16 physical cores / 32 logical threads.
    let server_system = make_server_snapshot(local, elapsed);

    let server_machine = FleetMachine {
        id: "aplus-server".into(),
        name: "APlus-Server".into(),
        role: "physical server".into(),
        online: true,
        origin: MachineOrigin::Agent {
            endpoint: "aplus-physical-01:7443".into(),
        },
        agent_version: Some("wyn-agent 0.1.0-mock".into()),
        system: server_system,
        cpu_packages: vec![
            CpuPackageTopology {
                package_id: 0,
                model: "Server CPU Package 0".into(),
                physical_cores: 16,
                logical_cpu_ids: (0..32).collect(),
            },
            CpuPackageTopology {
                package_id: 1,
                model: "Server CPU Package 1".into(),
                physical_cores: 16,
                logical_cpu_ids: (32..64).collect(),
            },
        ],
    };

    // Offline nodes remain visible but cannot become the active target.
    let laptop_system = make_remote_snapshot(
        local,
        16,
        "Laptop CPU // last known snapshot",
        32,
        elapsed,
        0.0,
    );

    let laptop_machine = FleetMachine {
        id: "wyn-laptop".into(),
        name: "Wyn-Laptop".into(),
        role: "mobile".into(),
        online: false,
        origin: MachineOrigin::Agent {
            endpoint: "wyn-laptop.local:7443".into(),
        },
        agent_version: None,
        system: laptop_system,
        cpu_packages: vec![CpuPackageTopology {
            package_id: 0,
            model: "Laptop CPU".into(),
            physical_cores: 8,
            logical_cpu_ids: (0..16).collect(),
        }],
    };

    vec![local_machine, eevee_machine, server_machine, laptop_machine]
}

fn make_remote_snapshot(
    local: &SystemSnapshot,
    logical_cpus: usize,
    model: &str,
    memory_gib: u64,
    elapsed: f32,
    base_load: f32,
) -> SystemSnapshot {
    let mut system = local.clone();

    system.cpu.model = model.to_string();

    // Remote mock machines must not inherit the local machine's NPU merely
    // because their snapshot started as a clone. Real agents will report
    // their own capability.
    system.npu.available = false;
    system.npu.model = "No NPU reported // mock".into();
    system.npu.pci_address.clear();
    system.npu.utilization = 0.0;
    system.npu.current_frequency_mhz = 0;
    system.npu.max_frequency_mhz = 0;
    system.npu.memory_used_bytes = 0;
    system.npu.power_state = "unavailable".into();

    // Remote mock machines must also own their GPU identity instead of
    // inheriting the local workstation's real NVML snapshot.
    let remote_gpu_available = base_load > 0.0;
    let remote_gpu_usage = if remote_gpu_available {
        ((elapsed * 0.38).sin() * 0.5 + 0.5) * 62.0
    } else {
        0.0
    };

    system.gpu.available = remote_gpu_available;
    system.gpu.model = if remote_gpu_available {
        "Remote NVIDIA GPU // mock".into()
    } else {
        "No GPU reported // mock".into()
    };
    system.gpu.uuid = if remote_gpu_available {
        "GPU-MOCK-REMOTE".into()
    } else {
        String::new()
    };
    system.gpu.pci_address = if remote_gpu_available {
        "00000000:01:00.0".into()
    } else {
        String::new()
    };
    system.gpu.utilization = remote_gpu_usage;
    system.gpu.memory_activity_percent = if remote_gpu_available {
        remote_gpu_usage * 0.42
    } else {
        0.0
    };
    system.gpu.vram_total_bytes = if remote_gpu_available {
        8 * 1024 * 1024 * 1024
    } else {
        0
    };
    system.gpu.vram_used_bytes = if remote_gpu_available {
        (system.gpu.vram_total_bytes as f32 * 0.31) as u64
    } else {
        0
    };
    system.gpu.temperature_c = if remote_gpu_available {
        39.0 + remote_gpu_usage * 0.28
    } else {
        0.0
    };
    system.gpu.power_watts = if remote_gpu_available {
        24.0 + remote_gpu_usage * 1.5
    } else {
        0.0
    };
    system.gpu.graphics_clock_mhz = remote_gpu_available.then_some(2100);
    system.gpu.sm_clock_mhz = remote_gpu_available.then_some(2100);
    system.gpu.memory_clock_mhz = remote_gpu_available.then_some(7000);
    system.gpu.video_clock_mhz = remote_gpu_available.then_some(1200);
    system.gpu.performance_state = remote_gpu_available.then(|| "P2".into());
    system.gpu.fan_percent = remote_gpu_available.then_some(34);
    system.gpu.power_limit_watts = remote_gpu_available.then_some(220.0);
    system.gpu.pcie_rx_mib_s = remote_gpu_available.then_some(6.0);
    system.gpu.pcie_tx_mib_s = remote_gpu_available.then_some(3.0);

    // Remote mock machines must also own storage/network identity and rates
    // instead of inheriting the local workstation's live collectors.
    let storage_utilization = 9.0 + (elapsed * 0.31).sin().abs() * 44.0;

    system.storage.available = true;
    system.storage.device = "nvme0n1".into();
    system.storage.model = "Remote NVMe // mock".into();
    system.storage.read_mib_s = 18.0 + (elapsed * 0.39).sin().abs() * 540.0;
    system.storage.write_mib_s = 8.0 + (elapsed * 0.47).sin().abs() * 260.0;
    system.storage.read_iops = 180.0 + system.storage.read_mib_s * 14.0;
    system.storage.write_iops = 90.0 + system.storage.write_mib_s * 11.0;
    system.storage.utilization = storage_utilization;
    system.storage.io_in_progress = (storage_utilization / 17.0) as u64;
    system.storage.average_queue_depth = storage_utilization / 28.0;
    system.storage.rates_available = true;

    system.network.available = true;
    system.network.interface = "eth0 // mock".into();
    system.network.rx_mib_s = 2.0 + (elapsed * 0.43).sin().abs() * 34.0;
    system.network.tx_mib_s = 1.0 + (elapsed * 0.51).sin().abs() * 18.0;
    system.network.rx_packets_per_second = (600.0 + system.network.rx_mib_s * 920.0) as u64;
    system.network.tx_packets_per_second = (400.0 + system.network.tx_mib_s * 810.0) as u64;
    system.network.rx_errors_per_second = 0;
    system.network.tx_errors_per_second = 0;
    system.network.rx_drops_per_second = 0;
    system.network.tx_drops_per_second = 0;
    system.network.connections = 18 + ((elapsed * 0.63).sin().abs() * 24.0) as usize;
    system.network.rates_available = true;
    system.network.connections_available = true;

    system.cpu.logical_cpus = (0..logical_cpus)
        .map(|logical_id| {
            let wave = ((elapsed * 0.47 + logical_id as f32 * 0.63).sin() * 0.5 + 0.5) * 52.0;

            CpuCoreSnapshot {
                logical_id,
                usage: (base_load * 35.0 + wave).clamp(0.0, 100.0),
                frequency_mhz: (2200.0 + wave * 28.0) as u32,
                temperature_c: 37.0 + wave * 0.24,
                process: remote_process_name(logical_id).into(),
            }
        })
        .collect();

    system.cpu.package_usage = system
        .cpu
        .logical_cpus
        .iter()
        .map(|cpu| cpu.usage)
        .sum::<f32>()
        / logical_cpus.max(1) as f32;

    system.cpu.package_temperature_c = 40.0 + system.cpu.package_usage * 0.22;

    system.cpu.context_switches_per_second = 9_000 + (system.cpu.package_usage * 290.0) as u64;

    system.cpu.migrations_per_second = 180 + (system.cpu.package_usage * 8.0) as u64;

    system.memory.total_bytes = memory_gib * 1024 * 1024 * 1024;

    let ratio = (0.34 + ((elapsed * 0.11).sin() * 0.5 + 0.5) * 0.22).clamp(0.0, 0.92);

    system.memory.used_bytes = (system.memory.total_bytes as f32 * ratio) as u64;

    system.memory.available_bytes = system
        .memory
        .total_bytes
        .saturating_sub(system.memory.used_bytes);

    system.memory.cached_bytes = (system.memory.total_bytes as f32 * 0.13) as u64;

    system.memory.active_bytes = (system.memory.used_bytes as f32 * 0.64) as u64;

    system.processes = make_processes(logical_cpus, elapsed);
    system.processes_available = true;
    system.process_count = system.processes.len();
    system.thread_count = system.processes.iter().map(|process| process.threads).sum();

    system
}

fn make_server_snapshot(local: &SystemSnapshot, elapsed: f32) -> SystemSnapshot {
    let mut system = make_remote_snapshot(
        local,
        64,
        "2 × Server CPU // 32 physical cores // 64 logical",
        256,
        elapsed + 4.0,
        0.72,
    );

    // A headless server may not expose a desktop GPU at all.
    system.gpu.available = false;
    system.gpu.model = "No general-purpose GPU detected // mock".into();
    system.gpu.uuid.clear();
    system.gpu.pci_address.clear();
    system.gpu.utilization = 0.0;
    system.gpu.memory_activity_percent = 0.0;
    system.gpu.vram_total_bytes = 0;
    system.gpu.vram_used_bytes = 0;
    system.gpu.temperature_c = 0.0;
    system.gpu.power_watts = 0.0;
    system.gpu.graphics_clock_mhz = None;
    system.gpu.sm_clock_mhz = None;
    system.gpu.memory_clock_mhz = None;
    system.gpu.video_clock_mhz = None;
    system.gpu.performance_state = None;
    system.gpu.fan_percent = None;
    system.gpu.power_limit_watts = None;
    system.gpu.pcie_rx_mib_s = None;
    system.gpu.pcie_tx_mib_s = None;

    system.storage.available = true;
    system.storage.device = "zpool0".into();
    system.storage.model = "NVMe RAID / ZFS pool // mock".into();
    system.storage.read_mib_s = 620.0 + (elapsed * 0.31).sin().abs() * 1800.0;
    system.storage.write_mib_s = 240.0 + (elapsed * 0.43).sin().abs() * 940.0;
    system.storage.read_iops = 8_400.0 + system.storage.read_mib_s * 16.0;
    system.storage.write_iops = 2_100.0 + system.storage.write_mib_s * 10.0;
    system.storage.utilization = 28.0 + (elapsed * 0.22).sin().abs() * 48.0;
    system.storage.io_in_progress = (system.storage.utilization / 11.0) as u64;
    system.storage.average_queue_depth = 1.4 + system.storage.utilization / 18.0;
    system.storage.rates_available = true;

    system.network.available = true;
    system.network.interface = "bond0".into();
    system.network.rx_mib_s = 42.0 + (elapsed * 0.37).sin().abs() * 180.0;
    system.network.tx_mib_s = 21.0 + (elapsed * 0.29).sin().abs() * 96.0;
    system.network.rx_packets_per_second = (12_000.0 + system.network.rx_mib_s * 1_400.0) as u64;
    system.network.tx_packets_per_second = (8_000.0 + system.network.tx_mib_s * 1_100.0) as u64;
    system.network.rx_errors_per_second = 0;
    system.network.tx_errors_per_second = 0;
    system.network.rx_drops_per_second = 0;
    system.network.tx_drops_per_second = 0;
    system.network.connections = 260 + ((elapsed * 0.73).sin().abs() * 180.0) as usize;
    system.network.rates_available = true;
    system.network.connections_available = true;

    system.memory.dirty_bytes = 2 * 1024 * 1024 * 1024;

    system.memory.page_faults_per_second = 1200 + ((elapsed * 0.41).sin().abs() * 2400.0) as u64;

    system
}

fn make_processes(logical_cpus: usize, elapsed: f32) -> Vec<ProcessSnapshot> {
    [
        ("web-prod", 3101_u32, 1_420_000_000_u64, 24_usize),
        ("postgres", 1142, 18_800_000_000, 42),
        ("worker-prod", 3320, 2_840_000_000, 18),
        ("observatory-agent", 4412, 142_000_000, 7),
        ("containerd", 880, 482_000_000, 21),
    ]
    .into_iter()
    .enumerate()
    .map(
        |(index, (name, pid, memory_bytes, threads))| ProcessSnapshot {
            pid,
            parent_pid: 1,
            name: name.into(),
            executable: None,
            state: "sleeping".into(),
            cpu_usage: 2.0
                + ((elapsed * (0.25 + index as f32 * 0.09) + index as f32).sin() * 0.5 + 0.5)
                    * (18.0 + index as f32 * 11.0),
            cpu_rate_available: true,
            memory_bytes,
            memory_available: true,
            last_cpu: ((elapsed * (0.8 + index as f32 * 0.15)) as usize + index * 7)
                % logical_cpus.max(1),
            threads,
            read_mib_s: ((elapsed * (0.31 + index as f32 * 0.04)).sin().abs())
                * (4.0 + index as f32 * 2.5),
            write_mib_s: ((elapsed * (0.27 + index as f32 * 0.05)).sin().abs())
                * (2.0 + index as f32 * 1.8),
            io_rates_available: true,
            started_at_unix_ms: None,
        },
    )
    .collect()
}

fn remote_process_name(index: usize) -> &'static str {
    match index % 8 {
        0 => "postgres",
        1 => "web-prod",
        2 => "worker-prod",
        3 => "containerd",
        4 => "kernel",
        5 => "observatory-agent",
        6 => "idle",
        _ => "systemd",
    }
}
