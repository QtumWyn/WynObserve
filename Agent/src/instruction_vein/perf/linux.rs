use std::{
    io,
    mem::size_of,
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
    ptr::NonNull,
    sync::atomic::{Ordering, fence},
};

use perf_event_open_sys::bindings::PERF_RECORD_SAMPLE;
use perf_event_open_sys::{
    bindings::{
        PERF_COUNT_SW_TASK_CLOCK, PERF_SAMPLE_CPU, PERF_SAMPLE_IP, PERF_SAMPLE_TID,
        PERF_SAMPLE_TIME, PERF_TYPE_SOFTWARE, perf_event_attr, perf_event_mmap_page,
    },
    ioctls, perf_event_open,
};

const DATA_PAGES: usize = 8;

#[derive(Debug, Clone, Copy)]
pub(crate) struct PerfSample {
    pub ip: u64,
    pub pid: u32,
    pub tid: u32,
    pub time: u64,
    pub cpu: u32,
}

#[derive(Debug)]
pub(crate) struct PerfBuffer {
    fd: OwnedFd,
    mapping: NonNull<libc::c_void>,
    mapping_len: usize,
    page_size: usize,
}

impl Drop for PerfBuffer {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.mapping.as_ptr(), self.mapping_len);
        }
    }
}

impl PerfBuffer {
    fn metadata(&self) -> &perf_event_mmap_page {
        unsafe { &*(self.mapping.as_ptr() as *const perf_event_mmap_page) }
    }

    pub fn data_head(&self) -> u64 {
        let metadata = self.mapping.as_ptr() as *const perf_event_mmap_page;

        let head = unsafe { std::ptr::read_volatile(std::ptr::addr_of!((*metadata).data_head)) };

        fence(Ordering::Acquire);

        head
    }

    fn data_tail(&self) -> u64 {
        let metadata = self.mapping.as_ptr() as *const perf_event_mmap_page;

        unsafe { std::ptr::read_volatile(std::ptr::addr_of!((*metadata).data_tail)) }
    }

    fn set_data_tail(&mut self, tail: u64) {
        std::sync::atomic::fence(std::sync::atomic::Ordering::Release);

        let metadata = self.mapping.as_ptr() as *mut perf_event_mmap_page;

        unsafe {
            std::ptr::write_volatile(std::ptr::addr_of_mut!((*metadata).data_tail), tail);
        }
    }

    pub fn fd(&self) -> i32 {
        self.fd.as_raw_fd()
    }

    pub fn reset(&self) -> io::Result<()> {
        let result = unsafe { ioctls::RESET(self.fd(), 0) };

        if result < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    pub fn enable(&self) -> io::Result<()> {
        let result = unsafe { ioctls::ENABLE(self.fd(), 0) };

        if result < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    pub fn disable(&self) -> io::Result<()> {
        let result = unsafe { ioctls::DISABLE(self.fd(), 0) };

        if result < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    fn copy_from_ring(&self, absolute_offset: u64, output: &mut [u8]) {
        let metadata = self.metadata();

        let data_offset = metadata.data_offset as usize;

        let data_size = metadata.data_size as usize;

        let ring_offset = absolute_offset as usize % data_size;

        let first_len = output.len().min(data_size - ring_offset);

        let data_start = unsafe { self.mapping.as_ptr().cast::<u8>().add(data_offset) };

        unsafe {
            std::ptr::copy_nonoverlapping(
                data_start.add(ring_offset),
                output.as_mut_ptr(),
                first_len,
            );
        }

        let remaining = output.len() - first_len;

        if remaining > 0 {
            unsafe {
                std::ptr::copy_nonoverlapping(
                    data_start,
                    output.as_mut_ptr().add(first_len),
                    remaining,
                );
            }
        }
    }

    pub fn drain_samples(&mut self) -> io::Result<Vec<PerfSample>> {
        const HEADER_SIZE: usize = 8;
        const SAMPLE_SIZE: usize = 40;

        let head = self.data_head();

        let mut tail = self.data_tail();

        let mut samples = Vec::new();

        while tail < head {
            /*
             * Read perf_event_header:
             *
             * u32 type
             * u16 misc
             * u16 size
             */
            let mut header = [0_u8; HEADER_SIZE];

            self.copy_from_ring(tail, &mut header);

            let record_type = u32::from_ne_bytes(header[0..4].try_into().unwrap());

            let record_size = u16::from_ne_bytes(header[6..8].try_into().unwrap()) as usize;

            if record_size < HEADER_SIZE {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "perf record smaller than header",
                ));
            }

            if tail + record_size as u64 > head {
                break;
            }

            if record_type == PERF_RECORD_SAMPLE as u32 {
                if record_size < SAMPLE_SIZE {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "sample record too small: \
                             {record_size}"
                        ),
                    ));
                }

                let mut record = vec![0_u8; record_size];

                self.copy_from_ring(tail, &mut record);

                let ip = u64::from_ne_bytes(record[8..16].try_into().unwrap());

                let pid = u32::from_ne_bytes(record[16..20].try_into().unwrap());

                let tid = u32::from_ne_bytes(record[20..24].try_into().unwrap());

                let time = u64::from_ne_bytes(record[24..32].try_into().unwrap());

                let cpu = u32::from_ne_bytes(record[32..36].try_into().unwrap());

                samples.push(PerfSample {
                    ip,
                    pid,
                    tid,
                    time,
                    cpu,
                });
            }

