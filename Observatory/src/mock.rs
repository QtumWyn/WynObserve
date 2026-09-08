use crate::model::{
    ComponentId, CpuCoreSnapshot, CpuSnapshot, GpuSnapshot, InstructionArchitecture,
    InstructionSample, MemorySnapshot, NetworkSnapshot, NpuSnapshot, ProcessSnapshot,
    StorageSnapshot, SystemEvent, SystemSnapshot, TelemetrySource, TruthLevel,
};

#[derive(Default)]
pub struct MockTelemetry {
    frame: u64,
}

impl MockTelemetry {
    fn wave(t: f64, offset: f64, speed: f64, min: f32, max: f32) -> f32 {
        let normalized = ((t * speed + offset).sin() * 0.5 + 0.5) as f32;
        min + (max - min) * normalized
    }

    fn instruction_samples(t: f64) -> Vec<InstructionSample> {
        struct Template {
            component: ComponentId,
            process: &'static str,
            pid: u32,
            tid: u32,
            bytes: &'static [u8],
            mnemonic: &'static str,
            operands: &'static str,
            note: &'static str,
        }

        // These are intentionally plausible mock samples, not claims about the
        // actual instructions executing on the host running this prototype.
        let templates = [
            Template {
                component: ComponentId::Cpu,
                process: "rustc",
                pid: 18472,
                tid: 18491,
                bytes: &[0x48, 0x8B, 0x43, 0x08],
                mnemonic: "mov",
                operands: "rax, [rbx+0x8]",
                note: "mock CPU execution sample",
            },
            Template {
                component: ComponentId::Cpu,
                process: "rustc",
                pid: 18472,
                tid: 18491,
                bytes: &[0x48, 0x85, 0xC0],
                mnemonic: "test",
                operands: "rax, rax",
                note: "mock CPU execution sample",
            },
            Template {
                component: ComponentId::Cpu,
                process: "cargo",
                pid: 18430,
                tid: 18430,
                bytes: &[0x75, 0xE9],
                mnemonic: "jne",
                operands: "0x7F23A90E",
                note: "mock branch sample",
            },
            Template {
                component: ComponentId::Cpu,
                process: "wyn-observatory",
                pid: 18801,
                tid: 18808,
                bytes: &[0x48, 0x83, 0xEC, 0x20],
                mnemonic: "sub",
                operands: "rsp, 0x20",
                note: "mock stack-frame activity",
            },
            Template {
                component: ComponentId::Memory,
                process: "firefox",
                pid: 7291,
                tid: 7332,
                bytes: &[0x48, 0x8B, 0x04, 0x24],
                mnemonic: "mov",
                operands: "rax, [rsp]",
                note: "mock load classified as memory-relevant",
            },
            Template {
                component: ComponentId::Memory,
                process: "rustc",
                pid: 18472,
                tid: 18495,
                bytes: &[0x48, 0x89, 0x45, 0xE8],
                mnemonic: "mov",
                operands: "[rbp-0x18], rax",
                note: "mock store classified as memory-relevant",
            },
            Template {
                component: ComponentId::Memory,
                process: "rustc",
                pid: 18472,
                tid: 18495,
                bytes: &[0x0F, 0x18, 0x08],
                mnemonic: "prefetcht0",
                operands: "[rax]",
                note: "mock prefetch sample",
            },
            Template {
                component: ComponentId::Memory,
                process: "kwin_wayland",
                pid: 2203,
                tid: 2240,
                bytes: &[0x48, 0x8D, 0x54, 0x24, 0x20],
                mnemonic: "lea",
                operands: "rdx, [rsp+0x20]",
                note: "mock address-generation sample",
            },
            Template {
                component: ComponentId::Gpu,
                process: "kwin_wayland",
                pid: 2203,
                tid: 2244,
                bytes: &[0xFF, 0x15, 0x62, 0x31, 0x00, 0x00],
                mnemonic: "call",
                operands: "qword ptr [rip+0x3162]",
                note: "mock x86-64 host-side GPU submission path; not GPU SASS",
            },
            Template {
                component: ComponentId::Gpu,
                process: "firefox",
                pid: 7291,
                tid: 7368,
                bytes: &[0x48, 0x89, 0xDF],
                mnemonic: "mov",
                operands: "rdi, rbx",
                note: "mock host-side graphics call preparation",
            },
            Template {
                component: ComponentId::Gpu,
                process: "kwin_wayland",
                pid: 2203,
                tid: 2244,
                bytes: &[0xE8, 0x41, 0x02, 0x00, 0x00],
                mnemonic: "call",
                operands: "0x7F23B1A0",
                note: "mock host call correlated with GPU activity",
            },
            Template {
                component: ComponentId::Gpu,
                process: "wyn-observatory",
                pid: 18801,
                tid: 18806,
                bytes: &[0x0F, 0x28, 0xC1],
                mnemonic: "movaps",
                operands: "xmm0, xmm1",
                note: "mock renderer-side SIMD activity",
            },
            Template {
                component: ComponentId::Storage,
                process: "rustc",
                pid: 18472,
                tid: 18472,
                bytes: &[0x0F, 0x05],
                mnemonic: "syscall",
                operands: "",
                note: "mock syscall correlated with storage I/O",
            },
            Template {
                component: ComponentId::Storage,
                process: "cargo",
                pid: 18430,
                tid: 18430,
                bytes: &[0x48, 0x89, 0xC7],
                mnemonic: "mov",
                operands: "rdi, rax",
                note: "mock file-I/O call preparation",
            },
            Template {
                component: ComponentId::Storage,
                process: "rustc",
                pid: 18472,
                tid: 18498,
                bytes: &[0x48, 0x8B, 0x55, 0xF0],
                mnemonic: "mov",
                operands: "rdx, [rbp-0x10]",
                note: "mock buffer preparation near storage activity",
            },
            Template {
                component: ComponentId::Storage,
                process: "cargo",
                pid: 18430,
                tid: 18430,
                bytes: &[0xE8, 0x88, 0x01, 0x00, 0x00],
                mnemonic: "call",
                operands: "0x7F23C040",
                note: "mock call correlated with executable write",
            },
            Template {
                component: ComponentId::Network,
                process: "firefox",
                pid: 7291,
                tid: 7351,
                bytes: &[0x0F, 0x05],
                mnemonic: "syscall",
                operands: "",
                note: "mock syscall correlated with socket activity",
            },
            Template {
                component: ComponentId::Network,
                process: "firefox",
                pid: 7291,
                tid: 7351,
                bytes: &[0x48, 0x89, 0xD6],
                mnemonic: "mov",
                operands: "rsi, rdx",
                note: "mock network-buffer call preparation",
            },
            Template {
                component: ComponentId::Network,
                process: "pipewire",
                pid: 3118,
                tid: 3155,
                bytes: &[0x48, 0x8B, 0x7D, 0xE0],
                mnemonic: "mov",
                operands: "rdi, [rbp-0x20]",
                note: "mock socket/buffer processing sample",
            },
            Template {
                component: ComponentId::Network,
                process: "firefox",
                pid: 7291,
                tid: 7351,
                bytes: &[0x85, 0xC0],
                mnemonic: "test",
                operands: "eax, eax",
                note: "mock return-value check after network activity",
            },
        ];

        // The mock stream advances a few times per second, giving the right-hand
        // feed a slow, readable drip instead of replacing the whole list at 60 FPS.
        let newest_sequence = 50_000 + (t * 3.5).floor() as u64;

        (0..28)
            .map(|offset| {
                let sequence = newest_sequence.saturating_sub(offset as u64);
                let template = &templates[(sequence as usize) % templates.len()];

                InstructionSample {
                    sequence,
                    age_seconds: offset as f32 / 3.5,
                    component: template.component,
                    truth: TruthLevel::Sampled,
                    pid: template.pid,
                    tid: template.tid,
                    process_name: template.process.to_string(),
                    cpu_id: Some(((sequence as usize) + template.tid as usize) % 16),
                    architecture: InstructionArchitecture::X86_64,
                    address: 0x0000_7F23_A900 + ((sequence % 0x300) * 4),
                    bytes: template.bytes.to_vec(),
                    mnemonic: template.mnemonic.to_string(),
                    operands: template.operands.to_string(),
                    note: Some(template.note.to_string()),
                }
            })
            .collect()
    }
}

