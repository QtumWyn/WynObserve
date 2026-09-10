use crate::model::{
    CpuCoreSnapshot, CpuSnapshot, GpuSnapshot, MemorySnapshot, NetworkSnapshot, NpuSnapshot,
    ProcessSnapshot, StorageSnapshot, SystemEvent, SystemSnapshot, TruthLevel,
};
use wyn_protocol::TelemetrySnapshot;

pub fn telemetry_to_system(wire: TelemetrySnapshot) -> SystemSnapshot {
    let processes = wire
        .processes
        .into_iter()
        .map(|process| ProcessSnapshot {
            pid: process.pid,
            parent_pid: process.parent_pid,

            name: process.name,
            executable: process.executable,
            state: process.state,

            cpu_usage: process.cpu_usage_percent,
            cpu_rate_available: process.cpu_rate_available,

            memory_bytes: process.memory_bytes,
            memory_available: process.memory_available,

            last_cpu: process.last_cpu,
            threads: process.threads,

            read_mib_s: process.read_mib_s,
            write_mib_s: process.write_mib_s,
            io_rates_available: process.io_rates_available,

            started_at_unix_ms: process.started_at_unix_ms,
        })
        .collect::<Vec<_>>();

    let logical_cpus = wire
        .cpu
        .logical_cpus
        .into_iter()
        .map(|cpu| {
            // M6's process collector reports the logical CPU on which each
            // exported process last executed. Because the backend sorts
            // exported processes by CPU usage, the first match is a useful
            // sampled attribution hint, not a claim of current ownership.
            let process = processes
                .iter()
                .find(|process| process.last_cpu == cpu.logical_id && process.cpu_rate_available)
                .map(|process| process.name.clone())
                .unwrap_or_else(|| "unattributed".to_string());

            CpuCoreSnapshot {
                logical_id: cpu.logical_id,
                usage: cpu.usage_percent,

                frequency_mhz: u32::try_from(cpu.frequency_mhz).unwrap_or(u32::MAX),

                // Awaiting a real per-core thermal collector.
                temperature_c: 0.0,
                process,
            }
        })
        .collect();

    SystemSnapshot {
        cpu: CpuSnapshot {
            model: wire.cpu.brand,

            package_usage: wire.cpu.global_usage_percent,

            // Awaiting thermal collector.
            package_temperature_c: 0.0,

            logical_cpus,

            context_switches_per_second: wire.scheduler.context_switches_per_second,
            migrations_per_second: 0,
        },

        memory: MemorySnapshot {
            total_bytes: wire.memory.total_bytes,
            used_bytes: wire.memory.used_bytes,
            available_bytes: wire.memory.available_bytes,

            cached_bytes: wire.memory.cached_bytes,
            active_bytes: wire.memory.active_bytes,
            dirty_bytes: wire.memory.dirty_bytes,
            page_faults_per_second: wire.memory.page_faults_per_second,
        },

        gpu: GpuSnapshot {
            available: wire.gpu.available,
            model: wire.gpu.model,
            uuid: wire.gpu.uuid,
            pci_address: wire.gpu.pci_address,
            utilization: wire.gpu.utilization_percent,
            memory_activity_percent: wire.gpu.memory_activity_percent,
            vram_total_bytes: wire.gpu.vram_total_bytes,
            vram_used_bytes: wire.gpu.vram_used_bytes,
            temperature_c: wire.gpu.temperature_c,
            power_watts: wire.gpu.power_watts,
            graphics_clock_mhz: wire.gpu.graphics_clock_mhz,
            sm_clock_mhz: wire.gpu.sm_clock_mhz,
            memory_clock_mhz: wire.gpu.memory_clock_mhz,
            video_clock_mhz: wire.gpu.video_clock_mhz,
            performance_state: wire.gpu.performance_state,
            fan_percent: wire.gpu.fan_percent,
            power_limit_watts: wire.gpu.power_limit_watts,
            pcie_rx_mib_s: wire.gpu.pcie_rx_mib_s,
            pcie_tx_mib_s: wire.gpu.pcie_tx_mib_s,
        },

        npu: NpuSnapshot {
            available: wire.npu.available,
            model: wire.npu.model,
            pci_address: wire.npu.pci_address,
            utilization: wire.npu.utilization_percent,
            current_frequency_mhz: wire.npu.current_frequency_mhz,
            max_frequency_mhz: wire.npu.max_frequency_mhz,
            memory_used_bytes: wire.npu.memory_used_bytes,
            power_state: wire.npu.power_state,
        },

        storage: StorageSnapshot {
            available: wire.storage.available,
            device: wire.storage.device,
            model: wire.storage.model,
            read_mib_s: wire.storage.read_mib_s,
            write_mib_s: wire.storage.write_mib_s,
            read_iops: wire.storage.read_iops,
            write_iops: wire.storage.write_iops,
            utilization: wire.storage.utilization_percent,
            io_in_progress: wire.storage.io_in_progress,
            average_queue_depth: wire.storage.average_queue_depth,
            rates_available: wire.storage.rates_available,
            space_available: wire.storage.space_available,

            mount_point: wire.storage.mount_point,

            capacity_bytes: wire.storage.capacity_bytes,

            used_bytes: wire.storage.used_bytes,

            available_bytes: wire.storage.available_bytes,
        },

        network: NetworkSnapshot {
            available: wire.network.available,
            interface: wire.network.interface,
            rx_mib_s: wire.network.rx_mib_s,
            tx_mib_s: wire.network.tx_mib_s,
            rx_packets_per_second: wire.network.rx_packets_per_second,
            tx_packets_per_second: wire.network.tx_packets_per_second,
            rx_errors_per_second: wire.network.rx_errors_per_second,
            tx_errors_per_second: wire.network.tx_errors_per_second,
            rx_drops_per_second: wire.network.rx_drops_per_second,
            tx_drops_per_second: wire.network.tx_drops_per_second,
            connections: wire.network.connections,
            rates_available: wire.network.rates_available,
            connections_available: wire.network.connections_available,
        },

        processes_available: wire.processes_available,
        process_count: wire.process_count,
        thread_count: wire.thread_count,
        processes,

        events: vec![SystemEvent {
            age_seconds: 0.0,
            truth: TruthLevel::Observed,
            message: format!(
                "M6 telemetry received // snapshot {}",
                wire.captured_at_unix_ms
            ),
        }],

        instruction_samples: Vec::new(),
    }
}
