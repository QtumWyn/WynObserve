use serde::{Deserialize, Serialize};
use sysinfo::System;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
use linux::PlatformMemoryCollector;

#[cfg(target_os = "windows")]
use windows::PlatformMemoryCollector;

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
compile_error!("WynCommand memory telemetry currently supports Linux and Windows");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySnapshot {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,

    pub cached_bytes: u64,
    pub active_bytes: u64,
    pub dirty_bytes: u64,

    pub page_faults_per_second: u64,
    pub major_page_faults_per_second: u64,

    pub total_swap_bytes: u64,
    pub used_swap_bytes: u64,

    pub memory_detail_available: bool,
    pub fault_rates_available: bool,
}

impl MemorySnapshot {
    pub fn used_percent(&self) -> f32 {
        if self.total_bytes == 0 {
            return 0.0;
        }

        self.used_bytes as f32 / self.total_bytes as f32 * 100.0
    }

    pub fn swap_used_percent(&self) -> f32 {
        if self.total_swap_bytes == 0 {
            return 0.0;
        }

        self.used_swap_bytes as f32 / self.total_swap_bytes as f32 * 100.0
    }
}

#[derive(Debug)]
pub struct MemoryCollector {
    platform: PlatformMemoryCollector,
}

impl MemoryCollector {
    pub fn new() -> Self {
        Self {
            platform: PlatformMemoryCollector::new(),
        }
    }

    pub fn sample(&mut self, system: &System) -> MemorySnapshot {
        self.platform.sample(system)
    }
}

impl Default for MemoryCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::MemorySnapshot;

    #[test]
    fn calculates_memory_percentage() {
        let snapshot = MemorySnapshot {
            total_bytes: 100,
            used_bytes: 25,
            available_bytes: 75,
            cached_bytes: 10,
            active_bytes: 5,
            dirty_bytes: 0,
            page_faults_per_second: 0,
            major_page_faults_per_second: 0,
            total_swap_bytes: 0,
            used_swap_bytes: 0,
            memory_detail_available: true,
            fault_rates_available: true,
        };

        assert_eq!(snapshot.used_percent(), 25.0);
    }

    #[test]
    fn zero_total_memory_is_safe() {
        let snapshot = MemorySnapshot {
            total_bytes: 0,
            used_bytes: 0,
            available_bytes: 0,
            cached_bytes: 0,
            active_bytes: 0,
            dirty_bytes: 0,
            page_faults_per_second: 0,
            major_page_faults_per_second: 0,
            total_swap_bytes: 0,
            used_swap_bytes: 0,
            memory_detail_available: true,
            fault_rates_available: true,
        };

        assert_eq!(snapshot.used_percent(), 0.0);
        assert_eq!(snapshot.swap_used_percent(), 0.0);
    }
}
