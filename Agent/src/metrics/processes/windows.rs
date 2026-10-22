use super::{ProcessCollectionSnapshot, ProcessSnapshot};
use crate::windows::{Handle, executable, text};
use std::{
    collections::HashMap,
    mem::{size_of, zeroed},
    time::Instant,
};
use windows_sys::Win32::{
    Foundation::FILETIME,
    System::{Diagnostics::ToolHelp::*, ProcessStatus::*, Threading::*},
};
#[derive(Debug, Clone)]
struct Previous {
    started: u64,
    cpu: u64,
    read: u64,
    write: u64,
    at: Instant,
    cpu_valid: bool,
    io_valid: bool,
}
#[derive(Debug, Default)]
pub(super) struct PlatformProcessCollector {
    previous: HashMap<u32, Previous>,
}
fn ticks(time: FILETIME) -> u64 {
    ((time.dwHighDateTime as u64) << 32) | time.dwLowDateTime as u64
}
impl PlatformProcessCollector {
    pub(super) fn new() -> Self {
        Self::default()
    }
    pub(super) fn sample(&mut self) -> ProcessCollectionSnapshot {
        let mut result = ProcessCollectionSnapshot {
            available: false,
            total_processes: 0,
            total_threads: 0,
            processes: vec![],
        };
        let Ok(snapshot) = Handle::new(unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) })
        else {
            self.previous.clear();
            return result;
        };
        let mut entry: PROCESSENTRY32W = unsafe { zeroed() };
        entry.dwSize = size_of::<PROCESSENTRY32W>() as u32;
        let mut ok = unsafe { Process32FirstW(snapshot.0, &mut entry) };
        let mut next = HashMap::new();
        let now = Instant::now();
        while ok != 0 {
            result.total_processes += 1;
            result.total_threads += entry.cntThreads as usize;
            let pid = entry.th32ProcessID;
            let mut p = ProcessSnapshot {
                pid,
                parent_pid: entry.th32ParentProcessID,
                name: text(&entry.szExeFile),
                executable: None,
                state: "unknown".into(),
                cpu_usage_percent: 0.0,
                cpu_rate_available: false,
                memory_bytes: 0,
                memory_available: false,
                // ToolHelp has no last-CPU or running/sleeping state; sentinel is never CPU 0.
                last_cpu: usize::MAX,
                threads: entry.cntThreads as usize,
                read_mib_s: 0.0,
                write_mib_s: 0.0,
                io_rates_available: false,
                started_at_unix_ms: None,
            };
            if let Ok(process) =
                Handle::new(unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) })
            {
                p.executable = executable(process.0);
                let (mut created, mut exited, mut kernel, mut user): (
                    FILETIME,
                    FILETIME,
                    FILETIME,
                    FILETIME,
                ) = unsafe { zeroed() };
                let cpu_valid = unsafe {
                    GetProcessTimes(process.0, &mut created, &mut exited, &mut kernel, &mut user)
                } != 0;
                let started = ticks(created);
                let cpu = ticks(kernel).saturating_add(ticks(user));
                if cpu_valid {
                    p.started_at_unix_ms = started
                        .checked_sub(116_444_736_000_000_000)
                        .map(|t| t / 10_000);
                }
                let mut io: IO_COUNTERS = unsafe { zeroed() };
                let io_valid = unsafe { GetProcessIoCounters(process.0, &mut io) } != 0;
                if let Some(old) = self
                    .previous
                    .get(&pid)
                    .filter(|old| cpu_valid && old.started == started)
                {
                    let elapsed = now.duration_since(old.at).as_secs_f64();
                    if elapsed > 0.0 {
                        if old.cpu_valid && cpu >= old.cpu {
                            p.cpu_usage_percent =
                                ((cpu - old.cpu) as f64 / 10_000_000.0 / elapsed * 100.0) as f32;
                            p.cpu_rate_available = true;
                        }
                        if io_valid
                            && old.io_valid
                            && io.ReadTransferCount >= old.read
                            && io.WriteTransferCount >= old.write
                        {
                            // Windows process I/O includes file/device/network transfers, not physical disk-only traffic.
                            p.read_mib_s = ((io.ReadTransferCount - old.read) as f64
                                / elapsed
                                / 1048576.0) as f32;
                            p.write_mib_s = ((io.WriteTransferCount - old.write) as f64
                                / elapsed
                                / 1048576.0) as f32;
                            p.io_rates_available = true;
                        }
                    }
                }
                next.insert(
                    pid,
                    Previous {
                        started,
                        cpu,
                        read: io.ReadTransferCount,
                        write: io.WriteTransferCount,
                        at: now,
                        cpu_valid,
                        io_valid,
                    },
                );
                let mut memory: PROCESS_MEMORY_COUNTERS = unsafe { zeroed() };
                memory.cb = size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
                if unsafe {
                    K32GetProcessMemoryInfo(
                        process.0,
                        &mut memory,
                        size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
                    )
                } != 0
                {
                    p.memory_bytes = memory.WorkingSetSize as u64;
                    p.memory_available = true;
                }
            }
            result.processes.push(p);
            entry.dwSize = size_of::<PROCESSENTRY32W>() as u32;
            ok = unsafe { Process32NextW(snapshot.0, &mut entry) };
        }
        result.available = std::io::Error::last_os_error().raw_os_error() == Some(18);
        result
            .processes
            .sort_by(|a, b| b.cpu_usage_percent.total_cmp(&a.cpu_usage_percent));
        result.processes.truncate(256);
        self.previous = next;
        result
    }
}
