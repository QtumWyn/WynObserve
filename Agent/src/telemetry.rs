use std::{
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use sysinfo::{CpuRefreshKind, System};

use crate::{
    cpu_identity::{CpuIdentity, read_cpu_identity},
    metrics::{
        cpu::collect_cpu,
        gpu::GpuCollector,
        memory::{MemoryCollector, MemorySnapshot},
        network::{NetworkCollector, NetworkSnapshot},
        npu::{NpuCollector, NpuSnapshot},
        processes::{ProcessCollector, ProcessSnapshot},
        scheduler::{SchedulerCollector, SchedulerSnapshot},
        storage::{StorageCollector, StorageSnapshot},
    },
    snapshot::SystemSnapshot,
};

fn unix_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub struct TelemetryCollector {
    system: System,
    cpu_identity: CpuIdentity,
    memory: MemoryCollector,
    npu: NpuCollector,
    scheduler: SchedulerCollector,
    gpu: GpuCollector,
    storage: StorageCollector,
    network: NetworkCollector,
    processes: ProcessCollector,
}

impl TelemetryCollector {
    pub fn new() -> Self {
        let cpu_identity = read_cpu_identity();
        let memory = MemoryCollector::new();
        let npu = NpuCollector::new();
        let scheduler = SchedulerCollector::new();
        let gpu = GpuCollector::new();
        let storage = StorageCollector::new();
        let network = NetworkCollector::new();
        let processes = ProcessCollector::new();

        let mut system = System::new();

        // Build the CPU list and take the first CPU sample.
        //
        // sysinfo calculates CPU usage from the difference between two samples,
        // so the first reading is not yet useful by itself.
        system.refresh_cpu_list(CpuRefreshKind::everything());
        system.refresh_cpu_all();
        system.refresh_memory();

        // One short startup delay gives us a meaningful second CPU sample.
        //
        // Later, once Observatory has a continuously running event loop, we can
        // remove this startup sleep and simply let the next scheduled refresh
        // become the second sample.
        thread::sleep(Duration::from_millis(250));

        system.refresh_cpu_all();

        Self {
            system,
            cpu_identity,
            memory,
            npu,
            scheduler,
            gpu,
            storage,
            network,
            processes,
        }
    }

    pub fn sample(&mut self) -> SystemSnapshot {
        self.system.refresh_cpu_all();
        self.system.refresh_memory();
        let scheduler = self.scheduler.sample();
        let storage = self.storage.sample();

        let network = self.network.sample();
        let process_sample = self.processes.sample();

        SystemSnapshot {
            schema_version: 1,
            captured_at_unix_ms: unix_timestamp_ms(),

            cpu: collect_cpu(&self.system, &self.cpu_identity),

            memory: self.memory.sample(&self.system),

            npu: self.npu.sample(),

            scheduler,
            gpu: self.gpu.sample(),
            storage,
            network,
            processes_available: process_sample.available,
            process_count: process_sample.total_processes,
            thread_count: process_sample.total_threads,
            processes: process_sample.processes,
        }
    }
}

impl Default for TelemetryCollector {
    fn default() -> Self {
        Self::new()
    }
}
