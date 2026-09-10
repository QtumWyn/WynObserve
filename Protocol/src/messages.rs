use serde::{Deserialize, Serialize};

use crate::process_memory::ProcessMemoryMap;
use crate::{InstructionVeinBatch, PROTOCOL_VERSION, TelemetrySnapshot};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineIdentity {
    pub machine_id: String,
    pub machine_name: String,

    #[serde(default = "default_machine_role")]
    pub role: String,

    pub os: String,
    pub architecture: String,

    pub agent_version: String,
}

fn default_machine_role() -> String {
    "workstation".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCommand {
    pub protocol_version: u16,
    pub request_id: u64,
    pub command: AgentCommandKind,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum AgentCommandKind {
    ProcessMemoryMap {
        pid: u32,
        expected_started_at_unix_ms: Option<u64>,
    },

    InstructionVeinSample {
        pid: u32,
        expected_started_at_unix_ms: Option<u64>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResponse {
    pub protocol_version: u16,
    pub request_id: u64,
    pub result: AgentResponseKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum AgentResponseKind {
    ProcessMemoryMap { map: ProcessMemoryMap },

    InstructionVein { batch: InstructionVeinBatch },

    Error { code: String, message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservatoryRequest {
    pub protocol_version: u16,

    pub request_id: u64,

    pub request: ObservatoryRequestKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "request", rename_all = "snake_case")]
pub enum ObservatoryRequestKind {
    ProcessMemoryMap {
        machine_id: String,

        pid: u32,

        expected_started_at_unix_ms: Option<u64>,
    },

    InstructionVeinSample {
        machine_id: String,
        pid: u32,
        expected_started_at_unix_ms: Option<u64>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum ObservatoryResponseKind {
    ProcessMemoryMap {
        machine_id: String,
        map: ProcessMemoryMap,
    },

    InstructionVein {
        machine_id: String,
        batch: InstructionVeinBatch,
    },

    Error {
        code: String,
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentHello {
    pub protocol_version: u16,

    pub machine: MachineIdentity,
}

impl AgentHello {
    pub fn new(machine: MachineIdentity) -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,

            machine,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentMessage {
    Hello {
        hello: AgentHello,
    },

    Snapshot {
        machine_id: String,
        sequence: u64,
        snapshot: TelemetrySnapshot,
    },

    Response {
        response: AgentResponse,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MachineStatus {
    Online,
    Stale,
    Offline,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetMachineState {
    pub machine: MachineIdentity,

    pub status: MachineStatus,

    pub latest_sequence: Option<u64>,

    /// Age of the latest successful agent communication
    /// when this Fleet packet was generated.
    pub last_seen_ms: u64,

    /// Last known telemetry remains available even if
    /// a machine becomes stale/offline.
    pub snapshot: Option<TelemetrySnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HubMessage {
    FleetSnapshot {
        protocol_version: u16,

        generated_at_unix_ms: u64,

        machines: Vec<FleetMachineState>,
    },
    Response {
        protocol_version: u16,
        request_id: u64,
        response: ObservatoryResponseKind,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_hello_round_trips_json() {
        let original = AgentHello::new(MachineIdentity {
            machine_id: "wyn-itpc".into(),
            machine_name: "Wyn-ITPC".into(),
            role: "workstation".into(),
            os: "linux".into(),
            architecture: "x86_64".into(),
            agent_version: "0.1.0".into(),
        });

        let json = serde_json::to_string(&original).unwrap();

        let decoded: AgentHello = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded.protocol_version, PROTOCOL_VERSION);

        assert_eq!(decoded.machine.machine_id, "wyn-itpc");
    }
}
