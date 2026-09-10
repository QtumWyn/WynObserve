use serde::{Deserialize, Serialize};

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
use linux::PlatformStorageCollector;

#[cfg(target_os = "windows")]
use windows::PlatformStorageCollector;

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
compile_error!("WynCommand storage telemetry currently supports Linux and Windows");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageSnapshot {
    pub available: bool,

    pub device: String,
    pub model: String,

    /*
     * Filesystem capacity telemetry.
     *
     * For now this represents the root
     * filesystem mounted at "/".
     */
    pub space_available: bool,

    pub mount_point: String,

    pub capacity_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,

    pub read_mib_s: f32,
    pub write_mib_s: f32,

    pub read_iops: f32,
    pub write_iops: f32,

    pub utilization_percent: f32,

    pub io_in_progress: u64,
    pub average_queue_depth: f32,

    pub rates_available: bool,
}

#[derive(Debug)]
pub struct StorageCollector {
    platform: PlatformStorageCollector,
}

impl StorageCollector {
    pub fn new() -> Self {
        Self {
            platform: PlatformStorageCollector::new(),
        }
    }

    pub fn sample(&mut self) -> StorageSnapshot {
        self.platform.sample()
    }
}

impl Default for StorageCollector {
    fn default() -> Self {
        Self::new()
    }
}
