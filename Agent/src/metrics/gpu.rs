use nvml_wrapper::{
    Nvml,
    enum_wrappers::device::{Clock, PcieUtilCounter, TemperatureSensor},
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuSnapshot {
    pub available: bool,

    pub model: String,
    pub uuid: String,
    pub pci_address: String,

    pub utilization_percent: f32,
    pub memory_activity_percent: f32,

    pub vram_total_bytes: u64,
    pub vram_used_bytes: u64,

    pub temperature_c: f32,
    pub power_watts: f32,

    pub graphics_clock_mhz: Option<u32>,
    pub sm_clock_mhz: Option<u32>,
    pub memory_clock_mhz: Option<u32>,
    pub video_clock_mhz: Option<u32>,

    pub performance_state: Option<String>,

    pub fan_percent: Option<u32>,
    pub power_limit_watts: Option<f32>,

    pub pcie_rx_mib_s: Option<f32>,
    pub pcie_tx_mib_s: Option<f32>,
}

impl GpuSnapshot {
    pub fn vram_used_percent(&self) -> f32 {
        if self.vram_total_bytes == 0 {
            return 0.0;
        }

        self.vram_used_bytes as f32 / self.vram_total_bytes as f32 * 100.0
    }
}

pub struct GpuCollector {
    nvml: Option<Nvml>,
}

impl GpuCollector {
    pub fn new() -> Self {
        Self {
            nvml: Nvml::init().ok(),
        }
    }

    pub fn sample(&self) -> GpuSnapshot {
        let Some(nvml) = &self.nvml else {
            return unavailable_snapshot();
        };
        let Ok(device) = nvml.device_by_index(0) else {
            return unavailable_snapshot();
        };
        let model = device
            .name()
            .unwrap_or_else(|_| "Unknown NVIDIA GPU".to_string());

        let uuid = device.uuid().unwrap_or_default();

        let pci_address = device
            .pci_info()
            .map(|info| info.bus_id)
            .unwrap_or_default();

        let graphics_clock_mhz = device.clock_info(Clock::Graphics).ok();

        let sm_clock_mhz = device.clock_info(Clock::SM).ok();

        let memory_clock_mhz = device.clock_info(Clock::Memory).ok();

        let video_clock_mhz = device.clock_info(Clock::Video).ok();

        let performance_state = device
            .performance_state()
            .map(|state| format!("{state:?}"))
            .ok();

        let utilization = device.utilization_rates().ok();

        let memory = device.memory_info().ok();

        let temperature_c = device
            .temperature(TemperatureSensor::Gpu)
            .map(|value| value as f32)
            .unwrap_or(0.0);

        let power_watts = device
            .power_usage()
            .map(|milliwatts| milliwatts as f32 / 1000.0)
            .unwrap_or(0.0);

        let power_limit_watts = device
            .enforced_power_limit()
            .map(|milliwatts| milliwatts as f32 / 1000.0)
            .ok();

        let utilization_percent = utilization
            .as_ref()
            .map(|value| value.gpu as f32)
            .unwrap_or(0.0);

        let memory_activity_percent = utilization
            .as_ref()
            .map(|value| value.memory as f32)
            .unwrap_or(0.0);

        let vram_total_bytes = memory.as_ref().map(|value| value.total).unwrap_or(0);

        let vram_used_bytes = memory.as_ref().map(|value| value.used).unwrap_or(0);

        let fan_percent = device.fan_speed(0).ok();

        let pcie_tx_mib_s = device
            .pcie_throughput(PcieUtilCounter::Send)
            .map(|kib_per_second| kib_per_second as f32 / 1024.0)
            .ok();

        let pcie_rx_mib_s = device
            .pcie_throughput(PcieUtilCounter::Receive)
            .map(|kib_per_second| kib_per_second as f32 / 1024.0)
            .ok();

        GpuSnapshot {
            available: true,

            model,
            uuid,
            pci_address,

            utilization_percent,
            memory_activity_percent,

            vram_total_bytes,
            vram_used_bytes,

            temperature_c,
            power_watts,

            graphics_clock_mhz,
            sm_clock_mhz,
            memory_clock_mhz,
            video_clock_mhz,

            performance_state,

            fan_percent,
            power_limit_watts,

            pcie_rx_mib_s,
            pcie_tx_mib_s,
        }
    }
}

impl Default for GpuCollector {
    fn default() -> Self {
        Self::new()
    }
}

fn unavailable_snapshot() -> GpuSnapshot {
    GpuSnapshot {
        available: false,

        model: "GPU unavailable".into(),
        uuid: String::new(),
        pci_address: String::new(),

        utilization_percent: 0.0,
        memory_activity_percent: 0.0,

        vram_total_bytes: 0,
        vram_used_bytes: 0,

        temperature_c: 0.0,
        power_watts: 0.0,

        graphics_clock_mhz: None,
        sm_clock_mhz: None,
        memory_clock_mhz: None,
        video_clock_mhz: None,

        performance_state: None,

        fan_percent: None,
        power_limit_watts: None,

        pcie_rx_mib_s: None,
        pcie_tx_mib_s: None,
    }
}
