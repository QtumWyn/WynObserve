use super::{
    decode_one,
    perf::{PerfBuffer, PerfSample, map_probe_for_tid},
    read_process_bytes,
};

#[derive(Debug, Clone)]
pub struct InstructionSample {
    pub ip: u64,
    pub pid: u32,
    pub tid: u32,
    pub cpu: u32,
    pub timestamp: u64,
    pub bytes: Vec<u8>,
    pub instruction: String,
}

impl InstructionSample {
    fn from_perf(sample: PerfSample) -> Result<Self, String> {
        let bytes = read_process_bytes(sample.pid, sample.ip, 15)
            .map_err(|error| format!("failed to read bytes at 0x{:016X}: {error}", sample.ip,))?;

        let decoded = decode_one(sample.ip, &bytes)?;

        Ok(Self {
            ip: sample.ip,
            pid: sample.pid,
            tid: sample.tid,
            cpu: sample.cpu,
            timestamp: sample.time,
            bytes: decoded.bytes,
            instruction: decoded.text,
        })
    }
}

pub struct InstructionSampler {
    tid: u32,
    buffer: PerfBuffer,
    active: bool,
}

impl InstructionSampler {
    pub fn start_for_tid(tid: u32) -> Result<Self, String> {
        let buffer = map_probe_for_tid(tid).map_err(|error| {
            format!(
                "failed to create perf buffer \
                         for TID {tid}: {error}"
            )
        })?;

        buffer
            .reset()
            .map_err(|error| format!("failed to reset TID {tid}: {error}"))?;

        buffer
            .enable()
            .map_err(|error| format!("failed to enable TID {tid}: {error}"))?;

        Ok(Self {
            tid,
            buffer,
            active: true,
        })
    }

    pub fn tid(&self) -> u32 {
        self.tid
    }

    pub fn poll(&mut self) -> Result<Vec<InstructionSample>, String> {
        if !self.active {
            return Err("instruction sampler is stopped".to_string());
        }

        let raw_samples = self
            .buffer
            .drain_samples()
            .map_err(|error| format!("failed to drain perf samples: {error}"))?;

        let mut samples = Vec::with_capacity(raw_samples.len());

        for raw in raw_samples {
            /*
             * A sampled address may disappear before
             * we inspect it, especially once we begin
             * observing arbitrary processes.
             *
             * One unreadable sample should not destroy
             * the entire live stream.
             */
            if let Ok(sample) = InstructionSample::from_perf(raw) {
                samples.push(sample);
            }
        }

        Ok(samples)
    }

    pub fn stop(&mut self) -> Result<(), String> {
        if !self.active {
            return Ok(());
        }

        self.buffer
            .disable()
            .map_err(|error| format!("failed to disable perf event: {error}"))?;

        self.active = false;

        Ok(())
    }
}

impl Drop for InstructionSampler {
    fn drop(&mut self) {
        if self.active {
            let _ = self.buffer.disable();
        }
    }
}

impl From<InstructionSample> for wyn_protocol::instruction_vein::InstructionVeinSample {
    fn from(sample: InstructionSample) -> Self {
        Self {
            ip: sample.ip,
            pid: sample.pid,
            tid: sample.tid,
            cpu: sample.cpu,
            perf_time: sample.timestamp,
            bytes: sample.bytes,
            instruction: sample.instruction,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn sampler_produces_decoded_instructions() {
        let tid = unsafe { libc::syscall(libc::SYS_gettid) } as u32;

        let mut sampler =
            InstructionSampler::start_for_tid(tid).expect("failed to start instruction sampler");
        let start = Instant::now();

        let mut value = 0x1234_5678_u64;

        while start.elapsed() < Duration::from_millis(50) {
            value = value
                .rotate_left(9)
                .wrapping_mul(0x9E37_79B9)
                .wrapping_add(7);
        }

        std::hint::black_box(value);

        let samples = sampler.poll().expect("failed to poll instruction sampler");

        sampler.stop().expect("failed to stop instruction sampler");

        println!(
            "Instruction Vein // sampler // {} decoded sample(s)",
            samples.len()
        );

        for sample in samples.iter().take(5) {
            let bytes = sample
                .bytes
                .iter()
                .map(|byte| format!("{byte:02X}"))
                .collect::<Vec<_>>()
                .join(" ");

            println!(
                "CPU {:02} // PID {} // TID {} \
         // RIP 0x{:016X} // {} // {}",
                sample.cpu, sample.pid, sample.tid, sample.ip, bytes, sample.instruction,
            );
        }

        assert!(
            !samples.is_empty(),
            "sampler returned no decoded instructions"
        );
    }

    #[test]
    fn samples_specific_thread() {
        use std::{
            sync::{
                Arc,
                atomic::{AtomicBool, Ordering},
                mpsc,
            },
            thread,
            time::Duration,
        };

        let running = Arc::new(AtomicBool::new(true));

        let worker_running = Arc::clone(&running);

        let (tx, rx) = mpsc::channel();

        let worker = thread::spawn(move || {
            let tid = unsafe { libc::syscall(libc::SYS_gettid) } as u32;

            tx.send(tid).unwrap();

            let mut value = 0x1234_5678_u64;

            while worker_running.load(Ordering::Relaxed) {
                value = value
                    .rotate_left(7)
                    .wrapping_mul(0x9E37_79B9)
                    .wrapping_add(3);

                std::hint::black_box(value);
            }
        });

        let tid = rx.recv().expect("worker did not report TID");

        let mut sampler =
            InstructionSampler::start_for_tid(tid).expect("failed to start targeted sampler");

        thread::sleep(Duration::from_millis(75));

        let samples = sampler.poll().expect("failed to poll targeted sampler");

        sampler.stop().expect("failed to stop sampler");

        running.store(false, Ordering::Relaxed);

        worker.join().unwrap();

        println!(
            "Instruction Vein // target TID {tid} \
         // {} decoded sample(s)",
            samples.len()
        );

        for sample in samples.iter().take(5) {
            println!(
                "CPU {:02} // PID {} // TID {} \
             // RIP 0x{:016X} // {}",
                sample.cpu, sample.pid, sample.tid, sample.ip, sample.instruction,
            );
        }

        assert!(!samples.is_empty(), "targeted sampler produced no samples");

        assert!(
            samples.iter().all(|sample| sample.tid == tid),
            "received samples from another thread"
        );
    }
}
