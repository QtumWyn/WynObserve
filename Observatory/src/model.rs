#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TruthLevel {
    Observed,
    Sampled,
    Inferred,
    Schematic,
}

impl TruthLevel {
    pub const fn glyph(self) -> &'static str {
        match self {
            Self::Observed => "●",
            Self::Sampled => "◉",
            Self::Inferred => "≈",
            Self::Schematic => "◇",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Observed => "OBSERVED",
            Self::Sampled => "SAMPLED",
            Self::Inferred => "INFERRED",
            Self::Schematic => "SCHEMATIC",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentId {
    Cpu,
    Memory,
    Gpu,
    Npu,
    Storage,
    Network,
}

impl ComponentId {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Cpu => "CPU",
            Self::Memory => "MEMORY",
            Self::Gpu => "GPU",
            Self::Npu => "NPU",
            Self::Storage => "NVMe",
            Self::Network => "NETWORK",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstructionArchitecture {
    X86_64,
    NvidiaSass,
    Unknown,
}

impl InstructionArchitecture {
    pub const fn label(self) -> &'static str {
        match self {
            Self::X86_64 => "x86-64",
            Self::NvidiaSass => "NVIDIA SASS",
            Self::Unknown => "unknown ISA",
        }
    }
}

#[derive(Debug, Clone)]
pub struct InstructionSample {
    /// Monotonically increasing identifier for the sample stream.
    pub sequence: u64,

    /// How old this sample is relative to the newest sample in the snapshot.
    pub age_seconds: f32,

    /// Activity context used by the UI to filter the stream.
    ///
    /// IMPORTANT: this is not a claim that the instruction physically "lives"
    /// inside the selected component. For example, a GPU-tagged x86-64 sample
    /// may be host code submitting work to the GPU.
    pub component: ComponentId,

    /// Provenance of the instruction information.
    pub truth: TruthLevel,

    pub pid: u32,
    pub tid: u32,
    pub process_name: String,

    /// Logical CPU where the sampled thread was observed, when known.
    pub cpu_id: Option<usize>,

    pub architecture: InstructionArchitecture,
    pub address: u64,

    /// Raw instruction bytes when the source provides them.
    pub bytes: Vec<u8>,

    pub mnemonic: String,
    pub operands: String,

    /// Optional explanation of how this instruction relates to the selected
    /// component or how the sample was obtained.
    pub note: Option<String>,
}

impl InstructionSample {
    pub fn bytes_hex(&self) -> String {
        self.bytes
            .iter()
            .map(|byte| format!("{byte:02X}"))
            .collect::<Vec<_>>()
            .join(" ")
    }

    pub fn assembly(&self) -> String {
        if self.operands.is_empty() {
            self.mnemonic.clone()
        } else {
            format!("{} {}", self.mnemonic, self.operands)
        }
    }
}

#[derive(Debug, Clone)]
pub struct CpuCoreSnapshot {
    pub logical_id: usize,
    pub usage: f32,
    pub frequency_mhz: u32,
    pub temperature_c: f32,
    pub process: String,
}

#[derive(Debug, Clone)]
pub struct CpuSnapshot {
    pub model: String,
    pub package_usage: f32,
    pub package_temperature_c: f32,
    pub logical_cpus: Vec<CpuCoreSnapshot>,
    pub context_switches_per_second: u64,
    pub migrations_per_second: u64,
}

#[derive(Debug, Clone)]
pub struct MemorySnapshot {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub cached_bytes: u64,
    pub active_bytes: u64,
    pub dirty_bytes: u64,
    pub page_faults_per_second: u64,
}

#[derive(Debug, Clone)]
pub struct GpuSnapshot {
    pub available: bool,

    pub model: String,
    pub uuid: String,
    pub pci_address: String,

    pub utilization: f32,
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

impl GpuSnapshot {
    pub fn vram_used_percent(&self) -> f32 {
        if self.vram_total_bytes == 0 {
            return 0.0;
        }

        self.vram_used_bytes as f32 / self.vram_total_bytes as f32 * 100.0
    }
}

#[derive(Debug, Clone)]
pub struct NpuSnapshot {
    pub available: bool,
    pub model: String,
    pub pci_address: String,
    pub utilization: f32,
    pub current_frequency_mhz: u64,
    pub max_frequency_mhz: u64,
    pub memory_used_bytes: u64,
    pub power_state: String,
}

#[derive(Debug, Clone)]
pub struct StorageSnapshot {
    pub available: bool,

    pub device: String,
    pub model: String,

    pub read_mib_s: f32,
    pub write_mib_s: f32,

    pub read_iops: f32,
    pub write_iops: f32,

    pub utilization: f32,

    pub io_in_progress: u64,
    pub average_queue_depth: f32,

    pub rates_available: bool,
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct ProcessSnapshot {
    pub pid: u32,
    pub parent_pid: u32,

    pub name: String,
    pub executable: Option<String>,
    pub state: String,

    pub cpu_usage: f32,
    pub cpu_rate_available: bool,

    pub memory_bytes: u64,
    pub memory_available: bool,

    /// Logical CPU on which the process was last observed executing.
    pub last_cpu: usize,
    pub threads: usize,

    pub read_mib_s: f32,
    pub write_mib_s: f32,
    pub io_rates_available: bool,

    pub started_at_unix_ms: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct SystemEvent {
    pub age_seconds: f32,
    pub truth: TruthLevel,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct SystemSnapshot {
    pub cpu: CpuSnapshot,
    pub memory: MemorySnapshot,
    pub gpu: GpuSnapshot,
    pub npu: NpuSnapshot,
    pub storage: StorageSnapshot,
    pub network: NetworkSnapshot,

    pub processes_available: bool,
    pub process_count: usize,
    pub thread_count: usize,
    pub processes: Vec<ProcessSnapshot>,

    pub events: Vec<SystemEvent>,

    /// Slow-drip machine-code / disassembly samples.
    ///
    /// The mock backend populates this now. The real backend can later fill
    /// the same structure from an instruction sampler, profiler, or hardware
    /// trace decoder.
    pub instruction_samples: Vec<InstructionSample>,
}

pub trait TelemetrySource {
    fn poll(&mut self, elapsed_seconds: f64) -> SystemSnapshot;
}

pub fn format_bytes(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * 1024.0;
    const GIB: f64 = MIB * 1024.0;
    const TIB: f64 = GIB * 1024.0;

    let value = bytes as f64;

    if value >= TIB {
        format!("{:.2} TiB", value / TIB)
    } else if value >= GIB {
        format!("{:.2} GiB", value / GIB)
    } else if value >= MIB {
        format!("{:.2} MiB", value / MIB)
    } else if value >= KIB {
        format!("{:.2} KiB", value / KIB)
    } else {
        format!("{bytes} B")
    }
}
