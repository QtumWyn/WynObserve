use serde::{Deserialize, Serialize};
use sysinfo::System;

use crate::cpu_identity::CpuIdentity;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogicalCpuSnapshot {
    pub logical_id: usize,
    pub usage_percent: f32,
    pub frequency_mhz: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuSnapshot {
    pub vendor: String,
    pub brand: String,

    pub global_usage_percent: f32,
    pub logical_cpu_count: usize,
    pub physical_core_count: Option<usize>,
    pub logical_cpus: Vec<LogicalCpuSnapshot>,
}

pub fn collect_cpu(system: &System, identity: &CpuIdentity) -> CpuSnapshot {
    let logical_cpus = system
        .cpus()
        .iter()
        .enumerate()
        .map(|(logical_id, cpu)| LogicalCpuSnapshot {
            logical_id,
            usage_percent: cpu.cpu_usage(),
            frequency_mhz: cpu.frequency(),
        })
        .collect::<Vec<_>>();

    CpuSnapshot {
        vendor: identity.vendor.clone(),
        brand: identity.brand.clone(),

        global_usage_percent: system.global_cpu_usage(),
        logical_cpu_count: logical_cpus.len(),
        physical_core_count: System::physical_core_count(),
        logical_cpus,
    }
}
