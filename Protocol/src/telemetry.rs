use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetrySnapshot {
    pub schema_version: u16,
    pub captured_at_unix_ms: u64,

    pub cpu: CpuSnapshot,
    pub memory: MemorySnapshot,
    pub npu: NpuSnapshot,
    pub scheduler: SchedulerSnapshot,
    pub gpu: GpuSnapshot,
    pub storage: StorageSnapshot,
    pub network: NetworkSnapshot,

    pub processes_available: bool,
    pub process_count: usize,
    pub thread_count: usize,

    pub processes: Vec<ProcessSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuSnapshot {
    pub vendor: String,
    pub brand: String,

    pub global_usage_percent: f32,

    pub logical_cpu_count: usize,

    #[serde(default)]
    pub physical_core_count: Option<usize>,

    pub logical_cpus: Vec<LogicalCpuSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogicalCpuSnapshot {
    pub logical_id: usize,
    pub usage_percent: f32,
    pub frequency_mhz: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySnapshot {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,

    pub cached_bytes: u64,
    pub active_bytes: u64,
    pub dirty_bytes: u64,

    pub page_faults_per_second: u64,
    pub major_page_faults_per_second: u64,

    pub total_swap_bytes: u64,
    pub used_swap_bytes: u64,

    pub memory_detail_available: bool,
    pub fault_rates_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpuSnapshot {
    pub available: bool,

    pub model: String,
    pub pci_address: String,

    pub utilization_percent: f32,

    pub current_frequency_mhz: u64,
    pub max_frequency_mhz: u64,

    pub memory_used_bytes: u64,

    pub power_state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerSnapshot {
    pub context_switches_per_second: u64,

    pub runnable_tasks: u64,
    pub blocked_tasks: u64,

    pub context_switch_rate_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuSnapshot {
    pub available: bool,

    pub model: String,
    pub uuid: String,
    pub pci_address: String,

    pub utilization_percent: f32,
    pub memory_activity_percent: f32,

    pub vram_total_bytes: u64,
    pub vram_used_bytes: u64,

    pub temperature_c: f32,
    pub power_watts: f32,

    pub graphics_clock_mhz: Option<u32>,
    pub sm_clock_mhz: Option<u32>,
    pub memory_clock_mhz: Option<u32>,
    pub video_clock_mhz: Option<u32>,

    pub performance_state: Option<String>,
    pub fan_percent: Option<u32>,
    pub power_limit_watts: Option<f32>,

    pub pcie_rx_mib_s: Option<f32>,
    pub pcie_tx_mib_s: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageSnapshot {
    pub available: bool,

    pub device: String,
    pub model: String,

    pub read_mib_s: f32,
    pub write_mib_s: f32,

    pub read_iops: f32,
    pub write_iops: f32,

    pub utilization_percent: f32,

    pub io_in_progress: u64,
    pub average_queue_depth: f32,

    pub rates_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSnapshot {
    pub available: bool,

    pub interface: String,

    pub rx_mib_s: f32,
    pub tx_mib_s: f32,

    pub rx_packets_per_second: u64,
    pub tx_packets_per_second: u64,

    pub rx_errors_per_second: u64,
    pub tx_errors_per_second: u64,

    pub rx_drops_per_second: u64,
    pub tx_drops_per_second: u64,

    pub connections: usize,

    pub rates_available: bool,
    pub connections_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessSnapshot {
    pub pid: u32,
    pub parent_pid: u32,

    pub name: String,
    pub executable: Option<String>,
    pub state: String,

    pub cpu_usage_percent: f32,
    pub cpu_rate_available: bool,

    pub memory_bytes: u64,
    pub memory_available: bool,

    pub last_cpu: usize,
    pub threads: usize,

    pub read_mib_s: f32,
    pub write_mib_s: f32,

    pub io_rates_available: bool,

    pub started_at_unix_ms: Option<u64>,
}
