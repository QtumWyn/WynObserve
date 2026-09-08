use super::NpuSnapshot;

#[derive(Debug, Default)]
pub(super) struct PlatformNpuCollector;

impl PlatformNpuCollector {
    pub(super) fn new() -> Self {
        Self
    }

    pub(super) fn sample(&mut self) -> NpuSnapshot {
        NpuSnapshot {
            available: false,
            model: String::new(),
            pci_address: String::new(),
            utilization_percent: 0.0,
            current_frequency_mhz: 0,
            max_frequency_mhz: 0,
            memory_used_bytes: 0,
            power_state: "unavailable".into(),
        }
    }
}
