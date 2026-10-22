use super::StorageSnapshot;
use crate::windows::{Counters, wide};
use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
#[derive(Debug)]
pub(super) struct PlatformStorageCollector {
    drive: String,
    counters: Option<Counters>,
}
impl PlatformStorageCollector {
    pub(super) fn new() -> Self {
        let drive = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into());
        let paths = [
            "Disk Read Bytes/sec",
            "Disk Write Bytes/sec",
            "Disk Reads/sec",
            "Disk Writes/sec",
            "% Idle Time",
            "Current Disk Queue Length",
            "Avg. Disk Queue Length",
        ]
        .map(|name| format!(r"\LogicalDisk({drive})\{name}"));
        Self {
            drive,
            counters: Counters::new(&paths.iter().map(String::as_str).collect::<Vec<_>>()),
        }
    }
    pub(super) fn sample(&mut self) -> StorageSnapshot {
        let v = self
            .counters
            .as_mut()
            .map(|q| q.sample())
            .unwrap_or_else(|| vec![None; 7]);
        let (mut available, mut total, mut free) = (0, 0, 0);
        let mount = format!("{}\\", self.drive);
        let space = unsafe {
            GetDiskFreeSpaceExW(wide(&mount).as_ptr(), &mut available, &mut total, &mut free)
        } != 0;
        StorageSnapshot {
            available: space || v.iter().any(Option::is_some),
            device: self.drive.clone(),
            model: "Windows system volume".into(),
            space_available: space,
            mount_point: mount,
            capacity_bytes: total,
            used_bytes: total.saturating_sub(free),
            available_bytes: available,
            read_mib_s: (v[0].unwrap_or(0.0) / 1048576.0) as f32,
            write_mib_s: (v[1].unwrap_or(0.0) / 1048576.0) as f32,
            read_iops: v[2].unwrap_or(0.0) as f32,
            write_iops: v[3].unwrap_or(0.0) as f32,
            utilization_percent: v[4]
                .map(|idle| (100.0 - idle).clamp(0.0, 100.0))
                .unwrap_or(0.0) as f32,
            io_in_progress: v[5].unwrap_or(0.0).round() as u64,
            average_queue_depth: v[6].unwrap_or(0.0) as f32,
            rates_available: v.iter().all(Option::is_some),
        }
    }
}
