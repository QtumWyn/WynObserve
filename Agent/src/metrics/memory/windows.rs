use sysinfo::System;

use super::MemorySnapshot;

#[derive(Debug, Default)]
pub(super) struct PlatformMemoryCollector;

impl PlatformMemoryCollector {
    pub(super) fn new() -> Self {
        Self
    }

    pub(super) fn sample(&mut self, system: &System) -> MemorySnapshot {
        MemorySnapshot {
            total_bytes: system.total_memory(),
            used_bytes: system.used_memory(),
            available_bytes: system.available_memory(),
            cached_bytes: 0,
            active_bytes: 0,
            dirty_bytes: 0,
            page_faults_per_second: 0,
            major_page_faults_per_second: 0,
            total_swap_bytes: system.total_swap(),
            used_swap_bytes: system.used_swap(),
            memory_detail_available: false,
            fault_rates_available: false,
        }
    }
}
