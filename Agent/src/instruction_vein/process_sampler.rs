use std::{
    collections::{HashMap, HashSet},
    fs, io,
};

use super::{InstructionSample, InstructionSampler};

pub struct ProcessInstructionSampler {
    pid: u32,

    samplers: HashMap<u32, InstructionSampler>,
}

fn read_tids(pid: u32) -> io::Result<HashSet<u32>> {
    let path = format!("/proc/{pid}/task");

    let mut tids = HashSet::new();

    for entry in fs::read_dir(path)? {
        let entry = entry?;

        let name = entry.file_name();

        let Some(name) = name.to_str() else {
            continue;
        };

        let Ok(tid) = name.parse::<u32>() else {
            continue;
        };

        tids.insert(tid);
    }

    Ok(tids)
}

impl ProcessInstructionSampler {
    pub fn start(pid: u32) -> Result<Self, String> {
        let tids = read_tids(pid).map_err(|error| {
            format!(
                "failed to enumerate threads \
                         for PID {pid}: {error}"
            )
        })?;

        if tids.is_empty() {
            return Err(format!("PID {pid} has no visible threads"));
        }

        let mut samplers = HashMap::new();

        for tid in tids {
            match InstructionSampler::start_for_tid(tid) {
                Ok(sampler) => {
                    samplers.insert(tid, sampler);
                }

                Err(error) => {
                    eprintln!(
                        "Instruction Vein // \
                         unable to sample TID {tid} \
                         // {error}"
                    );
                }
            }
        }

        if samplers.is_empty() {
            return Err(format!(
                "could not open any perf events \
                     for PID {pid}"
            ));
        }

        Ok(Self { pid, samplers })
    }

    fn sync_threads(&mut self) -> Result<(), String> {
        let current = read_tids(self.pid).map_err(|error| {
            format!(
                "failed to refresh threads \
                     for PID {}: {error}",
                self.pid,
            )
        })?;

        /*
         * Remove threads that no longer exist.
         */
        self.samplers.retain(|tid, _| current.contains(tid));

        /*
         * Add newly created threads.
         */
        for tid in current {
            if self.samplers.contains_key(&tid) {
                continue;
            }

            if let Ok(sampler) = InstructionSampler::start_for_tid(tid) {
                self.samplers.insert(tid, sampler);
            }
        }

        Ok(())
    }

    pub fn poll(&mut self) -> Result<Vec<InstructionSample>, String> {
        self.sync_threads()?;

        let mut samples = Vec::new();

        for sampler in self.samplers.values_mut() {
            match sampler.poll() {
                Ok(mut thread_samples) => {
                    samples.append(&mut thread_samples);
                }

                Err(error) => {
                    eprintln!(
                        "Instruction Vein // \
                     TID {} poll failed // {}",
                        sampler.tid(),
                        error,
                    );
                }
            }
        }

        /*
         * Give the caller a useful temporal order
         * rather than HashMap iteration order.
         */
        samples.sort_unstable_by_key(|sample| sample.timestamp);

        Ok(samples)
    }

    pub fn pid(&self) -> u32 {
        self.pid
    }

    pub fn thread_count(&self) -> usize {
        self.samplers.len()
    }

    pub fn stop(&mut self) {
        for sampler in self.samplers.values_mut() {
            let _ = sampler.stop();
        }

        self.samplers.clear();
    }
}

impl Drop for ProcessInstructionSampler {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::{
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        thread,
        time::Duration,
    };

    #[test]
    fn samples_whole_process() {
        let running = Arc::new(AtomicBool::new(true));

        let mut workers = Vec::new();

        for seed in 1..=3_u64 {
            let running = Arc::clone(&running);

            workers.push(thread::spawn(move || {
                let mut value = seed;

                while running.load(Ordering::Relaxed) {
                    value = value
                        .rotate_left(7)
                        .wrapping_mul(0x9E37_79B9)
                        .wrapping_add(seed);

                    std::hint::black_box(value);
                }
            }));
        }

        let pid = std::process::id();

        let mut sampler =
            ProcessInstructionSampler::start(pid).expect("failed to start process sampler");

        thread::sleep(Duration::from_millis(100));

        let samples = sampler.poll().expect("failed to poll process sampler");

        running.store(false, Ordering::Relaxed);

        for worker in workers {
            worker.join().unwrap();
        }

        println!(
            "Instruction Vein // PID {pid} \
             // {} thread sampler(s) \
             // {} decoded samples",
            sampler.thread_count(),
            samples.len(),
        );

        for sample in samples.iter().take(10) {
            let bytes = sample
                .bytes
                .iter()
                .map(|byte| format!("{byte:02X}"))
                .collect::<Vec<_>>()
                .join(" ");

            println!(
                "CPU {:02} // TID {} \
                 // RIP 0x{:016X} \
                 // {} // {}",
                sample.cpu, sample.tid, sample.ip, bytes, sample.instruction,
            );
        }

        assert!(
            !samples.is_empty(),
            "process sampler returned no instructions"
        );

        let tids = samples
            .iter()
            .map(|sample| sample.tid)
            .collect::<HashSet<_>>();

        assert!(
            tids.len() >= 2,
            "expected samples from multiple threads, \
             saw only {tids:?}"
        );
    }
}
