use crate::model::SystemSnapshot;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MachineOrigin {
    Local,
    Agent { endpoint: String },
}

impl MachineOrigin {
    pub fn label(&self) -> String {
        match self {
            Self::Local => "LOCAL".to_string(),

            Self::Agent { endpoint } => {
                format!("AGENT // {endpoint}")
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct CpuPackageTopology {
    pub package_id: usize,
    pub model: String,
    pub physical_cores: usize,
    pub logical_cpu_ids: Vec<usize>,
}

#[derive(Debug, Clone)]
pub struct FleetMachine {
    pub id: String,
    pub name: String,
    pub role: String,
    pub online: bool,
    pub origin: MachineOrigin,
    pub agent_version: Option<String>,

    pub system: SystemSnapshot,

    pub cpu_packages: Vec<CpuPackageTopology>,
}

#[derive(Debug, Clone, Default)]
pub struct FleetState {
    pub machines: Vec<FleetMachine>,
}

impl FleetState {
    pub fn machine(&self, id: &str) -> Option<&FleetMachine> {
        self.machines.iter().find(|machine| machine.id == id)
    }

    pub fn first_online(&self) -> Option<&FleetMachine> {
        self.machines.iter().find(|machine| machine.online)
    }
}
