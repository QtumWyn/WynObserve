use std::{fs, time::Instant};

use super::SchedulerSnapshot;

#[derive(Debug, Default)]
struct ProcStat {
    context_switches: u64,
    runnable_tasks: u64,
    blocked_tasks: u64,
}

fn parse_proc_stat(contents: &str) -> ProcStat {
    let mut stat = ProcStat::default();

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
            "ctxt" => {
                stat.context_switches = value;
            }

            "procs_running" => {
                stat.runnable_tasks = value;
            }

            "procs_blocked" => {
                stat.blocked_tasks = value;
            }

            _ => {}
        }
    }

    stat
}

fn read_proc_stat() -> std::io::Result<ProcStat> {
    let contents = fs::read_to_string("/proc/stat")?;

    Ok(parse_proc_stat(&contents))
}

#[derive(Debug)]
pub(super) struct PlatformSchedulerCollector {
    previous_context_switches: Option<u64>,

    previous_sample_at: Option<Instant>,
}

impl PlatformSchedulerCollector {
    pub(super) fn new() -> Self {
        Self {
            previous_context_switches: None,

            previous_sample_at: None,
        }
    }

    pub(super) fn sample(&mut self) -> SchedulerSnapshot {
        let now = Instant::now();

        let proc_stat = read_proc_stat().ok();
        let context_switch_rate_available = proc_stat.is_some()
            && self.previous_context_switches.is_some()
            && self.previous_sample_at.is_some();
        let context_switches_per_second = if let Some(ref stat) = proc_stat {
            match (self.previous_context_switches, self.previous_sample_at) {
                (Some(previous_context_switches), Some(previous_time)) => {
                    let elapsed = now.duration_since(previous_time).as_secs_f64();

                    if elapsed <= 0.0 {
                        0
                    } else {
                        let delta = stat
                            .context_switches
                            .saturating_sub(previous_context_switches);

                        (delta as f64 / elapsed).round() as u64
                    }
                }

                _ => 0,
            }
        } else {
            0
        };
        let runnable_tasks = proc_stat
            .as_ref()
            .map(|stat| stat.runnable_tasks)
            .unwrap_or(0);

        let blocked_tasks = proc_stat
            .as_ref()
            .map(|stat| stat.blocked_tasks)
            .unwrap_or(0);

        if let Some(stat) = proc_stat {
            self.previous_context_switches = Some(stat.context_switches);

            self.previous_sample_at = Some(now);
        }

        SchedulerSnapshot {
            context_switches_per_second,
            runnable_tasks,
            blocked_tasks,
            context_switch_rate_available,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parse_proc_stat;

    #[test]
    fn parses_scheduler_fields() {
        let input = "\
cpu  100 0 200 300
intr 123456
ctxt 2114644996
btime 1234567890
processes 900000
procs_running 2
procs_blocked 0
";

        let stat = parse_proc_stat(input);

        assert_eq!(stat.context_switches, 2_114_644_996);

        assert_eq!(stat.runnable_tasks, 2);

        assert_eq!(stat.blocked_tasks, 0);
    }
}
