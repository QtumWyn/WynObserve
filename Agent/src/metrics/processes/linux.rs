use std::{collections::HashMap, fs, io, path::PathBuf, time::Instant};

use super::{ProcessCollectionSnapshot, ProcessSnapshot};

const PROC_ROOT: &str = "/proc";

const MIB_BYTES: f64 = 1024.0 * 1024.0;

// Do not stuff every process on the machine into
// every 500 ms NDJSON packet.
//
// We still SAMPLE every visible process so rates
// remain correct, but export only the most
// interesting ones.
const MAX_EXPORTED_PROCESSES: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ProcessIdentity {
    pid: u32,
    start_time_ticks: u64,
}

#[derive(Debug, Clone, Copy)]
struct PreviousProcessCounters {
    cpu_ticks: u64,

    read_bytes: Option<u64>,
    write_bytes: Option<u64>,
}

#[derive(Debug)]
struct RawProcessStat {
    pid: u32,
    parent_pid: u32,

    comm: String,
    state: char,

    user_ticks: u64,
    system_ticks: u64,

    threads: usize,

    start_time_ticks: u64,

    last_cpu: usize,
}

#[derive(Debug)]
pub(super) struct PlatformProcessCollector {
    previous: HashMap<ProcessIdentity, PreviousProcessCounters>,

    previous_sample_at: Option<Instant>,

    clock_ticks_per_second: Option<f64>,

    boot_time_unix_seconds: Option<u64>,
}

impl PlatformProcessCollector {
    pub(super) fn new() -> Self {
        Self {
            previous: HashMap::new(),

            previous_sample_at: None,

            clock_ticks_per_second: Self::clock_ticks_per_second(),

            boot_time_unix_seconds: read_boot_time_unix_seconds().ok(),
        }
    }

    fn clock_ticks_per_second() -> Option<f64> {
        let value = unsafe { libc::sysconf(libc::_SC_CLK_TCK) };

        if value > 0 { Some(value as f64) } else { None }
    }

    fn discover_pids() -> io::Result<Vec<u32>> {
        let mut pids = Vec::new();

        for entry in fs::read_dir(PROC_ROOT)? {
            let Ok(entry) = entry else {
                continue;
            };

            let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };

            let Ok(pid) = name.parse::<u32>() else {
                continue;
            };

            pids.push(pid);
        }

