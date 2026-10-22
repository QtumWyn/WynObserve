use crate::windows::{Handle, executable};
use std::{
    io,
    mem::{size_of, zeroed},
    time::{SystemTime, UNIX_EPOCH},
};
use windows_sys::Win32::System::{Memory::*, ProcessStatus::K32GetMappedFileNameW, Threading::*};
use wyn_protocol::process_memory::*;

pub(super) fn read_process_memory_map(pid: u32) -> io::Result<ProcessMemoryMap> {
    let process =
        Handle::new(unsafe { OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, 0, pid) })?;
    let exe = executable(process.0);
    let mut regions = Vec::new();
    let mut address = 0usize;
    let mut overview = ProcessMemoryOverview::default();
    loop {
        let mut info: MEMORY_BASIC_INFORMATION = unsafe { zeroed() };
        if unsafe {
            VirtualQueryEx(
                process.0,
                address as *const _,
                &mut info,
                size_of::<MEMORY_BASIC_INFORMATION>(),
            )
        } == 0
        {
            let error = io::Error::last_os_error();
            if error.raw_os_error() == Some(87) {
                break;
            } // end of address space
            return Err(error);
        }
        let start = info.BaseAddress as usize;
        let end = start
            .checked_add(info.RegionSize)
            .ok_or_else(|| io::Error::other("memory region overflow"))?;
        if end <= address {
            return Err(io::Error::other("non-progressing memory map"));
        }
        address = end;
        if info.State == MEM_FREE {
            continue;
        }
        let accessible = info.State == MEM_COMMIT && info.Protect & PAGE_GUARD == 0;
        let protect = info.Protect & 0xff;
        let permissions = ProcessMemoryPermissions {
            readable: accessible
                && matches!(
                    protect,
                    PAGE_READONLY
                        | PAGE_READWRITE
                        | PAGE_WRITECOPY
                        | PAGE_EXECUTE_READ
                        | PAGE_EXECUTE_READWRITE
                        | PAGE_EXECUTE_WRITECOPY
                ),
            writable: accessible
                && matches!(
                    protect,
                    PAGE_READWRITE
                        | PAGE_WRITECOPY
                        | PAGE_EXECUTE_READWRITE
                        | PAGE_EXECUTE_WRITECOPY
                ),
            executable: accessible
                && matches!(
                    protect,
                    PAGE_EXECUTE
                        | PAGE_EXECUTE_READ
                        | PAGE_EXECUTE_READWRITE
                        | PAGE_EXECUTE_WRITECOPY
                ),
            private: info.Type == MEM_PRIVATE,
            shared: info.Type == MEM_MAPPED || info.Type == MEM_IMAGE,
        };
        let mut name = vec![0u16; 32768];
        let count = unsafe {
            K32GetMappedFileNameW(
                process.0,
                info.BaseAddress,
                name.as_mut_ptr(),
                name.len() as u32,
            )
        };
        let pathname = (count > 0).then(|| String::from_utf16_lossy(&name[..count as usize]));
        // VirtualQueryEx does not identify stacks/heaps or reveal file offsets/inodes.
        // Keep those unknown rather than guessing from writable/private mappings.
        let kind = if info.Type == MEM_IMAGE {
            if pathname
                .as_ref()
                .is_some_and(|s| s.to_ascii_lowercase().ends_with(".exe"))
            {
                ProcessMemoryRegionKind::ExecutableImage
            } else {
                ProcessMemoryRegionKind::SharedLibrary
            }
        } else if info.Type == MEM_MAPPED {
            ProcessMemoryRegionKind::MappedFile
        } else if info.Type == MEM_PRIVATE {
            ProcessMemoryRegionKind::Anonymous
        } else {
            ProcessMemoryRegionKind::Other
        };
        let bytes = info.RegionSize as u64;
        match kind {
            ProcessMemoryRegionKind::ExecutableImage => overview.executable_image_bytes += bytes,
            ProcessMemoryRegionKind::SharedLibrary => overview.shared_library_bytes += bytes,
            ProcessMemoryRegionKind::MappedFile => overview.mapped_file_bytes += bytes,
            ProcessMemoryRegionKind::Anonymous => overview.anonymous_bytes += bytes,
            _ => (),
        }
        regions.push(ProcessMemoryRegion {
            start_address: start as u64,
            end_address: end as u64,
            size_bytes: bytes,
            permissions,
            offset: 0,
            device_major: 0,
            device_minor: 0,
            inode: 0,
            pathname,
            kind,
        });
    }
    let process_name = exe
        .as_deref()
        .and_then(|s| s.rsplit('\\').next())
        .map(str::to_owned)
        .unwrap_or_else(|| format!("PID {pid}"));
    Ok(ProcessMemoryMap {
        pid,
        process_name,
        executable: exe,
        captured_at_unix_ms: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64,
        total_virtual_bytes: regions.iter().map(|r| r.size_bytes).sum(),
        region_count: regions.len(),
        writable_executable_regions: regions
            .iter()
            .filter(|r| r.permissions.writable && r.permissions.executable)
            .count(),
        overview,
        regions,
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn maps_current_process() {
        let map = super::read_process_memory_map(std::process::id()).unwrap();
        assert!(!map.regions.is_empty());
        assert!(map.regions.iter().all(|r| r.end_address > r.start_address));
    }
}
