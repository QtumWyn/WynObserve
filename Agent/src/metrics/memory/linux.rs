use std::{fs, io, time::Instant};

use sysinfo::System;

use super::MemorySnapshot;

#[derive(Debug, Default)]
struct ProcMemInfo {
    cached_bytes: u64,
    active_bytes: u64,
    dirty_bytes: u64,
}

#[derive(Debug, Default)]
struct ProcVmStat {
    page_faults: u64,
    major_page_faults: u64,
}

#[derive(Debug)]
pub(super) struct PlatformMemoryCollector {
    previous_page_faults: Option<u64>,
    previous_major_page_faults: Option<u64>,
    previous_sample_at: Option<Instant>,
}

impl PlatformMemoryCollector {
    pub(super) fn new() -> Self {
        Self {
            previous_page_faults: None,
            previous_major_page_faults: None,
            previous_sample_at: None,
        }
    }

    pub(super) fn sample(&mut self, system: &System) -> MemorySnapshot {
        let now = Instant::now();

        let proc_mem = read_proc_meminfo().ok();
        let vmstat = read_proc_vmstat().ok();

        let memory_detail_available = proc_mem.is_some();

        let cached_bytes = proc_mem.as_ref().map(|info| info.cached_bytes).unwrap_or(0);

        let active_bytes = proc_mem.as_ref().map(|info| info.active_bytes).unwrap_or(0);

        let dirty_bytes = proc_mem.as_ref().map(|info| info.dirty_bytes).unwrap_or(0);

        let mut fault_rates_available = false;

        let (page_faults_per_second, major_page_faults_per_second) = if let Some(vmstat) = vmstat {
            let rates = match (
                self.previous_page_faults,
                self.previous_major_page_faults,
                self.previous_sample_at,
            ) {
                (Some(previous_faults), Some(previous_major), Some(previous_time)) => {
                    let elapsed = now.duration_since(previous_time).as_secs_f64();

                    if elapsed <= 0.0 {
                        (0, 0)
                    } else {
                        fault_rates_available = true;

                        let fault_delta = vmstat.page_faults.saturating_sub(previous_faults);

                        let major_delta = vmstat.major_page_faults.saturating_sub(previous_major);

                        (
                            (fault_delta as f64 / elapsed).round() as u64,
                            (major_delta as f64 / elapsed).round() as u64,
                        )
                    }
                }
                _ => (0, 0),
            };

            self.previous_page_faults = Some(vmstat.page_faults);
            self.previous_major_page_faults = Some(vmstat.major_page_faults);
            self.previous_sample_at = Some(now);

            rates
        } else {
            (0, 0)
        };

        MemorySnapshot {
            total_bytes: system.total_memory(),
            used_bytes: system.used_memory(),
            available_bytes: system.available_memory(),
            cached_bytes,
            active_bytes,
            dirty_bytes,
            page_faults_per_second,
            major_page_faults_per_second,
            total_swap_bytes: system.total_swap(),
            used_swap_bytes: system.used_swap(),
            memory_detail_available,
            fault_rates_available,
        }
    }
}

fn kib_to_bytes(value: u64) -> u64 {
    value.saturating_mul(1024)
}

fn parse_proc_meminfo(contents: &str) -> ProcMemInfo {
    let mut info = ProcMemInfo::default();

    for line in contents.lines() {
        let Some((key, rest)) = line.split_once(':') else {
            continue;
        };

        let Some(value_text) = rest.split_whitespace().next() else {
            continue;
        };

        let Ok(value_kib) = value_text.parse::<u64>() else {
            continue;
        };

        let value_bytes = kib_to_bytes(value_kib);

        match key {
            "Cached" => info.cached_bytes = value_bytes,
            "Active" => info.active_bytes = value_bytes,
            "Dirty" => info.dirty_bytes = value_bytes,
            _ => {}
        }
    }

    info
}

fn read_proc_meminfo() -> io::Result<ProcMemInfo> {
    let contents = fs::read_to_string("/proc/meminfo")?;
    Ok(parse_proc_meminfo(&contents))
}

fn parse_proc_vmstat(contents: &str) -> ProcVmStat {
    let mut stat = ProcVmStat::default();

    for line in contents.lines() {
        let mut parts = line.split_whitespace();

        let Some(key) = parts.next() else {
            continue;
        };

        let Some(value_text) = parts.next() else {
            continue;
        };

        let Ok(value) = value_text.parse::<u64>() else {
            continue;
        };

        match key {
            "pgfault" => stat.page_faults = value,
            "pgmajfault" => stat.major_page_faults = value,
            _ => {}
        }
    }

    stat
}

fn read_proc_vmstat() -> io::Result<ProcVmStat> {
    let contents = fs::read_to_string("/proc/vmstat")?;
    Ok(parse_proc_vmstat(&contents))
}

#[cfg(test)]
mod tests {
    use super::{parse_proc_meminfo, parse_proc_vmstat};

    #[test]
    fn parses_proc_meminfo_fields() {
        let input = "Cached: 4000 kB\nActive: 6000 kB\nDirty: 100 kB\n";

        let info = parse_proc_meminfo(input);

        assert_eq!(info.cached_bytes, 4000 * 1024);
        assert_eq!(info.active_bytes, 6000 * 1024);
        assert_eq!(info.dirty_bytes, 100 * 1024);
    }

    #[test]
    fn parses_proc_vmstat_fields() {
        let input = "pgfault 123456\npgmajfault 789\npgfree 999999\n";

        let stat = parse_proc_vmstat(input);

        assert_eq!(stat.page_faults, 123456);
        assert_eq!(stat.major_page_faults, 789);
    }
}
