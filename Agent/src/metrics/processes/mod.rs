use serde::{Deserialize, Serialize};

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
use linux::PlatformProcessCollector;

#[cfg(target_os = "windows")]
use windows::PlatformProcessCollector;

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
compile_error!("WynCommand process telemetry currently supports Linux and Windows");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessSnapshot {
    pub pid: u32,
    pub parent_pid: u32,

    pub name: String,
    pub executable: Option<String>,
    pub state: String,

    pub cpu_usage_percent: f32,
    pub cpu_rate_available: bool,

    pub memory_bytes: u64,
    pub memory_available: bool,

    pub last_cpu: usize,
    pub threads: usize,

    pub read_mib_s: f32,
    pub write_mib_s: f32,
    pub io_rates_available: bool,

    pub started_at_unix_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessCollectionSnapshot {
    pub available: bool,

    pub total_processes: usize,
    pub total_threads: usize,

    pub processes: Vec<ProcessSnapshot>,
}

#[derive(Debug)]
pub struct ProcessCollector {
    platform: PlatformProcessCollector,
}

impl ProcessCollector {
    pub fn new() -> Self {
        Self {
            platform: PlatformProcessCollector::new(),
        }
    }

    pub fn sample(&mut self) -> ProcessCollectionSnapshot {
        self.platform.sample()
    }
}

impl Default for ProcessCollector {
    fn default() -> Self {
        Self::new()
    }
}
