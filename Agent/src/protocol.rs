use wyn_protocol as wire;

use crate::snapshot::SystemSnapshot;

pub fn to_wire(snapshot: &SystemSnapshot) -> wire::TelemetrySnapshot {
    wire::TelemetrySnapshot {
        schema_version: wire::TELEMETRY_SCHEMA_VERSION,

        captured_at_unix_ms: snapshot.captured_at_unix_ms,

        cpu: wire::CpuSnapshot {
            vendor: snapshot.cpu.vendor.clone(),

            brand: snapshot.cpu.brand.clone(),

            global_usage_percent: snapshot.cpu.global_usage_percent,

            logical_cpu_count: snapshot.cpu.logical_cpu_count,

            physical_core_count: snapshot.cpu.physical_core_count,

            logical_cpus: snapshot
                .cpu
                .logical_cpus
                .iter()
                .map(|cpu| wire::LogicalCpuSnapshot {
                    logical_id: cpu.logical_id,

                    usage_percent: cpu.usage_percent,

                    frequency_mhz: cpu.frequency_mhz,
                })
                .collect(),
        },
        memory: wire::MemorySnapshot {
            total_bytes: snapshot.memory.total_bytes,

            used_bytes: snapshot.memory.used_bytes,

            available_bytes: snapshot.memory.available_bytes,

            cached_bytes: snapshot.memory.cached_bytes,

            active_bytes: snapshot.memory.active_bytes,

            dirty_bytes: snapshot.memory.dirty_bytes,

            page_faults_per_second: snapshot.memory.page_faults_per_second,

            major_page_faults_per_second: snapshot.memory.major_page_faults_per_second,

            total_swap_bytes: snapshot.memory.total_swap_bytes,

            used_swap_bytes: snapshot.memory.used_swap_bytes,

            memory_detail_available: snapshot.memory.memory_detail_available,

            fault_rates_available: snapshot.memory.fault_rates_available,
        },
        npu: wire::NpuSnapshot {
            available: snapshot.npu.available,

            model: snapshot.npu.model.clone(),

            pci_address: snapshot.npu.pci_address.clone(),

            utilization_percent: snapshot.npu.utilization_percent,

            current_frequency_mhz: snapshot.npu.current_frequency_mhz,

            max_frequency_mhz: snapshot.npu.max_frequency_mhz,

            memory_used_bytes: snapshot.npu.memory_used_bytes,

            power_state: snapshot.npu.power_state.clone(),
        },
        scheduler: wire::SchedulerSnapshot {
            context_switches_per_second: snapshot.scheduler.context_switches_per_second,

            runnable_tasks: snapshot.scheduler.runnable_tasks,

            blocked_tasks: snapshot.scheduler.blocked_tasks,

            context_switch_rate_available: snapshot.scheduler.context_switch_rate_available,
        },
        gpu: wire::GpuSnapshot {
            available: snapshot.gpu.available,

            model: snapshot.gpu.model.clone(),

            uuid: snapshot.gpu.uuid.clone(),

            pci_address: snapshot.gpu.pci_address.clone(),

            utilization_percent: snapshot.gpu.utilization_percent,

            memory_activity_percent: snapshot.gpu.memory_activity_percent,

            vram_total_bytes: snapshot.gpu.vram_total_bytes,

            vram_used_bytes: snapshot.gpu.vram_used_bytes,

            temperature_c: snapshot.gpu.temperature_c,

            power_watts: snapshot.gpu.power_watts,

            graphics_clock_mhz: snapshot.gpu.graphics_clock_mhz,

            sm_clock_mhz: snapshot.gpu.sm_clock_mhz,

            memory_clock_mhz: snapshot.gpu.memory_clock_mhz,

            video_clock_mhz: snapshot.gpu.video_clock_mhz,

            performance_state: snapshot.gpu.performance_state.clone(),

            fan_percent: snapshot.gpu.fan_percent,

            power_limit_watts: snapshot.gpu.power_limit_watts,

            pcie_rx_mib_s: snapshot.gpu.pcie_rx_mib_s,

            pcie_tx_mib_s: snapshot.gpu.pcie_tx_mib_s,
        },
        storage: wire::StorageSnapshot {
            available: snapshot.storage.available,

            device: snapshot.storage.device.clone(),

            model: snapshot.storage.model.clone(),

            read_mib_s: snapshot.storage.read_mib_s,

            write_mib_s: snapshot.storage.write_mib_s,

            read_iops: snapshot.storage.read_iops,

            write_iops: snapshot.storage.write_iops,

            utilization_percent: snapshot.storage.utilization_percent,

            io_in_progress: snapshot.storage.io_in_progress,

            average_queue_depth: snapshot.storage.average_queue_depth,

            rates_available: snapshot.storage.rates_available,
        },
        network: wire::NetworkSnapshot {
            available: snapshot.network.available,

            interface: snapshot.network.interface.clone(),

            rx_mib_s: snapshot.network.rx_mib_s,

            tx_mib_s: snapshot.network.tx_mib_s,

            rx_packets_per_second: snapshot.network.rx_packets_per_second,

            tx_packets_per_second: snapshot.network.tx_packets_per_second,

            rx_errors_per_second: snapshot.network.rx_errors_per_second,

            tx_errors_per_second: snapshot.network.tx_errors_per_second,

            rx_drops_per_second: snapshot.network.rx_drops_per_second,

            tx_drops_per_second: snapshot.network.tx_drops_per_second,

            connections: snapshot.network.connections,

            rates_available: snapshot.network.rates_available,

            connections_available: snapshot.network.connections_available,
        },
        processes_available: snapshot.processes_available,

        process_count: snapshot.process_count,

        thread_count: snapshot.thread_count,

        processes: snapshot
            .processes
            .iter()
            .map(|process| wire::ProcessSnapshot {
                pid: process.pid,

                parent_pid: process.parent_pid,

                name: process.name.clone(),

                executable: process.executable.clone(),

                state: process.state.clone(),

                cpu_usage_percent: process.cpu_usage_percent,

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
            .collect(),
    }
}
