use super::SchedulerSnapshot;

#[derive(Debug, Default)]
pub(super) struct PlatformSchedulerCollector;

impl PlatformSchedulerCollector {
    pub(super) fn new() -> Self {
        Self
    }

    pub(super) fn sample(&mut self) -> SchedulerSnapshot {
        SchedulerSnapshot {
            context_switches_per_second: 0,
            runnable_tasks: 0,
            blocked_tasks: 0,
            context_switch_rate_available: false,
        }
    }
}
