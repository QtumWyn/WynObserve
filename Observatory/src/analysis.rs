use crate::model::{SystemSnapshot, TruthLevel};

#[derive(Debug, Clone)]
pub struct TimelinePoint {
    pub age_seconds: f32,
    pub cpu_percent: f32,
    pub memory_percent: f32,
    pub network_mib_s: f32,
    pub event: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CausalStep {
    pub title: String,
    pub detail: String,
    pub truth: TruthLevel,
}

#[derive(Debug, Clone)]
pub struct SyscallStat {
    pub name: String,
    pub calls_per_second: u64,
    pub avg_latency_us: f32,
    pub errors_per_second: u64,
}

#[derive(Debug, Clone)]
pub struct FlameFrame {
    pub label: String,
    pub depth: usize,
    pub start: f32,
    pub width: f32,
    pub samples: u64,
}

#[derive(Debug, Clone)]
pub struct MemoryRegion {
    pub start: u64,
    pub end: u64,
    pub label: String,
    pub permissions: String,
    pub resident_percent: f32,
    pub dirty_percent: f32,
}

#[derive(Debug, Clone)]
pub struct SchedulerCpu {
    pub cpu: usize,
    pub run_queue: usize,
    pub wakeups_per_second: u64,
    pub migrations_per_second: u64,
    pub pressure: f32,
}

#[derive(Debug, Clone)]
pub struct CacheSnapshot {
    pub ipc: f32,
    pub cycles_per_second: f64,
    pub l1d_miss_percent: f32,
    pub l1i_miss_percent: f32,
    pub l2_miss_percent: f32,
    pub llc_miss_percent: f32,
    pub branch_miss_percent: f32,
    pub stalled_cycle_percent: f32,
}

#[derive(Debug, Clone)]
pub struct IrqStat {
    pub irq: String,
    pub source: String,
    pub cpu: usize,
    pub interrupts_per_second: u64,
}

#[derive(Debug, Clone)]
pub struct BinarySection {
    pub name: String,
    pub size_bytes: u64,
    pub flags: String,
}

#[derive(Debug, Clone)]
pub struct BinarySnapshot {
    pub path: String,
    pub format: String,
    pub architecture: String,
    pub entry_point: u64,
    pub build_id: String,
    pub sections: Vec<BinarySection>,
    pub libraries: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct AutopsyEvent {
    pub age_seconds: f32,
    pub message: String,
    pub truth: TruthLevel,
}

#[derive(Debug, Clone)]
pub struct AutopsySnapshot {
    pub process: String,
    pub pid: u32,
    pub exit_reason: String,
    pub peak_memory_bytes: u64,
    pub last_cpu: usize,
    pub last_instruction: String,
    pub events: Vec<AutopsyEvent>,
}

#[derive(Debug, Clone)]
pub struct ConnectionFlow {
    pub process: String,
    pub protocol: String,
    pub remote: String,
    pub rx_mib_s: f32,
    pub tx_mib_s: f32,
    pub latency_ms: f32,
}

#[derive(Debug, Clone)]
pub struct FleetMachine {
    pub name: String,
    pub role: String,
    pub online: bool,
    pub cpu_percent: f32,
    pub memory_percent: f32,
    pub network_mib_s: f32,
    pub note: String,
}

#[derive(Debug, Clone)]
pub struct ServerService {
    pub name: String,
    pub status: String,
    pub requests_per_second: f32,
    pub latency_ms: f32,
    pub error_percent: f32,
}

#[derive(Debug, Clone)]
pub struct ServerContainer {
    pub name: String,
    pub cpu_percent: f32,
    pub memory_bytes: u64,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct ServerSnapshot {
    pub hostname: String,
    pub role: String,
    pub requests_per_second: f32,
    pub active_connections: usize,
    pub error_rate_percent: f32,
    pub p95_latency_ms: f32,
    pub services: Vec<ServerService>,
    pub containers: Vec<ServerContainer>,
}

#[derive(Debug, Clone)]
pub struct AnalysisSnapshot {
    pub timeline: Vec<TimelinePoint>,
    pub causal_chain: Vec<CausalStep>,
    pub syscalls: Vec<SyscallStat>,
    pub flamegraph: Vec<FlameFrame>,
    pub memory_regions: Vec<MemoryRegion>,
    pub scheduler: Vec<SchedulerCpu>,
    pub cache: CacheSnapshot,
    pub irqs: Vec<IrqStat>,
    pub binary: BinarySnapshot,
    pub autopsy: AutopsySnapshot,
    pub flows: Vec<ConnectionFlow>,
    pub fleet: Vec<FleetMachine>,
    pub server: ServerSnapshot,
}

impl AnalysisSnapshot {
    /// Pure mock data for dormant UI work.
    ///
    /// The backend can later replace this with normalized collectors without
    /// requiring the visualizations themselves to be redesigned.
    pub fn mock(system: &SystemSnapshot, t: f32) -> Self {
        let mem_pct =
            system.memory.used_bytes as f32 / system.memory.total_bytes.max(1) as f32 * 100.0;

        let timeline = (0..48)
            .map(|i| {
                let age = (47 - i) as f32 * 0.75;
                let phase = t - age;
                let event = match i {
                    10 => Some("thread migration burst".to_string()),
                    21 => Some("major page fault".to_string()),
                    33 => Some("NVMe read spike".to_string()),
                    41 => Some("network ingress burst".to_string()),
                    _ => None,
                };

                TimelinePoint {
                    age_seconds: age,
                    cpu_percent: (system.cpu.package_usage + (phase * 0.47).sin() * 17.0)
                        .clamp(0.0, 100.0),
                    memory_percent: (mem_pct + (phase * 0.13).sin() * 3.5).clamp(0.0, 100.0),
                    network_mib_s: (system.network.rx_mib_s + system.network.tx_mib_s)
                        * (0.62 + (phase * 0.31).sin().abs() * 0.75),
                    event,
                }
            })
            .collect();

        let causal_chain = vec![
            CausalStep {
                title: "THREAD WAKEUP".into(),
                detail: "rustc/T18491 becomes runnable".into(),
                truth: TruthLevel::Observed,
            },
            CausalStep {
                title: "SCHEDULER DISPATCH".into(),
                detail: "thread selected for CPU07".into(),
                truth: TruthLevel::Observed,
            },
            CausalStep {
                title: "MAJOR PAGE FAULT".into(),
                detail: "requested page is not resident".into(),
                truth: TruthLevel::Observed,
            },
            CausalStep {
                title: "NVMe READ".into(),
                detail: "backing page fetched from storage".into(),
                truth: TruthLevel::Inferred,
            },
            CausalStep {
                title: "PAGE RESIDENT".into(),
                detail: "page enters working set".into(),
                truth: TruthLevel::Inferred,
            },
            CausalStep {
                title: "THREAD RESUME".into(),
                detail: "execution continues on CPU13 after migration".into(),
                truth: TruthLevel::Observed,
            },
        ];

        let syscalls = vec![
            SyscallStat {
                name: "futex".into(),
                calls_per_second: 3481,
                avg_latency_us: 4.2,
                errors_per_second: 0,
            },
            SyscallStat {
                name: "read".into(),
                calls_per_second: 1210,
                avg_latency_us: 18.6,
                errors_per_second: 2,
            },
            SyscallStat {
                name: "write".into(),
                calls_per_second: 991,
                avg_latency_us: 12.4,
                errors_per_second: 0,
            },
            SyscallStat {
                name: "mmap".into(),
                calls_per_second: 188,
                avg_latency_us: 31.1,
                errors_per_second: 0,
            },
            SyscallStat {
                name: "epoll_wait".into(),
                calls_per_second: 670,
                avg_latency_us: 112.0,
                errors_per_second: 0,
            },
            SyscallStat {
                name: "io_uring_enter".into(),
                calls_per_second: 84,
                avg_latency_us: 8.8,
                errors_per_second: 0,
            },
        ];

        let flamegraph = vec![
            FlameFrame {
                label: "rustc".into(),
                depth: 0,
                start: 0.00,
                width: 1.00,
                samples: 10000,
            },
            FlameFrame {
                label: "rustc_driver".into(),
                depth: 1,
                start: 0.02,
                width: 0.72,
                samples: 7200,
            },
            FlameFrame {
                label: "query_system".into(),
                depth: 2,
                start: 0.04,
                width: 0.42,
                samples: 4200,
            },
            FlameFrame {
                label: "typeck".into(),
                depth: 3,
                start: 0.06,
                width: 0.21,
                samples: 2100,
            },
            FlameFrame {
                label: "mir".into(),
                depth: 3,
                start: 0.28,
                width: 0.17,
                samples: 1700,
            },
            FlameFrame {
                label: "codegen".into(),
                depth: 2,
                start: 0.48,
                width: 0.24,
                samples: 2400,
            },
            FlameFrame {
                label: "LLVM".into(),
                depth: 3,
                start: 0.50,
                width: 0.20,
                samples: 2000,
            },
            FlameFrame {
                label: "allocator".into(),
                depth: 1,
                start: 0.76,
                width: 0.13,
                samples: 1300,
            },
            FlameFrame {
                label: "kernel".into(),
                depth: 1,
                start: 0.90,
                width: 0.08,
                samples: 800,
            },
        ];

        let memory_regions = vec![
            MemoryRegion {
                start: 0x0040_0000,
                end: 0x0069_0000,
                label: "ELF .text".into(),
                permissions: "r-xp".into(),
                resident_percent: 96.0,
                dirty_percent: 0.0,
            },
            MemoryRegion {
                start: 0x0069_0000,
                end: 0x0071_0000,
                label: "ELF data".into(),
                permissions: "rw-p".into(),
                resident_percent: 84.0,
                dirty_percent: 18.0,
            },
            MemoryRegion {
                start: 0x5555_6000,
                end: 0x5B20_0000,
                label: "[heap]".into(),
                permissions: "rw-p".into(),
                resident_percent: 71.0,
                dirty_percent: 52.0,
            },
            MemoryRegion {
                start: 0x7F11_0000,
                end: 0x7F55_0000,
                label: "libLLVM.so".into(),
                permissions: "r-xp".into(),
                resident_percent: 61.0,
                dirty_percent: 0.0,
            },
            MemoryRegion {
                start: 0x7F70_0000,
                end: 0x7F80_0000,
                label: "anonymous".into(),
                permissions: "rw-p".into(),
                resident_percent: 43.0,
                dirty_percent: 31.0,
            },
            MemoryRegion {
                start: 0x7FFF_0000,
                end: 0x7FFF_F000,
                label: "[stack]".into(),
                permissions: "rw-p".into(),
                resident_percent: 92.0,
                dirty_percent: 65.0,
            },
        ];

        let scheduler = (0..system.cpu.logical_cpus.len().min(20))
            .map(|cpu| {
                let pressure = ((t * 0.62 + cpu as f32 * 0.41).sin() * 0.5 + 0.5) * 100.0;
                SchedulerCpu {
                    cpu,
                    run_queue: 1 + ((pressure / 24.0) as usize),
                    wakeups_per_second: 180 + (pressure * 13.0) as u64,
                    migrations_per_second: 11 + (pressure * 1.9) as u64,
                    pressure,
                }
            })
            .collect();

        let cache = CacheSnapshot {
            ipc: 1.82 + (t * 0.2).sin() * 0.22,
            cycles_per_second: 61_000_000_000.0,
            l1d_miss_percent: 2.8,
            l1i_miss_percent: 0.7,
            l2_miss_percent: 8.4,
            llc_miss_percent: 17.9,
            branch_miss_percent: 3.2,
            stalled_cycle_percent: 21.4,
        };

        let irqs = vec![
            IrqStat {
                irq: "124".into(),
                source: "nvme0q0".into(),
                cpu: 4,
                interrupts_per_second: 1840,
            },
            IrqStat {
                irq: "125".into(),
                source: "nvme0q1".into(),
                cpu: 6,
                interrupts_per_second: 1211,
            },
            IrqStat {
                irq: "142".into(),
                source: "nvidia".into(),
                cpu: 12,
                interrupts_per_second: 663,
            },
            IrqStat {
                irq: "158".into(),
                source: "eth0-rx".into(),
                cpu: 8,
                interrupts_per_second: 2894,
            },
            IrqStat {
                irq: "NMI".into(),
                source: "performance".into(),
                cpu: 0,
                interrupts_per_second: 82,
            },
        ];

        let binary = BinarySnapshot {
            path: "/usr/bin/rustc".into(),
            format: "ELF64".into(),
            architecture: "x86-64".into(),
            entry_point: 0x0000_0000_0040_11C0,
            build_id: "mock-1d9b0d4217a9".into(),
            sections: vec![
                BinarySection {
                    name: ".text".into(),
                    size_bytes: 2_846_112,
                    flags: "AX".into(),
                },
                BinarySection {
                    name: ".rodata".into(),
                    size_bytes: 881_664,
                    flags: "A".into(),
                },
                BinarySection {
                    name: ".data".into(),
                    size_bytes: 149_504,
                    flags: "WA".into(),
                },
                BinarySection {
                    name: ".bss".into(),
                    size_bytes: 286_720,
                    flags: "WA".into(),
                },
                BinarySection {
                    name: ".eh_frame".into(),
                    size_bytes: 203_840,
                    flags: "A".into(),
                },
            ],
            libraries: vec![
                "libLLVM.so".into(),
                "libstdc++.so.6".into(),
                "libgcc_s.so.1".into(),
                "libc.so.6".into(),
                "libm.so.6".into(),
            ],
        };

        let autopsy = AutopsySnapshot {
            process: "example-worker".into(),
            pid: 22119,
            exit_reason: "SIGSEGV // mock capture".into(),
            peak_memory_bytes: 1_842_000_000,
            last_cpu: 13,
            last_instruction: "mov rax, [rbx+0x8]".into(),
            events: vec![
                AutopsyEvent {
                    age_seconds: 3.2,
                    message: "RSS climbed +184 MiB".into(),
                    truth: TruthLevel::Sampled,
                },
                AutopsyEvent {
                    age_seconds: 1.4,
                    message: "major page fault burst".into(),
                    truth: TruthLevel::Observed,
                },
                AutopsyEvent {
                    age_seconds: 0.8,
                    message: "thread migrated CPU07 → CPU13".into(),
                    truth: TruthLevel::Observed,
                },
                AutopsyEvent {
                    age_seconds: 0.1,
                    message: "invalid memory access inferred near sampled RIP".into(),
                    truth: TruthLevel::Inferred,
                },
                AutopsyEvent {
                    age_seconds: 0.0,
                    message: "SIGSEGV received // process exited".into(),
                    truth: TruthLevel::Observed,
                },
            ],
        };

        let flows = vec![
            ConnectionFlow {
                process: "firefox".into(),
                protocol: "TCP/TLS".into(),
                remote: "142.250.x.x:443".into(),
                rx_mib_s: 5.1,
                tx_mib_s: 0.8,
                latency_ms: 22.0,
            },
            ConnectionFlow {
                process: "wyn-observatory".into(),
                protocol: "QUIC".into(),
                remote: "server-agent:7443".into(),
                rx_mib_s: 1.8,
                tx_mib_s: 0.4,
                latency_ms: 4.8,
            },
            ConnectionFlow {
                process: "postgres".into(),
                protocol: "TCP".into(),
                remote: "10.0.0.14:53122".into(),
                rx_mib_s: 0.7,
                tx_mib_s: 1.2,
                latency_ms: 1.4,
            },
            ConnectionFlow {
                process: "containerd".into(),
                protocol: "TCP".into(),
                remote: "registry:443".into(),
                rx_mib_s: 0.4,
                tx_mib_s: 0.1,
                latency_ms: 18.9,
            },
        ];

        let fleet = Vec::new();

        let server = ServerSnapshot {
            hostname: "aplus-physical-01".into(),
            role: "application + database + container host".into(),
            requests_per_second: 1248.0 + (t * 0.42).sin() * 330.0,
            active_connections: 382 + ((t * 0.8).sin().abs() * 90.0) as usize,
            error_rate_percent: 0.17 + (t * 0.17).sin().abs() * 0.12,
            p95_latency_ms: 28.4 + (t * 0.31).sin().abs() * 11.0,
            services: vec![
                ServerService {
                    name: "web".into(),
                    status: "HEALTHY".into(),
                    requests_per_second: 823.0,
                    latency_ms: 18.4,
                    error_percent: 0.08,
                },
                ServerService {
                    name: "api".into(),
                    status: "HEALTHY".into(),
                    requests_per_second: 351.0,
                    latency_ms: 31.2,
                    error_percent: 0.21,
                },
                ServerService {
                    name: "postgres".into(),
                    status: "HEALTHY".into(),
                    requests_per_second: 188.0,
                    latency_ms: 4.8,
                    error_percent: 0.00,
                },
                ServerService {
                    name: "scheduler".into(),
                    status: "HEALTHY".into(),
                    requests_per_second: 24.0,
                    latency_ms: 11.7,
                    error_percent: 0.04,
                },
            ],
            containers: vec![
                ServerContainer {
                    name: "web-prod".into(),
                    cpu_percent: 22.1,
                    memory_bytes: 1_284_000_000,
                    status: "running".into(),
                },
                ServerContainer {
                    name: "worker-prod".into(),
                    cpu_percent: 18.7,
                    memory_bytes: 982_000_000,
                    status: "running".into(),
                },
                ServerContainer {
                    name: "observatory-agent".into(),
                    cpu_percent: 1.4,
                    memory_bytes: 124_000_000,
                    status: "running".into(),
                },
            ],
        };

        Self {
            timeline,
            causal_chain,
            syscalls,
            flamegraph,
            memory_regions,
            scheduler,
            cache,
            irqs,
            binary,
            autopsy,
            flows,
            fleet,
            server,
        }
    }
}