impl TelemetrySource for MockTelemetry {
    fn poll(&mut self, t: f64) -> SystemSnapshot {
        self.frame += 1;

        let names = [
            "rustc",
            "firefox",
            "kwin_wayland",
            "plasmashell",
            "cargo",
            "pipewire",
            "konsole",
            "idle",
        ];

        let logical_cpus = (0..16)
            .map(|i| {
                let usage = Self::wave(t, i as f64 * 0.71, 0.9 + (i % 4) as f64 * 0.12, 2.0, 96.0);

                CpuCoreSnapshot {
                    logical_id: i,
                    usage,
                    frequency_mhz: (3100.0 + usage * 17.0) as u32,
                    temperature_c: 39.0 + usage * 0.31,
                    process: names[(i + ((t / 2.2) as usize)) % names.len()].to_string(),
                }
            })
            .collect::<Vec<_>>();

        let package_usage =
            logical_cpus.iter().map(|c| c.usage).sum::<f32>() / logical_cpus.len() as f32;

        let total_ram = 32_u64 * 1024 * 1024 * 1024;
        let used_ratio = Self::wave(t, 0.5, 0.18, 0.39, 0.72) as f64;
        let used_ram = (total_ram as f64 * used_ratio) as u64;
        let cached = (total_ram as f64 * 0.17) as u64;
        let active = (used_ram as f64 * 0.61) as u64;

        let gpu_usage = Self::wave(t, 2.0, 0.47, 4.0, 86.0);
        let vram_total = 12_u64 * 1024 * 1024 * 1024;
        let vram_used = (vram_total as f64 * (0.24 + gpu_usage as f64 / 180.0)) as u64;

        let processes = vec![
            ProcessSnapshot {
                pid: 18472,
                parent_pid: 18430,
                name: "rustc".into(),
                executable: Some("/usr/bin/rustc".into()),
                state: "running".into(),
                cpu_usage: Self::wave(t, 0.0, 0.83, 18.0, 145.0),
                cpu_rate_available: true,
                memory_bytes: 2_160_000_000,
                memory_available: true,
                last_cpu: ((t * 1.7) as usize + 3) % 16,
                threads: 18,
                read_mib_s: Self::wave(t, 0.2, 0.37, 0.0, 18.0),
                write_mib_s: Self::wave(t, 0.7, 0.41, 0.0, 32.0),
                io_rates_available: true,
                started_at_unix_ms: None,
            },
            ProcessSnapshot {
                pid: 7291,
                parent_pid: 1,
                name: "firefox".into(),
                executable: Some("/usr/lib/firefox/firefox".into()),
                state: "sleeping".into(),
                cpu_usage: Self::wave(t, 1.0, 0.34, 2.0, 36.0),
                cpu_rate_available: true,
                memory_bytes: 3_840_000_000,
                memory_available: true,
                last_cpu: ((t * 0.8) as usize + 7) % 16,
                threads: 42,
                read_mib_s: Self::wave(t, 1.1, 0.29, 0.0, 6.0),
                write_mib_s: Self::wave(t, 2.3, 0.33, 0.0, 4.0),
                io_rates_available: true,
                started_at_unix_ms: None,
            },
            ProcessSnapshot {
                pid: 2203,
                parent_pid: 1,
                name: "kwin_wayland".into(),
                executable: Some("/usr/bin/kwin_wayland".into()),
                state: "sleeping".into(),
                cpu_usage: Self::wave(t, 4.0, 0.73, 1.0, 18.0),
                cpu_rate_available: true,
                memory_bytes: 615_000_000,
                memory_available: true,
                last_cpu: ((t * 1.1) as usize + 1) % 16,
                threads: 14,
                read_mib_s: 0.0,
                write_mib_s: 0.0,
                io_rates_available: true,
                started_at_unix_ms: None,
            },
            ProcessSnapshot {
                pid: 18801,
                parent_pid: 1,
                name: "wyn-observatory".into(),
                executable: Some("/opt/wyncommand/wyn-observatory".into()),
                state: "running".into(),
                cpu_usage: Self::wave(t, 3.1, 0.55, 1.0, 9.0),
                cpu_rate_available: true,
                memory_bytes: 188_000_000,
                memory_available: true,
                last_cpu: ((t * 0.5) as usize + 11) % 16,
                threads: 7,
                read_mib_s: 0.0,
                write_mib_s: Self::wave(t, 0.8, 0.27, 0.0, 1.5),
                io_rates_available: true,
                started_at_unix_ms: None,
            },
        ];

        let process_count = processes.len();
        let thread_count = processes.iter().map(|process| process.threads).sum();

        let migration_cpu = ((t * 1.8) as usize) % 16;
        let migration_to = (migration_cpu + 5) % 16;

        let events = vec![
            SystemEvent {
                age_seconds: 0.0,
                truth: TruthLevel::Observed,
                message: format!(
                    "thread 18491 migrated CPU{migration_cpu:02} → CPU{migration_to:02}"
                ),
            },
            SystemEvent {
                age_seconds: 0.4,
                truth: TruthLevel::Sampled,
                message: format!("CPU package utilization {:.1}%", package_usage),
            },
            SystemEvent {
                age_seconds: 0.8,
                truth: TruthLevel::Observed,
                message: "major page fault // rustc".into(),
            },
            SystemEvent {
                age_seconds: 1.2,
                truth: TruthLevel::Inferred,
                message: "storage → RAM transfer burst".into(),
            },
            SystemEvent {
                age_seconds: 1.6,
                truth: TruthLevel::Schematic,
                message: "logical topology visualization active".into(),
            },
        ];

        SystemSnapshot {
            cpu: CpuSnapshot {
                model: "Demo CPU // logical topology".into(),
                package_usage,
                package_temperature_c: 42.0 + package_usage * 0.24,
                logical_cpus,
                context_switches_per_second: 18_000 + (package_usage * 310.0) as u64,
                migrations_per_second: 470 + (package_usage * 11.0) as u64,
            },
            memory: MemorySnapshot {
                total_bytes: total_ram,
                used_bytes: used_ram,
                available_bytes: total_ram.saturating_sub(used_ram),
                cached_bytes: cached,
                active_bytes: active,
                dirty_bytes: (Self::wave(t, 0.0, 0.3, 12.0, 410.0) as u64) * 1024 * 1024,
                page_faults_per_second: 240 + (Self::wave(t, 0.0, 0.7, 0.0, 900.0) as u64),
            },
            gpu: GpuSnapshot {
                available: true,
                model: "NVIDIA RTX // demo".into(),
                uuid: "GPU-DEMO-0000".into(),
                pci_address: "00000000:01:00.0".into(),
                utilization: gpu_usage,
                memory_activity_percent: Self::wave(t, 1.2, 0.43, 1.0, 72.0),
                vram_total_bytes: vram_total,
                vram_used_bytes: vram_used,
                temperature_c: 38.0 + gpu_usage * 0.42,
                power_watts: 25.0 + gpu_usage * 2.1,
                graphics_clock_mhz: Some((300.0 + gpu_usage * 24.0) as u32),
                sm_clock_mhz: Some((300.0 + gpu_usage * 24.0) as u32),
                memory_clock_mhz: Some(7001),
                video_clock_mhz: Some(1065),
                performance_state: Some(if gpu_usage > 55.0 { "P0" } else { "P8" }.into()),
                fan_percent: Some((30.0 + gpu_usage * 0.5).clamp(30.0, 100.0) as u32),
                power_limit_watts: Some(250.0),
                pcie_rx_mib_s: Some(Self::wave(t, 1.0, 0.8, 4.0, 860.0)),
                pcie_tx_mib_s: Some(Self::wave(t, 5.0, 0.61, 2.0, 540.0)),
            },
            npu: NpuSnapshot {
                available: true,
                model: "Intel NPU // demo".into(),
                pci_address: "0000:00:0b.0".into(),
                utilization: Self::wave(t, 1.4, 0.31, 0.0, 72.0),
                current_frequency_mhz: Self::wave(t, 1.4, 0.31, 400.0, 1600.0) as u64,
                max_frequency_mhz: 1600,
                memory_used_bytes: 68 * 1024 * 1024,
                power_state: "D0".into(),
            },
            storage: StorageSnapshot {
                available: true,
                device: "nvme0n1".into(),
                model: "NVMe0 // demo".into(),
                read_mib_s: Self::wave(t, 2.4, 0.56, 0.0, 1250.0),
                write_mib_s: Self::wave(t, 0.4, 0.73, 0.0, 780.0),
                read_iops: Self::wave(t, 1.2, 0.64, 20.0, 12_000.0),
                write_iops: Self::wave(t, 2.1, 0.58, 10.0, 7_500.0),
                utilization: Self::wave(t, 0.0, 0.42, 3.0, 81.0),
                io_in_progress: Self::wave(t, 0.8, 0.77, 0.0, 9.0) as u64,
                average_queue_depth: Self::wave(t, 1.6, 0.49, 0.0, 4.8),
                rates_available: true,
            },
            network: NetworkSnapshot {
                available: true,
                interface: "eth0".into(),
                rx_mib_s: Self::wave(t, 0.2, 0.66, 0.1, 42.0),
                tx_mib_s: Self::wave(t, 3.0, 0.52, 0.1, 18.0),
                rx_packets_per_second: Self::wave(t, 0.5, 0.72, 100.0, 48_000.0) as u64,
                tx_packets_per_second: Self::wave(t, 1.7, 0.61, 80.0, 22_000.0) as u64,
                rx_errors_per_second: 0,
                tx_errors_per_second: 0,
                rx_drops_per_second: Self::wave(t, 2.8, 0.27, 0.0, 3.0) as u64,
                tx_drops_per_second: 0,
                connections: 34 + ((t.sin() * 4.0).abs() as usize),
                rates_available: true,
                connections_available: true,
            },
            processes_available: true,
            process_count,
            thread_count,
            processes,

            events,
            instruction_samples: Self::instruction_samples(t),
        }
    }
}
