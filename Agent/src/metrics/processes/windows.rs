use super::ProcessCollectionSnapshot;

#[derive(Debug, Default)]
pub(super) struct PlatformProcessCollector;

impl PlatformProcessCollector {
    pub(super) fn new() -> Self {
        Self
    }

    pub(super) fn sample(&mut self) -> ProcessCollectionSnapshot {
        ProcessCollectionSnapshot {
            available: false,

            total_processes: 0,
            total_threads: 0,

            processes: Vec::new(),
        }
    }
}
