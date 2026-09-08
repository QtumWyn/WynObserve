use serde::{Deserialize, Serialize};

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
use linux::PlatformNpuCollector;

#[cfg(target_os = "windows")]
use windows::PlatformNpuCollector;

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
compile_error!("WynCommand NPU telemetry currently supports Linux and Windows");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpuSnapshot {
    pub available: bool,
    pub model: String,
    pub pci_address: String,
    pub utilization_percent: f32,
    pub current_frequency_mhz: u64,
    pub max_frequency_mhz: u64,
    pub memory_used_bytes: u64,
    pub power_state: String,
}

#[derive(Debug)]
pub struct NpuCollector {
    platform: PlatformNpuCollector,
}

impl NpuCollector {
    pub fn new() -> Self {
        Self {
            platform: PlatformNpuCollector::new(),
        }
    }

    pub fn sample(&mut self) -> NpuSnapshot {
        self.platform.sample()
    }
}

impl Default for NpuCollector {
    fn default() -> Self {
        Self::new()
    }
}
