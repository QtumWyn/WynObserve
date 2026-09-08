use super::StorageSnapshot;

#[derive(Debug, Default)]
pub(super) struct PlatformStorageCollector;

impl PlatformStorageCollector {
    pub(super) fn new() -> Self {
        Self
    }

    pub(super) fn sample(&mut self) -> StorageSnapshot {
        StorageSnapshot {
            available: false,

            device: String::new(),
            model: "Windows storage telemetry pending".into(),

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
}
