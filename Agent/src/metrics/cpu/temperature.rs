use wyn_protocol::CpuTemperatureSnapshot;

pub fn sample() -> Option<CpuTemperatureSnapshot> {
    #[cfg(target_os = "linux")]
    return linux::sample_from(std::path::Path::new("/sys/class/hwmon"));

    // Windows' generic ACPI thermal zones do not identify a CPU package/die.
    // A CPU-specific sensor provider is required before reporting a measurement.
    #[cfg(not(target_os = "linux"))]
    None
}

#[cfg(target_os = "linux")]
mod linux {
    use super::CpuTemperatureSnapshot;
    use std::{fs, io::ErrorKind, path::Path};

    // Sources and units: docs.kernel.org/hwmon/{coretemp,k10temp,sysfs-interface}.html.
    // Only known CPU drivers and explicitly identified channels are accepted.
    // In particular, AMD Tctl is a control value, not necessarily physical °C.
    fn priority(driver: &str, label: &str) -> Option<u8> {
        match driver {
            "coretemp" if label.starts_with("Package id ") => Some(0),
            "coretemp" if label.starts_with("Core ") => Some(1),
            "k10temp" if label == "Tdie" => Some(0),
            "k10temp" if label.starts_with("Tccd") => Some(1),
            _ => None,
        }
    }

    fn optional_flag(path: &Path, expected: &str) -> bool {
        match fs::read_to_string(path) {
            Ok(value) => value.trim() == expected,
            Err(error) => error.kind() == ErrorKind::NotFound,
        }
    }

