//! Small, platform-independent decoder for AMD's public ADL PMLog ABI.
//! Sensor IDs and units: GPUOpen display-library/include/adl_defines.h.
use wyn_protocol::{CpuTemperatureSnapshot, GpuAdapterTelemetry};

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub(super) struct Sensor {
    pub supported: i32,
    pub value: i32,
}
#[repr(C)]
pub(super) struct PmLog {
    pub size: i32,
    pub sensors: [Sensor; 256],
}
impl Default for PmLog {
    fn default() -> Self {
        Self {
            size: std::mem::size_of::<Self>() as i32,
            sensors: [Sensor::default(); 256],
        }
    }
}
impl PmLog {
    fn value(&self, index: usize, min: i32, max: i32) -> Option<f32> {
        let sensor = self.sensors.get(index)?;
        (sensor.supported != 0 && (min..=max).contains(&sensor.value))
            .then_some(sensor.value as f32)
    }
    pub(super) fn cpu_temperature(&self) -> Option<CpuTemperatureSnapshot> {
        // Never substitute a GPU, SoC, motherboard or ACPI zone temperature.
        self.value(32, 1, 125)
            .map(|celsius| CpuTemperatureSnapshot {
                celsius,
                source: "AMD ADL · CPU temperature".into(),
            })
    }
    pub(super) fn gpu(&self) -> GpuAdapterTelemetry {
        let temperature = [(8, "GPU edge"), (28, "GPU graphics"), (27, "GPU hotspot")]
            .into_iter()
            .find_map(|(id, label)| self.value(id, 1, 150).map(|v| (v, label)));
        GpuAdapterTelemetry {
            source: "AMD ADL PMLog".into(),
            utilization_percent: self.value(19, 0, 100),
            temperature_c: temperature.map(|t| t.0),
            temperature_source: temperature.map(|t| t.1.into()),
            graphics_clock_mhz: self.value(1, 0, 10000).map(|v| v as u32),
            power_watts: self.value(23, 0, 2000),
            memory_used_bytes: None,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn supported_idle_differs_from_missing_and_cpu_is_never_gpu() {
        let mut data = PmLog::default();
        assert!(data.gpu().utilization_percent.is_none());
        data.sensors[19] = Sensor {
            supported: 1,
            value: 0,
        };
        data.sensors[8] = Sensor {
            supported: 1,
            value: 42,
        };
        assert_eq!(data.gpu().utilization_percent, Some(0.0));
        assert_eq!(data.gpu().temperature_c, Some(42.0));
        assert!(data.cpu_temperature().is_none());
        data.sensors[32] = Sensor {
            supported: 1,
            value: 61,
        };
        assert_eq!(data.cpu_temperature().unwrap().celsius, 61.0);
        data.sensors[32].supported = 0;
        assert!(data.cpu_temperature().is_none());
    }
    #[test]
    fn driver_sentinels_and_fresh_failed_samples_are_unavailable() {
        let mut data = PmLog::default();
        data.sensors[19] = Sensor {
            supported: 1,
            value: 65535,
        };
        data.sensors[32] = Sensor {
            supported: 1,
            value: 0,
        };
        data.sensors[27] = Sensor {
            supported: 1,
            value: 82,
        };
        assert!(data.gpu().utilization_percent.is_none());
        assert!(data.cpu_temperature().is_none());
        assert_eq!(
            data.gpu().temperature_source.as_deref(),
            Some("GPU hotspot")
        );
        assert!(PmLog::default().gpu().temperature_c.is_none());
        assert_eq!(std::mem::size_of::<PmLog>(), 2052);
    }
}
