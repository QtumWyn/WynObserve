#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "linux")]
pub(crate) use linux::{PerfBuffer, PerfSample, map_probe_for_tid};
