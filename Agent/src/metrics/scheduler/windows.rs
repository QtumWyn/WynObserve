use super::SchedulerSnapshot;
use crate::windows::Counters;
#[derive(Debug)]
pub(super) struct PlatformSchedulerCollector {
    counters: Option<Counters>,
}
impl PlatformSchedulerCollector {
    pub(super) fn new() -> Self {
        Self {
            counters: Counters::new(&[
                r"\System\Context Switches/sec",
                r"\System\Processor Queue Length",
            ]),
        }
    }
    pub(super) fn sample(&mut self) -> SchedulerSnapshot {
        let v = self
            .counters
            .as_mut()
            .map(|q| q.sample())
            .unwrap_or_else(|| vec![None; 2]);
        SchedulerSnapshot {
            context_switches_per_second: v[0].unwrap_or(0.0).round() as u64,
            runnable_tasks: v[1].unwrap_or(0.0).round() as u64,
            // Windows ready queue excludes executing threads; Linux blocked-task count has no PDH equivalent.
            blocked_tasks: 0,
            runnable_tasks_available: v[1].is_some(),
            blocked_tasks_available: false,
            context_switch_rate_available: v[0].is_some(),
        }
    }
}
