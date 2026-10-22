#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "linux")]
pub(crate) use linux::{PerfBuffer, PerfSample, map_probe_for_tid};

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub(crate) use windows::{PerfBuffer, PerfSample, map_probe_for_tid};
