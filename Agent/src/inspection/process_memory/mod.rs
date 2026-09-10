use std::io;

use wyn_protocol::process_memory::ProcessMemoryMap;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
use linux::read_process_memory_map;

#[cfg(target_os = "windows")]
use windows::read_process_memory_map;

pub fn read(pid: u32) -> io::Result<ProcessMemoryMap> {
    read_process_memory_map(pid)
}
