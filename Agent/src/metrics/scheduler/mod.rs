use serde::{Deserialize, Serialize};

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
use linux::PlatformSchedulerCollector;

#[cfg(target_os = "windows")]
use windows::PlatformSchedulerCollector;

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
compile_error!("WynCommand scheduler telemetry currently supports Linux and Windows");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerSnapshot {
    pub context_switches_per_second: u64,

    pub runnable_tasks: u64,
    pub blocked_tasks: u64,

    pub context_switch_rate_available: bool,
}

#[derive(Debug)]
pub struct SchedulerCollector {
    platform: PlatformSchedulerCollector,
}

impl SchedulerCollector {
    pub fn new() -> Self {
        Self {
            platform: PlatformSchedulerCollector::new(),
        }
    }

    pub fn sample(&mut self) -> SchedulerSnapshot {
        self.platform.sample()
    }
}

impl Default for SchedulerCollector {
    fn default() -> Self {
        Self::new()
    }
}
