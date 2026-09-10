use std::io;

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
