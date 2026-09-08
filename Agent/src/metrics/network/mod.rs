use serde::{Deserialize, Serialize};

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
use linux::PlatformNetworkCollector;

#[cfg(target_os = "windows")]
use windows::PlatformNetworkCollector;

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
compile_error!("WynCommand network telemetry currently supports Linux and Windows");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSnapshot {
    pub available: bool,
    pub interface: String,

    pub rx_mib_s: f32,
    pub tx_mib_s: f32,

    pub rx_packets_per_second: u64,
    pub tx_packets_per_second: u64,

    pub rx_errors_per_second: u64,
    pub tx_errors_per_second: u64,

    pub rx_drops_per_second: u64,
    pub tx_drops_per_second: u64,

    pub connections: usize,

    pub rates_available: bool,
    pub connections_available: bool,
}

#[derive(Debug)]
pub struct NetworkCollector {
    platform: PlatformNetworkCollector,
}

impl NetworkCollector {
    pub fn new() -> Self {
        Self {
            platform: PlatformNetworkCollector::new(),
        }
    }

    pub fn sample(&mut self) -> NetworkSnapshot {
        self.platform.sample()
    }
}

impl Default for NetworkCollector {
    fn default() -> Self {
        Self::new()
    }
}
