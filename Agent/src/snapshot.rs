use crate::metrics::{
    cpu::CpuSnapshot, gpu::GpuSnapshot, memory::MemorySnapshot, network::NetworkSnapshot,
    npu::NpuSnapshot, processes::ProcessSnapshot, scheduler::SchedulerSnapshot,
    storage::StorageSnapshot,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemSnapshot {
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