    pub(super) fn sample_from(root: &Path) -> Option<CpuTemperatureSnapshot> {
        let mut readings = Vec::new();
        for chip in fs::read_dir(root).ok()?.flatten() {
            let path = chip.path();
            let Ok(driver) = fs::read_to_string(path.join("name")) else {
                continue;
            };
            let driver = driver.trim();
            if !matches!(driver, "coretemp" | "k10temp") {
                continue;
            }
            let Ok(files) = fs::read_dir(&path) else {
                continue;
            };
            let mut candidates = Vec::new();
            for file in files.flatten() {
                let name = file.file_name();
                let Some(prefix) = name.to_str().and_then(|name| name.strip_suffix("_input"))
                else {
                    continue;
                };
                if prefix
                    .strip_prefix("temp")
                    .and_then(|id| id.parse::<u32>().ok())
                    .is_none_or(|id| id == 0)
                {
                    continue;
                }
                let Ok(label) = fs::read_to_string(path.join(format!("{prefix}_label"))) else {
                    continue;
                };
                let label = label.trim();
                let Some(priority) = priority(driver, label) else {
                    continue;
                };
                if !optional_flag(&path.join(format!("{prefix}_fault")), "0")
                    || !optional_flag(&path.join(format!("{prefix}_enable")), "1")
                {
                    continue;
                }
                let Some(millidegrees) = fs::read_to_string(file.path())
                    .ok()
                    .and_then(|value| value.trim().parse::<i32>().ok())
                    // Reject broken sensor values without clamping a real measurement.
                    .filter(|value| (-273_150..=200_000).contains(value))
                else {
                    continue;
                };
                candidates.push((
                    priority,
                    CpuTemperatureSnapshot {
                        celsius: millidegrees as f32 / 1000.0,
                        source: format!("{driver} · {label}"),
                    },
                ));
            }
            // Prefer package/Tdie per device; only use a core/CCD when unavailable.
            if let Some(best) = candidates.iter().map(|(priority, _)| *priority).min() {
                readings.extend(
                    candidates
                        .into_iter()
                        .filter(|(priority, _)| *priority == best)
                        .map(|(_, reading)| reading),
                );
            }
        }
        // Re-read/discover each time: errors and removed sensors must not retain a
        // previous value, and a driver loaded after Agent startup must be detected.
        readings.into_iter().max_by(|a, b| {
            a.celsius
                .total_cmp(&b.celsius)
                .then_with(|| a.source.cmp(&b.source))
        })
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::{
            path::PathBuf,
            sync::atomic::{AtomicU64, Ordering},
        };

        struct Fixture(PathBuf);
        impl Fixture {
            fn new() -> Self {
                static NEXT: AtomicU64 = AtomicU64::new(0);
                let path = std::env::temp_dir().join(format!(
                    "wyn-cpu-thermal-{}-{}",
                    std::process::id(),
                    NEXT.fetch_add(1, Ordering::Relaxed)
                ));
                fs::create_dir(&path).unwrap();
                Self(path)
            }
            fn sensor(&self, chip: &str, driver: &str, channel: u32, label: &str, value: &str) {
                let dir = self.0.join(chip);
                fs::create_dir_all(&dir).unwrap();
                fs::write(dir.join("name"), driver).unwrap();
                self.write(chip, &format!("temp{channel}_label"), label);
                self.write(chip, &format!("temp{channel}_input"), value);
            }
            fn write(&self, chip: &str, file: &str, value: &str) {
                fs::write(self.0.join(chip).join(file), value).unwrap();
            }
            fn read(&self) -> Option<CpuTemperatureSnapshot> {
                sample_from(&self.0)
            }
        }
        impl Drop for Fixture {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.0);
            }
        }

        #[test]
        fn intel_prefers_package_and_ignores_unrelated_sensors() {
            let f = Fixture::new();
            f.sensor("hwmon0", "nvme", 1, "Package id 0", "99000");
            f.sensor("hwmon1", "acpitz", 1, "CPU", "85000");
            f.sensor("hwmon7", "coretemp", 1, "Package id 0", "45125\n");
            f.sensor("hwmon7", "coretemp", 2, "Core 0", "60000");
            let reading = f.read().unwrap();
            assert_eq!(reading.celsius, 45.125);
            assert_eq!(reading.source, "coretemp · Package id 0");
        }

        #[test]
        fn hottest_package_is_selected_across_sockets() {
            let f = Fixture::new();
            f.sensor("hwmon0", "coretemp", 1, "Package id 0", "40000");
            f.sensor("hwmon1", "coretemp", 1, "Package id 1", "52000");
            assert_eq!(f.read().unwrap().source, "coretemp · Package id 1");
        }

        #[test]
        fn older_intel_can_use_the_hottest_identified_core() {
            let f = Fixture::new();
            f.sensor("hwmon0", "coretemp", 2, "Core 0", "40000");
            f.sensor("hwmon0", "coretemp", 3, "Core 1", "52000");
            assert_eq!(f.read().unwrap().source, "coretemp · Core 1");
        }

        #[test]
        fn amd_prefers_tdie_and_never_reports_control_values_as_physical_temperature() {
            let f = Fixture::new();
            f.sensor("hwmon0", "k10temp", 1, "Tctl", "85000");
            assert!(f.read().is_none());
            f.sensor("hwmon0", "k10temp", 3, "Tccd1", "65000");
            assert_eq!(f.read().unwrap().source, "k10temp · Tccd1");
            f.sensor("hwmon0", "k10temp", 2, "Tdie", "60000");
            assert_eq!(f.read().unwrap().celsius, 60.0);
        }

        #[test]
        fn failures_clear_the_reading_and_late_sensors_recover() {
            let f = Fixture::new();
            assert!(f.read().is_none());
            f.sensor("hwmon7", "coretemp", 1, "Package id 0", "42000");
            assert_eq!(f.read().unwrap().celsius, 42.0);
            for invalid in ["broken", "NaN", "300000", "-300000", ""] {
                f.write("hwmon7", "temp1_input", invalid);
                assert!(f.read().is_none());
            }
            f.write("hwmon7", "temp1_input", "0");
            assert_eq!(f.read().unwrap().celsius, 0.0);
            f.write("hwmon7", "temp1_fault", "1");
            assert!(f.read().is_none());
            f.write("hwmon7", "temp1_fault", "0");
            f.write("hwmon7", "temp1_enable", "0");
            assert!(f.read().is_none());
            f.write("hwmon7", "temp1_enable", "1");
            f.write("hwmon7", "temp1_input", "43000");
            assert_eq!(f.read().unwrap().celsius, 43.0);
            fs::remove_file(f.0.join("hwmon7/temp1_input")).unwrap();
            assert!(f.read().is_none());
            assert!(sample_from(&f.0.join("absent")).is_none());
        }
    }
}