        Ok(pids)
    }

    fn read_process_stat(pid: u32) -> io::Result<RawProcessStat> {
        let path = format!("{PROC_ROOT}/{pid}/stat");

        let contents = fs::read_to_string(path)?;

        Self::parse_process_stat(pid, &contents)
    }

    fn parse_process_stat(pid: u32, contents: &str) -> io::Result<RawProcessStat> {
        let open = contents
            .find('(')
            .ok_or_else(|| invalid_data("process stat missing '('"))?;

        let close = contents
            .rfind(')')
            .ok_or_else(|| invalid_data("process stat missing ')'"))?;

        if close <= open {
            return Err(invalid_data("invalid process command field"));
        }

        let comm = contents[open + 1..close].to_string();

        let fields = contents[close + 1..].split_whitespace().collect::<Vec<_>>();

        // fields[0] corresponds to Linux stat
        // field 3 because pid and comm were removed.
        if fields.len() < 37 {
            return Err(invalid_data("process stat contains too few fields"));
        }

        let state = fields[0]
            .chars()
            .next()
            .ok_or_else(|| invalid_data("missing process state"))?;

        let parent_pid = parse_field::<u32>(fields[1], "ppid")?;

        let user_ticks = parse_field::<u64>(fields[11], "utime")?;

        let system_ticks = parse_field::<u64>(fields[12], "stime")?;

        let threads = parse_field::<usize>(fields[17], "num_threads")?;

        let start_time_ticks = parse_field::<u64>(fields[19], "starttime")?;

        let last_cpu_raw = parse_field::<i64>(fields[36], "processor")?;

        let last_cpu = usize::try_from(last_cpu_raw.max(0)).unwrap_or(0);

        Ok(RawProcessStat {
            pid,
            parent_pid,

            comm,
            state,

            user_ticks,
            system_ticks,

            threads,

            start_time_ticks,

            last_cpu,
        })
    }

    pub(super) fn sample(&mut self) -> ProcessCollectionSnapshot {
        let now = Instant::now();

        let elapsed_seconds = self
            .previous_sample_at
            .map(|previous| now.duration_since(previous).as_secs_f64());

        let pids = match Self::discover_pids() {
            Ok(pids) => pids,

            Err(_) => {
                return ProcessCollectionSnapshot {
                    available: false,

                    total_processes: 0,
                    total_threads: 0,

                    processes: Vec::new(),
                };
            }
        };

        let mut next_previous = HashMap::new();

        let mut processes = Vec::new();

        let mut total_processes: usize = 0;
        let mut total_threads: usize = 0;

        for pid in pids {
            let Ok(stat) = Self::read_process_stat(pid) else {
                // Process exited, permission changed,
                // or proc raced us.
                continue;
            };

            total_processes += 1;

            total_threads = total_threads.saturating_add(stat.threads);

            let identity = ProcessIdentity {
                pid: stat.pid,

                start_time_ticks: stat.start_time_ticks,
            };

            let cpu_ticks = stat.user_ticks.saturating_add(stat.system_ticks);

            let process_io = read_process_io(pid).ok();

            let (read_bytes, write_bytes) = process_io
                .map(|(read, write)| (Some(read), Some(write)))
                .unwrap_or((None, None));

            let previous = self.previous.get(&identity);

            let mut cpu_usage_percent = 0.0;

            let mut cpu_rate_available = false;

            if let (Some(previous), Some(elapsed), Some(ticks_per_second)) =
                (previous, elapsed_seconds, self.clock_ticks_per_second)
            {
                if elapsed > 0.0 && ticks_per_second > 0.0 {
                    if let Some(delta_ticks) = cpu_ticks.checked_sub(previous.cpu_ticks) {
                        cpu_usage_percent =
                            (delta_ticks as f64 / ticks_per_second / elapsed * 100.0) as f32;

                        cpu_rate_available = true;
                    }
                }
            }

            let mut read_mib_s = 0.0;
            let mut write_mib_s = 0.0;

            let mut io_rates_available = false;

            if let (
                Some(previous),
                Some(elapsed),
                Some(current_read),
                Some(current_write),
                Some(previous_read),
                Some(previous_write),
            ) = (
                previous,
                elapsed_seconds,
                read_bytes,
                write_bytes,
                previous.and_then(|value| value.read_bytes),
                previous.and_then(|value| value.write_bytes),
            ) {
                if elapsed > 0.0 {
                    if let (Some(read_delta), Some(write_delta)) = (
                        current_read.checked_sub(previous_read),
                        current_write.checked_sub(previous_write),
                    ) {
                        read_mib_s = (read_delta as f64 / MIB_BYTES / elapsed) as f32;

                        write_mib_s = (write_delta as f64 / MIB_BYTES / elapsed) as f32;

                        io_rates_available = true;
                    }
                }
            }

            let memory = read_memory_bytes(pid).ok();

            let executable = read_executable(pid);

            let display_name = executable
                .as_deref()
                .and_then(|path| std::path::Path::new(path).file_name())
                .and_then(|name| name.to_str())
                .filter(|name| !name.is_empty())
                .map(str::to_owned)
                .unwrap_or_else(|| stat.comm.clone());

            processes.push(ProcessSnapshot {
                pid: stat.pid,

                parent_pid: stat.parent_pid,

                name: display_name,

                executable,

                state: process_state(stat.state),

                cpu_usage_percent,
                cpu_rate_available,

                memory_bytes: memory.unwrap_or(0),

                memory_available: memory.is_some(),

                last_cpu: stat.last_cpu,

                threads: stat.threads,

                read_mib_s,
                write_mib_s,

                io_rates_available,

                started_at_unix_ms: process_started_at(
                    self.boot_time_unix_seconds,
                    stat.start_time_ticks,
                    self.clock_ticks_per_second,
                ),
            });

            next_previous.insert(
                identity,
                PreviousProcessCounters {
                    cpu_ticks,

                    read_bytes,
                    write_bytes,
                },
            );
        }

        self.previous = next_previous;

        self.previous_sample_at = Some(now);

        // Most CPU-hungry first.
        //
        // On the initial baseline, all CPU
        // rates are zero, so memory becomes
        // the useful tie breaker.
        processes.sort_by(|left, right| {
            right
                .cpu_usage_percent
                .total_cmp(&left.cpu_usage_percent)
                .then_with(|| right.memory_bytes.cmp(&left.memory_bytes))
        });

        processes.truncate(MAX_EXPORTED_PROCESSES);

        ProcessCollectionSnapshot {
            available: true,

            total_processes,
            total_threads,

            processes,
        }
    }
}

