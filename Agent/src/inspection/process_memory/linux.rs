use std::{
    fs, io,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use wyn_protocol::process_memory::{
    ProcessMemoryMap, ProcessMemoryOverview, ProcessMemoryPermissions, ProcessMemoryRegion,
    ProcessMemoryRegionKind,
};

const PROC_ROOT: &str = "/proc";

pub(super) fn read_process_memory_map(pid: u32) -> io::Result<ProcessMemoryMap> {
    let maps_path = format!("{PROC_ROOT}/{pid}/maps");

    let contents = fs::read_to_string(maps_path)?;

    let executable = read_executable(pid);

    let process_name = read_process_name(pid).unwrap_or_else(|| format!("PID {pid}"));

    let mut regions = Vec::new();

    for line in contents.lines() {
        if line.trim().is_empty() {
            continue;
        }

        regions.push(parse_maps_line(line, executable.as_deref())?);
    }

    let overview = summarize(&regions);

    let total_virtual_bytes = regions.iter().map(|region| region.size_bytes).sum();

    let writable_executable_regions = regions
        .iter()
        .filter(|region| region.permissions.writable && region.permissions.executable)
        .count();

    Ok(ProcessMemoryMap {
        pid,

        process_name,

        executable,

        captured_at_unix_ms: unix_timestamp_ms(),

        total_virtual_bytes,

        region_count: regions.len(),

        writable_executable_regions,

        overview,

        regions,
    })
}

fn parse_maps_line(line: &str, executable: Option<&str>) -> io::Result<ProcessMemoryRegion> {
    let mut fields = line.split_whitespace();

    let address_range = required(fields.next(), "address range")?;

    let permissions_text = required(fields.next(), "permissions")?;

    let offset_text = required(fields.next(), "offset")?;

    let device_text = required(fields.next(), "device")?;

    let inode_text = required(fields.next(), "inode")?;

    let pathname = {
        let remaining = fields.collect::<Vec<_>>();

        if remaining.is_empty() {
            None
        } else {
            Some(remaining.join(" "))
        }
    };

    let (start_address, end_address) = parse_address_range(address_range)?;

    let permissions = parse_permissions(permissions_text)?;

    let offset = u64::from_str_radix(offset_text, 16)
        .map_err(|error| invalid_data(format!("invalid mapping offset: {error}")))?;

    let (device_major, device_minor) = parse_device(device_text)?;

    let inode = inode_text
        .parse::<u64>()
        .map_err(|error| invalid_data(format!("invalid inode: {error}")))?;

    let size_bytes = end_address.saturating_sub(start_address);

    let kind = classify_region(pathname.as_deref(), executable);

    Ok(ProcessMemoryRegion {
        start_address,
        end_address,
        size_bytes,

        permissions,

        offset,

        device_major,
        device_minor,

        inode,

        pathname,

        kind,
    })
}

fn parse_address_range(value: &str) -> io::Result<(u64, u64)> {
    let (start, end) = value
        .split_once('-')
        .ok_or_else(|| invalid_data("mapping address range missing '-'"))?;

    let start = u64::from_str_radix(start, 16)
        .map_err(|error| invalid_data(format!("invalid start address: {error}")))?;

    let end = u64::from_str_radix(end, 16)
        .map_err(|error| invalid_data(format!("invalid end address: {error}")))?;

    if end < start {
        return Err(invalid_data("mapping ends before it starts"));
    }

    Ok((start, end))
}

fn parse_permissions(value: &str) -> io::Result<ProcessMemoryPermissions> {
    let chars = value.chars().collect::<Vec<_>>();

    if chars.len() != 4 {
        return Err(invalid_data(
            "mapping permissions must contain four characters",
        ));
    }

    Ok(ProcessMemoryPermissions {
        readable: chars[0] == 'r',

        writable: chars[1] == 'w',

        executable: chars[2] == 'x',

        private: chars[3] == 'p',

        shared: chars[3] == 's',
    })
}

fn parse_device(value: &str) -> io::Result<(u32, u32)> {
    let (major, minor) = value
        .split_once(':')
        .ok_or_else(|| invalid_data("mapping device missing ':'"))?;

    let major = u32::from_str_radix(major, 16)
        .map_err(|error| invalid_data(format!("invalid device major: {error}")))?;

    let minor = u32::from_str_radix(minor, 16)
        .map_err(|error| invalid_data(format!("invalid device minor: {error}")))?;

    Ok((major, minor))
}

fn classify_region(pathname: Option<&str>, executable: Option<&str>) -> ProcessMemoryRegionKind {
    let Some(pathname) = pathname else {
        return ProcessMemoryRegionKind::Anonymous;
    };

    if pathname == "[heap]" {
        return ProcessMemoryRegionKind::Heap;
    }

    if pathname.starts_with("[stack") {
        return ProcessMemoryRegionKind::Stack;
    }

    if matches!(pathname, "[vdso]" | "[vvar]" | "[vsyscall]") {
        return ProcessMemoryRegionKind::Special;
    }

    if executable == Some(pathname) {
        return ProcessMemoryRegionKind::ExecutableImage;
    }

    if is_shared_library(pathname) {
        return ProcessMemoryRegionKind::SharedLibrary;
    }

    if pathname.starts_with('/') {
        return ProcessMemoryRegionKind::MappedFile;
    }

    if pathname.starts_with('[') {
        return ProcessMemoryRegionKind::Special;
    }

    ProcessMemoryRegionKind::Other
}

fn is_shared_library(pathname: &str) -> bool {
    let clean = pathname.strip_suffix(" (deleted)").unwrap_or(pathname);

    let Some(name) = Path::new(clean).file_name().and_then(|name| name.to_str()) else {
        return false;
    };

    name.contains(".so")
}

fn summarize(regions: &[ProcessMemoryRegion]) -> ProcessMemoryOverview {
    let mut overview = ProcessMemoryOverview::default();

    for region in regions {
        match region.kind {
            ProcessMemoryRegionKind::Heap => {
                overview.heap_bytes = overview.heap_bytes.saturating_add(region.size_bytes);
            }

            ProcessMemoryRegionKind::Anonymous => {
                overview.anonymous_bytes =
                    overview.anonymous_bytes.saturating_add(region.size_bytes);
            }

            ProcessMemoryRegionKind::SharedLibrary => {
                overview.shared_library_bytes = overview
                    .shared_library_bytes
                    .saturating_add(region.size_bytes);
            }

            ProcessMemoryRegionKind::MappedFile => {
                overview.mapped_file_bytes =
                    overview.mapped_file_bytes.saturating_add(region.size_bytes);
            }

            ProcessMemoryRegionKind::Stack => {
                overview.stack_bytes = overview.stack_bytes.saturating_add(region.size_bytes);
            }

            ProcessMemoryRegionKind::ExecutableImage => {
                overview.executable_image_bytes = overview
                    .executable_image_bytes
                    .saturating_add(region.size_bytes);
            }

            ProcessMemoryRegionKind::Special => {
                overview.special_bytes = overview.special_bytes.saturating_add(region.size_bytes);
            }

            ProcessMemoryRegionKind::Other => {}
        }
    }

    overview
}

fn read_executable(pid: u32) -> Option<String> {
    fs::read_link(format!("{PROC_ROOT}/{pid}/exe"))
        .ok()
        .map(|path| path.to_string_lossy().into_owned())
}

fn read_process_name(pid: u32) -> Option<String> {
    fs::read_to_string(format!("{PROC_ROOT}/{pid}/comm"))
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn unix_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn required<'a>(value: Option<&'a str>, field: &str) -> io::Result<&'a str> {
    value.ok_or_else(|| invalid_data(format!("mapping missing {field}")))
}

