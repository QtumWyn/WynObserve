use std::io;

use wyn_protocol::process_memory::ProcessMemoryMap;

pub(super) fn read_process_memory_map(_pid: u32) -> io::Result<ProcessMemoryMap> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "process memory maps are not yet supported on Windows",
    ))
}
