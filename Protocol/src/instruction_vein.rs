use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstructionVeinSample {
    pub ip: u64,
    pub pid: u32,
    pub tid: u32,
    pub cpu: u32,
    pub perf_time: u64,

    pub bytes: Vec<u8>,
    pub instruction: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstructionVeinBatch {
    pub pid: u32,
    pub samples: Vec<InstructionVeinSample>,
}
