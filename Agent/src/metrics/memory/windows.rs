use super::MemorySnapshot;
use crate::windows::Counters;
use sysinfo::System;

#[derive(Debug)]
pub(super) struct PlatformMemoryCollector {
    counters: Option<Counters>,
}
impl PlatformMemoryCollector {
    pub(super) fn new() -> Self {
        Self {
            counters: Counters::new(&[
                r"\Memory\Cache Bytes",
                r"\Memory\Modified Page List Bytes",
                r"\Memory\Page Faults/sec",
            ]),
        }
    }
    pub(super) fn sample(&mut self, system: &System) -> MemorySnapshot {
        let values = self
            .counters
            .as_mut()
            .map(|q| q.sample())
            .unwrap_or_else(|| vec![None; 3]);
        MemorySnapshot {
            total_bytes: system.total_memory(),
            used_bytes: system.used_memory(),
            available_bytes: system.available_memory(),
            cached_bytes: values[0].unwrap_or(0.0) as u64,
            // Windows does not expose Linux Active/Dirty semantics. The modified list is
            // dirty physical memory; active_bytes remains unavailable, not inferred.
            active_bytes: 0,
            dirty_bytes: values[1].unwrap_or(0.0) as u64,
            page_faults_per_second: values[2].unwrap_or(0.0).round() as u64,
            // Pages Input/sec is pages read, NOT a count of major faults. Do not relabel it.
            major_page_faults_per_second: 0,
            total_swap_bytes: system.total_swap(),
            used_swap_bytes: system.used_swap(),
            memory_detail_available: false,
            fault_rates_available: values[2].is_some(),
            major_fault_rate_available: false,
        }
    }
}
