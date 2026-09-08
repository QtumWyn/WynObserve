use std::{
    fs, io,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use super::NpuSnapshot;

const INTEL_VPU_DRIVER_PATH: &str = "/sys/bus/pci/drivers/intel_vpu";
const NPU_SAMPLE_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Debug)]
pub(super) struct PlatformNpuCollector {
    device_path: Option<PathBuf>,
    previous_busy_us: Option<u64>,
    previous_sample_at: Option<Instant>,
    last_utilization_percent: f32,
}

impl PlatformNpuCollector {
    pub(super) fn new() -> Self {
        Self {
            device_path: find_intel_npu(),
            previous_busy_us: None,
            previous_sample_at: None,
            last_utilization_percent: 0.0,
        }
    }

    pub(super) fn sample(&mut self) -> NpuSnapshot {
        let Some(device_path) = self.device_path.clone() else {
            return unavailable_snapshot();
        };

        let current_frequency_mhz =
            read_u64(&device_path.join("npu_current_frequency_mhz")).unwrap_or(0);

        let max_frequency_mhz = read_u64(&device_path.join("npu_max_frequency_mhz")).unwrap_or(0);

        let memory_used_bytes = read_u64(&device_path.join("npu_memory_utilization")).unwrap_or(0);

        let power_state =
            read_string(&device_path.join("power_state")).unwrap_or_else(|_| "unknown".to_string());

        let now = Instant::now();

        let should_refresh = self
            .previous_sample_at
            .map(|previous| now.duration_since(previous) >= NPU_SAMPLE_INTERVAL)
            .unwrap_or(true);

        let utilization_percent = if should_refresh {
            match read_u64(&device_path.join("npu_busy_time_us")) {
                Ok(busy_us) => self.update_utilization(busy_us, now),

                Err(_) => self.last_utilization_percent,
            }
        } else {
            self.last_utilization_percent
        };

        let pci_address = device_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown")
            .to_string();

        NpuSnapshot {
            available: true,

            model: "Intel NPU".to_string(),
            pci_address,

            utilization_percent,

            current_frequency_mhz,
            max_frequency_mhz,

            memory_used_bytes,

            power_state,
        }
    }

    fn update_utilization(&mut self, busy_us: u64, now: Instant) -> f32 {
        let (Some(previous_busy_us), Some(previous_sample_at)) =
            (self.previous_busy_us, self.previous_sample_at)
        else {
            self.previous_busy_us = Some(busy_us);
            self.previous_sample_at = Some(now);

            return 0.0;
        };

        let elapsed = now.duration_since(previous_sample_at);

        let elapsed_us = elapsed.as_micros() as u64;

        let busy_delta = busy_us.saturating_sub(previous_busy_us);

        let utilization = if elapsed_us == 0 {
            0.0
        } else {
            (busy_delta as f64 / elapsed_us as f64 * 100.0) as f32
        }
        .clamp(0.0, 100.0);

        self.previous_busy_us = Some(busy_us);

        self.previous_sample_at = Some(now);

        self.last_utilization_percent = utilization;

        utilization
    }
}

fn read_u64(path: &Path) -> io::Result<u64> {
    let contents = fs::read_to_string(path)?;

    contents
        .trim()
        .parse::<u64>()
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

fn find_intel_npu() -> Option<PathBuf> {
    let entries = fs::read_dir(INTEL_VPU_DRIVER_PATH).ok()?;

    for entry in entries {
        let Ok(entry) = entry else {
            continue;
        };

        let name = entry.file_name();
        let name = name.to_string_lossy();

        if looks_like_pci_address(&name) {
            return Some(entry.path());
        }
    }

    None
}

fn looks_like_pci_address(name: &str) -> bool {
    let mut parts = name.split(':');

    let Some(domain) = parts.next() else {
        return false;
    };

    let Some(bus) = parts.next() else {
        return false;
    };

    let Some(device_function) = parts.next() else {
        return false;
    };

    if parts.next().is_some() {
        return false;
    }

    domain.len() == 4 && bus.len() == 2 && device_function.contains('.')
}

fn read_string(path: &Path) -> io::Result<String> {
    Ok(fs::read_to_string(path)?.trim().to_string())
}

fn unavailable_snapshot() -> NpuSnapshot {
    NpuSnapshot {
        available: false,

        model: "NPU unavailable".to_string(),
        pci_address: String::new(),

        utilization_percent: 0.0,

        current_frequency_mhz: 0,
        max_frequency_mhz: 0,

        memory_used_bytes: 0,

        power_state: "unavailable".to_string(),
    }
}