fn invalid_data(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_heap_mapping() {
        let line = "555555559000-55555557a000 rw-p 00000000 00:00 0 [heap]";

        let region = parse_maps_line(line, None).unwrap();

        assert_eq!(region.kind, ProcessMemoryRegionKind::Heap);

        assert!(region.permissions.readable);

        assert!(region.permissions.writable);

        assert!(!region.permissions.executable);
    }

    #[test]
    fn parses_shared_library_mapping() {
        let line = "7f091b600000-7f091b625000 r-xp 00001000 08:02 12345 /usr/lib/libc.so.6";

        let region = parse_maps_line(line, None).unwrap();

        assert_eq!(region.kind, ProcessMemoryRegionKind::SharedLibrary);

        assert!(region.permissions.executable);
    }

    #[test]
    fn parses_anonymous_mapping() {
        let line = "7f091c000000-7f091c200000 rw-p 00000000 00:00 0";

        let region = parse_maps_line(line, None).unwrap();

        assert_eq!(region.kind, ProcessMemoryRegionKind::Anonymous);

        assert_eq!(region.pathname, None);
    }

    #[test]
    fn preserves_path_with_spaces() {
        let line =
            "7f091d000000-7f091d100000 r--p 00000000 08:02 444 /opt/Wyn Observe/data file.bin";

        let region = parse_maps_line(line, None).unwrap();

        assert_eq!(
            region.pathname.as_deref(),
            Some("/opt/Wyn Observe/data file.bin")
        );
    }

    #[test]
    fn detects_writable_executable_mapping() {
        let line = "7f091e000000-7f091e100000 rwxp 00000000 00:00 0";

        let region = parse_maps_line(line, None).unwrap();

        assert!(region.permissions.writable);

        assert!(region.permissions.executable);
    }
}
