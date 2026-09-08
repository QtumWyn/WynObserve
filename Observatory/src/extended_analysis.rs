use crate::{
    fleet::{FleetMachine, FleetState},
    model::{SystemSnapshot, TruthLevel},
};

#[derive(Debug, Clone)]
pub struct PcieDevice {
    pub address: String,
    pub class: String,
    pub name: String,
    pub parent: String,
    pub numa_node: Option<usize>,
    pub link: String,
    pub iommu_group: Option<u32>,
    pub activity_percent: f32,
}

#[derive(Debug, Clone)]
pub struct NumaNodeSnapshot {
    pub node_id: usize,
    pub cpu_ids: Vec<usize>,
    pub total_memory_bytes: u64,
    pub used_memory_bytes: u64,
    pub local_access_percent: f32,
    pub remote_access_percent: f32,
    pub migrations_per_second: u64,
}

#[derive(Debug, Clone)]
pub struct StorageIoSnapshot {
    pub device: String,
    pub filesystem: String,
    pub read_iops: f32,
    pub write_iops: f32,
    pub queue_depth: f32,
    pub read_latency_ms: f32,
    pub write_latency_ms: f32,
    pub utilization_percent: f32,
    pub dirty_writeback_mib_s: f32,
}

#[derive(Debug, Clone)]
pub struct ThermalZoneSnapshot {
    pub name: String,
    pub temperature_c: f32,
    pub trip_c: f32,
    pub power_watts: f32,
    pub throttling: bool,
    pub fan_rpm: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct LockSnapshot {
    pub lock_name: String,
    pub owner: String,
    pub waiter_count: usize,
    pub wait_ms: f32,
    pub acquisitions_per_second: u64,
    pub contention_percent: f32,
}

#[derive(Debug, Clone)]
pub struct CgroupSnapshot {
    pub path: String,
    pub kind: String,
    pub processes: usize,
    pub cpu_percent: f32,
    pub memory_bytes: u64,
    pub memory_limit_bytes: Option<u64>,
    pub cpu_quota_percent: Option<f32>,
    pub oom_events: u64,
}

#[derive(Debug, Clone)]
pub struct DatabaseSnapshot {
    pub name: String,
    pub active_connections: usize,
    pub transactions_per_second: f32,
    pub cache_hit_percent: f32,
    pub wal_mib_s: f32,
    pub blocked_queries: usize,
    pub longest_query_ms: f32,
    pub active_queries: Vec<DatabaseQuery>,
}

#[derive(Debug, Clone)]
pub struct DatabaseQuery {
    pub pid: u32,
    pub age_ms: f32,
    pub state: String,
    pub summary: String,
}

#[derive(Debug, Clone)]
pub struct TraceSpan {
    pub service: String,
    pub operation: String,
    pub machine: String,
    pub start_ms: f32,
    pub duration_ms: f32,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct DiffEntry {
    pub category: String,
    pub item: String,
    pub before: String,
    pub after: String,
    pub severity: String,
}

#[derive(Debug, Clone)]
pub struct FirmwareEntry {
    pub component: String,
    pub value: String,
    pub truth: TruthLevel,
}

#[derive(Debug, Clone)]
pub struct SecurityObservation {
    pub subject: String,
    pub detail: String,
    pub severity: String,
    pub truth: TruthLevel,
}

#[derive(Debug, Clone)]
pub struct SourceSiliconStep {
    pub layer: String,
    pub value: String,
    pub truth: TruthLevel,
}

#[derive(Debug, Clone)]
pub struct AnomalyObservation {
    pub score: f32,
    pub title: String,
    pub detail: String,
    pub truth: TruthLevel,
}

#[derive(Debug, Clone)]
pub struct IncidentDossier {
    pub id: String,
    pub machine: String,
    pub title: String,
    pub age_seconds: f32,
    pub severity: String,
    pub summary: String,
}

#[derive(Debug, Clone)]
pub struct CrossMachineEvent {
    pub offset_ms: i32,
    pub machine: String,
    pub event: String,
    pub truth: TruthLevel,
}

#[derive(Debug, Clone)]
pub struct ExtremeSnapshot {
    pub pcie_devices: Vec<PcieDevice>,
    pub numa_nodes: Vec<NumaNodeSnapshot>,
    pub storage_io: Vec<StorageIoSnapshot>,
    pub thermal_zones: Vec<ThermalZoneSnapshot>,
    pub locks: Vec<LockSnapshot>,
    pub cgroups: Vec<CgroupSnapshot>,
    pub database: DatabaseSnapshot,
    pub trace_spans: Vec<TraceSpan>,
    pub diff: Vec<DiffEntry>,
    pub firmware: Vec<FirmwareEntry>,
    pub security: Vec<SecurityObservation>,
    pub source_silicon: Vec<SourceSiliconStep>,
    pub anomalies: Vec<AnomalyObservation>,
    pub incidents: Vec<IncidentDossier>,
    pub cross_machine: Vec<CrossMachineEvent>,
}

impl ExtremeSnapshot {
    pub fn mock(machine: &FleetMachine, fleet: &FleetState, elapsed: f32) -> Self {
        let system = &machine.system;
        let serverish = machine.role.contains("server");

        let pcie_devices = vec![
            PcieDevice {
                address: "0000:00:00.0".into(),
                class: "Host bridge".into(),
                name: "CPU Root Complex".into(),
                parent: "CPU package 0".into(),
                numa_node: Some(0),
                link: "internal".into(),
                iommu_group: Some(0),
                activity_percent: system.cpu.package_usage,
            },
            PcieDevice {
                address: "0000:01:00.0".into(),
                class: "VGA / Compute".into(),
                name: if system.gpu.vram_total_bytes > 0 {
                    system.gpu.model.clone()
                } else {
                    "No GPU exposed".into()
                },
                parent: "PCIe root port 01".into(),
                numa_node: Some(0),
                link: "PCIe 5.0 x16".into(),
                iommu_group: Some(12),
                activity_percent: system.gpu.utilization,
            },
            PcieDevice {
                address: "0000:04:00.0".into(),
                class: "Non-Volatile memory".into(),
                name: system.storage.model.clone(),
                parent: if serverish {
                    "CPU package 1 / root port".into()
                } else {
                    "CPU package 0 / root port".into()
                },
                numa_node: Some(if serverish { 1 } else { 0 }),
                link: "PCIe 4.0 x4".into(),
                iommu_group: Some(18),
                activity_percent: system.storage.utilization,
            },
            PcieDevice {
                address: "0000:08:00.0".into(),
                class: "Ethernet controller".into(),
                name: system.network.interface.clone(),
                parent: if serverish {
                    "CPU package 1 / root port".into()
                } else {
                    "CPU package 0 / root port".into()
                },
                numa_node: Some(if serverish { 1 } else { 0 }),
                link: if serverish {
                    "PCIe 4.0 x8 / 25GbE".into()
                } else {
                    "PCIe 3.0 x1".into()
                },
                iommu_group: Some(21),
                activity_percent: (system.network.rx_mib_s + system.network.tx_mib_s).min(100.0),
            },
        ];

        let package_count = machine.cpu_packages.len().max(1);
        let memory_per_node = system.memory.total_bytes / package_count as u64;
        let used_per_node = system.memory.used_bytes / package_count as u64;

        let numa_nodes = machine
            .cpu_packages
            .iter()
            .enumerate()
            .map(|(index, package)| {
                let remote = if package_count > 1 {
                    7.0 + ((elapsed * 0.41 + index as f32).sin() * 0.5 + 0.5) * 18.0
                } else {
                    1.0
                };

                NumaNodeSnapshot {
                    node_id: index,
                    cpu_ids: package.logical_cpu_ids.clone(),
                    total_memory_bytes: memory_per_node,
                    used_memory_bytes: used_per_node,
                    local_access_percent: 100.0 - remote,
                    remote_access_percent: remote,
                    migrations_per_second: if package_count > 1 {
                        220 + (remote * 31.0) as u64
                    } else {
                        0
                    },
                }
            })
            .collect();

        let storage_io = vec![StorageIoSnapshot {
            device: if system.storage.device.is_empty() {
                if serverish {
                    "storage device unavailable".into()
                } else {
                    "unknown block device".into()
                }
            } else {
                system.storage.device.clone()
            },
            filesystem: if serverish {
                "zpool/aplus".into()
            } else {
                "/".into()
            },
            read_iops: if system.storage.rates_available {
                system.storage.read_iops
            } else {
                0.0
            },
            write_iops: if system.storage.rates_available {
                system.storage.write_iops
            } else {
                0.0
            },
            queue_depth: if system.storage.rates_available {
                system.storage.average_queue_depth
            } else {
                system.storage.io_in_progress as f32
            },
            // Latency and dirty-writeback are still schematic until M6
            // grows collectors for those specific signals.
            read_latency_ms: 0.18 + system.storage.utilization / 250.0,
            write_latency_ms: 0.31 + system.storage.utilization / 180.0,
            utilization_percent: system.storage.utilization,
            dirty_writeback_mib_s: 18.0 + (elapsed * 0.39).sin().abs() * 90.0,
        }];

        let cpu_temp = system.cpu.package_temperature_c;

        let thermal_zones = vec![
            ThermalZoneSnapshot {
                name: "CPU package".into(),
                temperature_c: cpu_temp,
                trip_c: 95.0,
                power_watts: 52.0 + system.cpu.package_usage * 1.6,
                throttling: cpu_temp > 88.0,
                fan_rpm: Some((820.0 + cpu_temp * 22.0) as u32),
            },
            ThermalZoneSnapshot {
                name: "GPU".into(),
                temperature_c: system.gpu.temperature_c,
                trip_c: 88.0,
                power_watts: system.gpu.power_watts,
                throttling: system.gpu.temperature_c > 84.0,
                fan_rpm: if system.gpu.vram_total_bytes > 0 {
                    Some((900.0 + system.gpu.temperature_c * 28.0) as u32)
                } else {
                    None
                },
            },
            ThermalZoneSnapshot {
                name: "NVMe".into(),
                temperature_c: 42.0 + system.storage.utilization * 0.18,
                trip_c: 78.0,
                power_watts: 6.8 + system.storage.utilization * 0.07,
                throttling: false,
                fan_rpm: None,
            },
        ];

        let locks = vec![
            LockSnapshot {
                lock_name: "futex://rustc/query-cache".into(),
                owner: "T18491".into(),
                waiter_count: 5,
                wait_ms: 4.8,
                acquisitions_per_second: 18_220,
                contention_percent: 12.4,
            },
            LockSnapshot {
                lock_name: "postgres://buffer_mapping".into(),
                owner: "PID1142/T22".into(),
                waiter_count: if serverish { 11 } else { 2 },
                wait_ms: if serverish { 13.2 } else { 2.1 },
                acquisitions_per_second: 8_420,
                contention_percent: if serverish { 27.0 } else { 5.3 },
            },
            LockSnapshot {
                lock_name: "kernel://mmap_lock".into(),
                owner: "various".into(),
                waiter_count: 2,
                wait_ms: 0.9,
                acquisitions_per_second: 4_810,
                contention_percent: 3.7,
            },
        ];

        let cgroups = vec![
            CgroupSnapshot {
                path: "/system.slice".into(),
                kind: "systemd cgroup".into(),
                processes: 86,
                cpu_percent: system.cpu.package_usage * 0.24,
                memory_bytes: system.memory.used_bytes / 7,
                memory_limit_bytes: None,
                cpu_quota_percent: None,
                oom_events: 0,
            },
            CgroupSnapshot {
                path: "/containers/web-prod".into(),
                kind: "container".into(),
                processes: 12,
                cpu_percent: 18.2 + (elapsed * 0.3).sin().abs() * 9.0,
                memory_bytes: 1_420_000_000,
                memory_limit_bytes: Some(4 * 1024 * 1024 * 1024),
                cpu_quota_percent: Some(200.0),
                oom_events: 0,
            },
            CgroupSnapshot {
                path: "/containers/worker-prod".into(),
                kind: "container".into(),
                processes: 18,
                cpu_percent: 22.4,
                memory_bytes: 2_840_000_000,
                memory_limit_bytes: Some(8 * 1024 * 1024 * 1024),
                cpu_quota_percent: Some(400.0),
                oom_events: 1,
            },
        ];

        let database = DatabaseSnapshot {
            name: if serverish {
                "PostgreSQL // production".into()
            } else {
                "PostgreSQL // local/dev".into()
            },
            active_connections: if serverish { 82 } else { 9 },
            transactions_per_second: if serverish {
                612.0 + (elapsed * 0.25).sin().abs() * 240.0
            } else {
                22.0
            },
            cache_hit_percent: 99.34,
            wal_mib_s: if serverish { 18.4 } else { 0.7 },
            blocked_queries: if serverish { 2 } else { 0 },
            longest_query_ms: if serverish { 1842.0 } else { 48.0 },
            active_queries: vec![
                DatabaseQuery {
                    pid: 20411,
                    age_ms: 31.4,
                    state: "active".into(),
                    summary: "SELECT schedule rows ...".into(),
                },
                DatabaseQuery {
                    pid: 20418,
                    age_ms: 1842.0,
                    state: "waiting // lock".into(),
                    summary: "UPDATE production_jobs ...".into(),
                },
                DatabaseQuery {
                    pid: 20433,
                    age_ms: 8.7,
                    state: "active".into(),
                    summary: "INSERT telemetry_event ...".into(),
                },
            ],
        };

        let trace_spans = vec![
            TraceSpan {
                service: "edge".into(),
                operation: "POST /api/job".into(),
                machine: machine.name.clone(),
                start_ms: 0.0,
                duration_ms: 4.8,
                status: "OK".into(),
            },
            TraceSpan {
                service: "web".into(),
                operation: "request handler".into(),
                machine: machine.name.clone(),
                start_ms: 4.8,
                duration_ms: 11.2,
                status: "OK".into(),
            },
            TraceSpan {
                service: "api".into(),
                operation: "validate + dispatch".into(),
                machine: machine.name.clone(),
                start_ms: 16.0,
                duration_ms: 9.4,
                status: "OK".into(),
            },
            TraceSpan {
                service: "postgres".into(),
                operation: "transaction".into(),
                machine: machine.name.clone(),
                start_ms: 25.4,
                duration_ms: 6.8,
                status: "OK".into(),
            },
            TraceSpan {
                service: "storage".into(),
                operation: "WAL flush".into(),
                machine: machine.name.clone(),
                start_ms: 32.2,
                duration_ms: 3.2,
                status: "OK".into(),
            },
        ];

        let diff = vec![
            DiffEntry {
                category: "kernel".into(),
                item: "version".into(),
                before: "6.14.0".into(),
                after: "6.15.3".into(),
                severity: "INFO".into(),
            },
            DiffEntry {
                category: "module".into(),
                item: "nvidia".into(),
                before: "580.159.04".into(),
                after: "581.22.03".into(),
                severity: "INFO".into(),
            },
            DiffEntry {
                category: "network".into(),
                item: "listening :7443".into(),
                before: "absent".into(),
                after: "observatory-agent".into(),
                severity: "EXPECTED".into(),
            },
            DiffEntry {
                category: "service".into(),
                item: "unknown-worker.service".into(),
                before: "absent".into(),
                after: "running".into(),
                severity: "REVIEW".into(),
            },
        ];

        let firmware = vec![
            FirmwareEntry {
                component: "System firmware".into(),
                value: "UEFI 3402 // mock".into(),
                truth: TruthLevel::Observed,
            },
            FirmwareEntry {
                component: "CPU microcode".into(),
                value: "0x0000012B // mock".into(),
                truth: TruthLevel::Observed,
            },
            FirmwareEntry {
                component: "Secure Boot".into(),
                value: "enabled".into(),
                truth: TruthLevel::Observed,
            },
            FirmwareEntry {
                component: "IOMMU".into(),
                value: "enabled // interrupt remapping active".into(),
                truth: TruthLevel::Inferred,
            },
            FirmwareEntry {
                component: "GPU VBIOS".into(),
                value: if system.gpu.vram_total_bytes > 0 {
                    "mock-vbios-96.04.xx".into()
                } else {
                    "not present".into()
                },
                truth: TruthLevel::Observed,
            },
            FirmwareEntry {
                component: "SMBIOS".into(),
                value: "3.7 // board inventory available".into(),
                truth: TruthLevel::Observed,
            },
        ];

        let security = vec![
            SecurityObservation {
                subject: "observatory-agent".into(),
                detail: "telemetry endpoint bound to authenticated channel".into(),
                severity: "GOOD".into(),
                truth: TruthLevel::Observed,
            },
            SecurityObservation {
                subject: "postgres".into(),
                detail: "listening only on private interface".into(),
                severity: "GOOD".into(),
                truth: TruthLevel::Observed,
            },
            SecurityObservation {
                subject: "worker-prod".into(),
                detail: "seccomp profile present; 31 syscalls permitted".into(),
                severity: "INFO".into(),
                truth: TruthLevel::Observed,
            },
            SecurityObservation {
                subject: "PID 28114".into(),
                detail: "unexpected executable anonymous mapping".into(),
                severity: "REVIEW".into(),
                truth: TruthLevel::Inferred,
            },
        ];

        let source_silicon = vec![
            SourceSiliconStep {
                layer: "SOURCE".into(),
                value: "src/parser.rs:184 // Parser::parse_node()".into(),
                truth: TruthLevel::Inferred,
            },
            SourceSiliconStep {
                layer: "SYMBOL".into(),
                value: "_ZN6parser10parse_node17h...".into(),
                truth: TruthLevel::Observed,
            },
            SourceSiliconStep {
                layer: "ASSEMBLY".into(),
                value: "mov rax,[rbx+0x8] // test rax,rax".into(),
                truth: TruthLevel::Sampled,
            },
            SourceSiliconStep {
                layer: "MACHINE BYTES".into(),
                value: "48 8B 43 08 48 85 C0".into(),
                truth: TruthLevel::Sampled,
            },
            SourceSiliconStep {
                layer: "THREAD".into(),
                value: "PID18472 / T18491".into(),
                truth: TruthLevel::Observed,
            },
            SourceSiliconStep {
                layer: "LOGICAL CPU".into(),
                value: "CPU13 / package 0".into(),
                truth: TruthLevel::Observed,
            },
            SourceSiliconStep {
                layer: "PMU".into(),
                value: "LLC miss + stalled cycles".into(),
                truth: TruthLevel::Sampled,
            },
            SourceSiliconStep {
                layer: "MEMORY".into(),
                value: if machine.cpu_packages.len() > 1 {
                    "NUMA node 1 // remote from execution node".into()
                } else {
                    "NUMA node 0 // local memory".into()
                },
                truth: TruthLevel::Inferred,
            },
        ];

        let anomalies = vec![
            AnomalyObservation {
                score: 0.91,
                title: "context-switch burst".into(),
                detail: "4.2× rolling baseline for this machine".into(),
                truth: TruthLevel::Inferred,
            },
            AnomalyObservation {
                score: if serverish { 0.83 } else { 0.42 },
                title: "remote NUMA traffic".into(),
                detail: "memory accesses crossing package boundary above baseline".into(),
                truth: TruthLevel::Inferred,
            },
            AnomalyObservation {
                score: 0.64,
                title: "network destination novelty".into(),
                detail: "one low-volume endpoint absent from the 7-day baseline".into(),
                truth: TruthLevel::Inferred,
            },
            AnomalyObservation {
                score: 0.38,
                title: "storage latency".into(),
                detail: "within expected envelope; no alert generated".into(),
                truth: TruthLevel::Inferred,
            },
        ];

        let incidents = vec![
            IncidentDossier {
                id: "INC-0042".into(),
                machine: machine.name.clone(),
                title: "latency spike".into(),
                age_seconds: 312.0,
                severity: "MEDIUM".into(),
                summary: "captured 60s before + 30s after event".into(),
            },
            IncidentDossier {
                id: "INC-0041".into(),
                machine: machine.name.clone(),
                title: "process crash".into(),
                age_seconds: 1842.0,
                severity: "HIGH".into(),
                summary: "autopsy + instruction + memory snapshot retained".into(),
            },
        ];

        let peer_names = fleet
            .machines
            .iter()
            .filter(|m| m.online)
            .map(|m| m.name.clone())
            .collect::<Vec<_>>();

        let peer_a = peer_names
            .get(0)
            .cloned()
            .unwrap_or_else(|| machine.name.clone());

        let peer_b = peer_names
            .get(1)
            .cloned()
            .unwrap_or_else(|| machine.name.clone());

        let cross_machine = vec![
            CrossMachineEvent {
                offset_ms: -42,
                machine: peer_a,
                event: "client request emitted".into(),
                truth: TruthLevel::Observed,
            },
            CrossMachineEvent {
                offset_ms: -18,
                machine: machine.name.clone(),
                event: "NIC receives request".into(),
                truth: TruthLevel::Observed,
            },
            CrossMachineEvent {
                offset_ms: -9,
                machine: machine.name.clone(),
                event: "web worker wakes".into(),
                truth: TruthLevel::Observed,
            },
            CrossMachineEvent {
                offset_ms: 0,
                machine: machine.name.clone(),
                event: "database lock wait begins".into(),
                truth: TruthLevel::Observed,
            },
            CrossMachineEvent {
                offset_ms: 84,
                machine: peer_b,
                event: "dependent service reports elevated latency".into(),
                truth: TruthLevel::Inferred,
            },
            CrossMachineEvent {
                offset_ms: 132,
                machine: machine.name.clone(),
                event: "request completes".into(),
                truth: TruthLevel::Observed,
            },
        ];

        Self {
            pcie_devices,
            numa_nodes,
            storage_io,
            thermal_zones,
            locks,
            cgroups,
            database,
            trace_spans,
            diff,
            firmware,
            security,
            source_silicon,
            anomalies,
            incidents,
            cross_machine,
        }
    }
}
