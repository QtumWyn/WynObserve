use std::io;

#[cfg(target_os = "linux")]
pub fn read_process_bytes(pid: u32, address: u64, len: usize) -> io::Result<Vec<u8>> {
    let mut buffer = vec![0_u8; len];

    let local = libc::iovec {
        iov_base: buffer.as_mut_ptr().cast(),
        iov_len: buffer.len(),
    };

    let remote = libc::iovec {
        iov_base: address as usize as *mut libc::c_void,
        iov_len: len,
    };

    let bytes_read =
        unsafe { libc::process_vm_readv(pid as libc::pid_t, &local, 1, &remote, 1, 0) };

    if bytes_read < 0 {
        return Err(io::Error::last_os_error());
    }

    buffer.truncate(bytes_read as usize);

    Ok(buffer)
}

#[cfg(target_os = "windows")]
pub fn read_process_bytes(pid: u32, address: u64, len: usize) -> io::Result<Vec<u8>> {
    use crate::windows::Handle;
    use windows_sys::Win32::System::{
        Diagnostics::Debug::ReadProcessMemory,
        Threading::{OpenProcess, PROCESS_VM_READ},
    };
    let process = Handle::new(unsafe { OpenProcess(PROCESS_VM_READ, 0, pid) })?;
    let mut bytes = vec![0; len];
    let mut count = 0;
    let ok = unsafe {
        ReadProcessMemory(
            process.0,
            address as usize as *const _,
            bytes.as_mut_ptr().cast(),
            len,
            &mut count,
        )
    };
    if ok == 0 && count == 0 {
        return Err(io::Error::last_os_error());
    }
    bytes.truncate(count);
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    #[test]
    fn reads_own_process_bytes() {
        let bytes = [0x12u8, 0x34, 0x56, 0x78];
        let read =
            super::read_process_bytes(std::process::id(), bytes.as_ptr() as u64, bytes.len())
                .unwrap();
        assert_eq!(read, bytes);
    }
    #[test]
    fn invalid_process_returns_error() {
        assert!(super::read_process_bytes(u32::MAX, 0, 16).is_err());
    }
}