            tail += record_size as u64;
        }

        self.set_data_tail(tail);

        Ok(samples)
    }
}

pub fn map_probe_for_tid(tid: u32) -> io::Result<PerfBuffer> {
    let fd = open_probe_for_tid(tid)?;

    let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };

    if page_size <= 0 {
        return Err(io::Error::last_os_error());
    }

    let page_size = page_size as usize;

    let mapping_len = page_size * (1 + DATA_PAGES);

    let mapping = unsafe {
        libc::mmap(
            std::ptr::null_mut(),
            mapping_len,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_SHARED,
            fd.as_raw_fd(),
            0,
        )
    };

    if mapping == libc::MAP_FAILED {
        return Err(io::Error::last_os_error());
    }

    let mapping =
        NonNull::new(mapping).ok_or_else(|| io::Error::other("perf mmap returned null"))?;

    Ok(PerfBuffer {
        fd,
        mapping,
        mapping_len,
        page_size,
    })
}

pub fn open_probe_for_tid(tid: u32) -> io::Result<OwnedFd> {
    let mut attrs = perf_event_attr::default();

    attrs.size = size_of::<perf_event_attr>() as u32;

    attrs.type_ = PERF_TYPE_SOFTWARE;

    attrs.config = PERF_COUNT_SW_TASK_CLOCK as u64;

    attrs.set_freq(1);
    attrs.sample_freq = 1_000;

    attrs.sample_type = (PERF_SAMPLE_IP as u64)
        | (PERF_SAMPLE_TID as u64)
        | (PERF_SAMPLE_TIME as u64)
        | (PERF_SAMPLE_CPU as u64);

    attrs.set_disabled(1);
    attrs.set_exclude_kernel(1);
    attrs.set_exclude_hv(1);

    let fd = unsafe { perf_event_open(&mut attrs, tid as i32, -1, -1, 0) };

    if fd < 0 {
        return Err(io::Error::last_os_error());
    }

    Ok(unsafe { OwnedFd::from_raw_fd(fd) })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::fd::AsRawFd;

    fn current_tid() -> u32 {
        (unsafe { libc::syscall(libc::SYS_gettid) } as u32)
    }

    #[test]
    fn opens_perf_event() {
        let fd = open_probe_for_tid(current_tid()).expect("failed to open perf event");

        assert!(fd.as_raw_fd() >= 0);

        println!("Instruction Vein // perf FD // {}", fd.as_raw_fd());
    }

    #[test]
    fn perf_ring_receives_data() {
        let buffer = map_probe_for_tid(current_tid()).expect("failed to map perf buffer");

        buffer.reset().expect("failed to reset perf event");

        buffer.enable().expect("failed to enable perf event");

        let before = buffer.data_head();

        let start_time = std::time::Instant::now();

        let mut value = 0_u64;

        while start_time.elapsed() < std::time::Duration::from_millis(50) {
            value = value.wrapping_add(value.rotate_left(7).wrapping_add(0x9E37_79B9));
        }

        std::hint::black_box(value);

        buffer.disable().expect("failed to disable perf event");

        let after = buffer.data_head();

        println!("Instruction Vein // perf ring // head {before} -> {after}");

        assert!(after > before, "perf ring received no sample data");
    }

    #[test]
    fn captures_real_instruction_sample() {
        let mut buffer = map_probe_for_tid(current_tid()).expect("failed to map perf buffer");

        buffer.reset().expect("failed to reset perf event");

        buffer.enable().expect("failed to enable perf event");

        let start = std::time::Instant::now();

        let mut value = 0x1234_5678_u64;

        while start.elapsed() < std::time::Duration::from_millis(50) {
            value = value
                .rotate_left(7)
                .wrapping_mul(0x9E37_79B9)
                .wrapping_add(1);
        }

        std::hint::black_box(value);

        buffer.disable().expect("failed to disable perf event");

        let samples = buffer
            .drain_samples()
            .expect("failed to decode perf samples");

        let sample = samples
            .iter()
            .find(|sample| sample.ip != 0)
            .expect("no sample with a valid instruction pointer");

        let bytes = crate::instruction_vein::read_process_bytes(sample.pid, sample.ip, 15)
            .expect("failed to read instruction bytes");

        println!("Instruction Vein // {} sample(s)", samples.len());

        let decoded = crate::instruction_vein::decode_one(sample.ip, &bytes)
            .expect("failed to decode instruction");

        let hex = decoded
            .bytes
            .iter()
            .map(|byte| format!("{byte:02X}"))
            .collect::<Vec<_>>()
            .join(" ");

        println!(
            "Instruction Vein // CPU {:02} \
     // PID {} // RIP 0x{:016X}",
            sample.cpu, sample.pid, sample.ip,
        );

        println!("Instruction Vein // {} // {}", hex, decoded.text,);

        for sample in samples.iter().take(5) {
            println!(
                "CPU {:02} // PID {} // TID {} \
             // RIP 0x{:016X} // TIME {}",
                sample.cpu, sample.pid, sample.tid, sample.ip, sample.time,
            );
        }

        assert!(!samples.is_empty(), "no instruction samples captured");

        assert!(
            samples.iter().any(|sample| sample.ip != 0),
            "all sampled instruction pointers were zero"
        );
    }
}