fn parse_field<T>(value: &str, name: &str) -> io::Result<T>
where
    T: std::str::FromStr,
    T::Err: std::error::Error + Send + Sync + 'static,
{
    value.parse::<T>().map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid {name}: {error}"),
        )
    })
}

fn invalid_data(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn read_memory_bytes(pid: u32) -> io::Result<u64> {
    let path = format!("{PROC_ROOT}/{pid}/status");

    let contents = fs::read_to_string(path)?;

    for line in contents.lines() {
        let Some(value) = line.strip_prefix("VmRSS:") else {
            continue;
        };

        let kib = value
            .split_whitespace()
            .next()
            .ok_or_else(|| invalid_data("VmRSS missing value"))?
            .parse::<u64>()
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;

        return Ok(kib.saturating_mul(1024));
    }

    Err(io::Error::new(io::ErrorKind::NotFound, "VmRSS not found"))
}

fn read_executable(pid: u32) -> Option<String> {
    let path = PathBuf::from(format!("{PROC_ROOT}/{pid}/exe"));

    fs::read_link(path)
        .ok()
        .map(|path| path.to_string_lossy().into_owned())
}

fn read_process_io(pid: u32) -> io::Result<(u64, u64)> {
    let path = format!("{PROC_ROOT}/{pid}/io");

    let contents = fs::read_to_string(path)?;

    let mut read_bytes = None;
    let mut write_bytes = None;

    for line in contents.lines() {
        if let Some(value) = line.strip_prefix("read_bytes:") {
            read_bytes = value.trim().parse::<u64>().ok();
        }

        if let Some(value) = line.strip_prefix("write_bytes:") {
            write_bytes = value.trim().parse::<u64>().ok();
        }
    }

    match (read_bytes, write_bytes) {
        (Some(read_bytes), Some(write_bytes)) => Ok((read_bytes, write_bytes)),

        _ => Err(invalid_data("process I/O counters missing")),
    }
}

fn process_state(state: char) -> String {
    match state {
        'R' => "running",
        'S' => "sleeping",
        'D' => "disk-sleep",
        'Z' => "zombie",
        'T' => "stopped",
        't' => "tracing-stop",
        'I' => "idle",
        'X' | 'x' => "dead",

        _ => "unknown",
    }
    .to_string()
}

fn read_boot_time_unix_seconds() -> io::Result<u64> {
    let contents = fs::read_to_string("/proc/stat")?;

    for line in contents.lines() {
        let Some(value) = line.strip_prefix("btime ") else {
            continue;
        };

        return value
            .trim()
            .parse::<u64>()
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error));
    }

    Err(io::Error::new(io::ErrorKind::NotFound, "btime not found"))
}

fn process_started_at(
    boot_time_seconds: Option<u64>,
    start_ticks: u64,
    ticks_per_second: Option<f64>,
) -> Option<u64> {
    let boot = boot_time_seconds?;

    let ticks = ticks_per_second?;

    if ticks <= 0.0 {
        return None;
    }

    let after_boot_ms = (start_ticks as f64 / ticks * 1000.0) as u64;

    Some(boot.saturating_mul(1000).saturating_add(after_boot_ms))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_process_stat_with_spaces_and_parentheses_in_name() {
        let mut fields = vec!["0"; 37];

        // field 3
        fields[0] = "R";

        // field 4
        fields[1] = "123";

        // field 14
        fields[11] = "140";

        // field 15
        fields[12] = "15";

        // field 20
        fields[17] = "8";

        // field 22
        fields[19] = "2200";

        // field 39
        fields[36] = "7";

        let contents = format!("4242 (worker pool (A)) {}", fields.join(" "));

        let stat = PlatformProcessCollector::parse_process_stat(4242, &contents).unwrap();

        assert_eq!(stat.pid, 4242);

        assert_eq!(stat.comm, "worker pool (A)");

        assert_eq!(stat.parent_pid, 123);

        assert_eq!(stat.user_ticks, 140);

        assert_eq!(stat.system_ticks, 15);

        assert_eq!(stat.threads, 8);

        assert_eq!(stat.start_time_ticks, 2200);

        assert_eq!(stat.last_cpu, 7);
    }
}
