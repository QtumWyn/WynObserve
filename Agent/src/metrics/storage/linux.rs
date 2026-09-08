use std::{fs, io, time::Instant};

use super::StorageSnapshot;

const SYS_BLOCK: &str = "/sys/block";
const SECTOR_BYTES: f64 = 512.0;
const MIB_BYTES: f64 = 1024.0 * 1024.0;

#[derive(Debug, Clone, Copy)]
struct DiskCounters {
    reads_completed: u64,
    sectors_read: u64,

    writes_completed: u64,
    sectors_written: u64,

    io_in_progress: u64,
    io_time_ms: u64,
    weighted_io_time_ms: u64,
}

#[derive(Debug)]
pub(super) struct PlatformStorageCollector {
    device: Option<String>,

    previous: Option<DiskCounters>,
    previous_sample_at: Option<Instant>,
}

impl PlatformStorageCollector {
    pub(super) fn new() -> Self {
        Self {
            device: discover_device(),
            previous: None,
            previous_sample_at: None,
        }
    }

    pub(super) fn sample(&mut self) -> StorageSnapshot {
        if self.device.is_none() {
            self.device = discover_device();
        }

        let Some(device) = self.device.clone() else {
            return unavailable_snapshot();
        };

        let counters = match read_disk_counters(&device) {
            Ok(counters) => counters,

            Err(_) => {
                self.device = None;

                return unavailable_snapshot();
            }
        };

        let model = read_model(&device).unwrap_or_else(|| device.clone());

        let now = Instant::now();

        let mut read_mib_s = 0.0;
        let mut write_mib_s = 0.0;

        let mut read_iops = 0.0;
        let mut write_iops = 0.0;

        let mut utilization_percent = 0.0;
        let mut average_queue_depth = 0.0;

        let mut rates_available = false;

        if let (Some(previous), Some(previous_at)) = (self.previous, self.previous_sample_at) {
            let elapsed = now.duration_since(previous_at);

            let elapsed_seconds = elapsed.as_secs_f64();

            let elapsed_ms = elapsed.as_secs_f64() * 1000.0;

            let deltas = (
                counters.sectors_read.checked_sub(previous.sectors_read),
                counters
                    .sectors_written
                    .checked_sub(previous.sectors_written),
                counters
                    .reads_completed
                    .checked_sub(previous.reads_completed),
                counters
                    .writes_completed
                    .checked_sub(previous.writes_completed),
                counters.io_time_ms.checked_sub(previous.io_time_ms),
                counters
                    .weighted_io_time_ms
                    .checked_sub(previous.weighted_io_time_ms),
            );

            if let (
                Some(read_sectors),
                Some(written_sectors),
                Some(reads),
                Some(writes),
                Some(io_ms),
                Some(weighted_io_ms),
            ) = deltas
            {
                if elapsed_seconds > 0.0 {
                    read_mib_s =
                        (read_sectors as f64 * SECTOR_BYTES / MIB_BYTES / elapsed_seconds) as f32;

                    write_mib_s = (written_sectors as f64 * SECTOR_BYTES
                        / MIB_BYTES
                        / elapsed_seconds) as f32;

                    read_iops = (reads as f64 / elapsed_seconds) as f32;

                    write_iops = (writes as f64 / elapsed_seconds) as f32;

                    utilization_percent = if elapsed_ms > 0.0 {
                        (io_ms as f64 / elapsed_ms * 100.0) as f32
                    } else {
                        0.0
                    }
                    .clamp(0.0, 100.0);

                    average_queue_depth = if elapsed_ms > 0.0 {
                        (weighted_io_ms as f64 / elapsed_ms) as f32
                    } else {
                        0.0
                    };

                    rates_available = true;
                }
            }
        }

        self.previous = Some(counters);
        self.previous_sample_at = Some(now);

        StorageSnapshot {
            available: true,

            device,
            model,

            read_mib_s,
            write_mib_s,

            read_iops,
            write_iops,

            utilization_percent,

            io_in_progress: counters.io_in_progress,

            average_queue_depth,

            rates_available,
        }
    }
}

fn read_disk_counters(device: &str) -> io::Result<DiskCounters> {
    let path = format!("{SYS_BLOCK}/{device}/stat");

    let contents = fs::read_to_string(path)?;

    let values = contents
        .split_whitespace()
        .map(|value| {
            value
                .parse::<u64>()
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
        })
        .collect::<Result<Vec<_>, _>>()?;

    if values.len() < 11 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "block stat contains fewer than 11 fields",
        ));
    }

    Ok(DiskCounters {
        reads_completed: values[0],
        sectors_read: values[2],

        writes_completed: values[4],
        sectors_written: values[6],

        io_in_progress: values[8],
        io_time_ms: values[9],
        weighted_io_time_ms: values[10],
    })
}

fn discover_device() -> Option<String> {
    let entries = fs::read_dir(SYS_BLOCK).ok()?;

    let mut devices = entries
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| {
            !name.starts_with("loop") && !name.starts_with("ram") && !name.starts_with("zram")
        })
        .collect::<Vec<_>>();

    devices.sort_by_key(|device| device_priority(device));

    devices.into_iter().next()
}

fn device_priority(device: &str) -> usize {
    if device.starts_with("nvme") {
        0
    } else if device.starts_with("sd") {
        1
    } else if device.starts_with("vd") {
        2
    } else if device.starts_with("xvd") {
        3
    } else if device.starts_with("mmcblk") {
        4
    } else if device.starts_with("md") {
        5
    } else if device.starts_with("dm-") {
        6
    } else {
        10
    }
}

fn read_model(device: &str) -> Option<String> {
    let path = format!("{SYS_BLOCK}/{device}/device/model");

    fs::read_to_string(path)
        .ok()
        .map(|model| model.trim().to_string())
        .filter(|model| !model.is_empty())
}

fn unavailable_snapshot() -> StorageSnapshot {
    StorageSnapshot {
        available: false,

        device: String::new(),
        model: "Storage unavailable".into(),

        read_mib_s: 0.0,
        write_mib_s: 0.0,

        read_iops: 0.0,
        write_iops: 0.0,

        utilization_percent: 0.0,

        io_in_progress: 0,
        average_queue_depth: 0.0,

        rates_available: false,
    }
}
