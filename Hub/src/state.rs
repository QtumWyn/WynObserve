use std::{
    collections::{HashMap, hash_map::Entry},
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

use wyn_protocol::{FleetMachineState, MachineIdentity, MachineStatus, TelemetrySnapshot};

#[derive(Debug, Clone)]
pub struct AgentRecord {
    pub machine: MachineIdentity,

    pub latest_sequence: Option<u64>,

    pub latest_snapshot: Option<TelemetrySnapshot>,

    pub last_seen: Instant,
}

#[derive(Debug, Clone, Copy)]
pub enum SnapshotUpdate {
    Accepted,

    UnknownMachine,

    StaleSequence { previous: u64 },
}

#[derive(Clone, Default)]
pub struct FleetRegistry {
    inner: Arc<RwLock<HashMap<String, AgentRecord>>>,
}

impl FleetRegistry {
    pub fn register(&self, machine: MachineIdentity) {
        let machine_id = machine.machine_id.clone();

        let now = Instant::now();

        let mut guard = self
            .inner
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        match guard.entry(machine_id) {
            Entry::Occupied(mut entry) => {
                let record = entry.get_mut();

                record.machine = machine;

                // A new Hello begins a new agent session.
                // Sequence numbers are monotonic within a session,
                // not across process restarts.
                record.latest_sequence = None;

                record.last_seen = now;
            }

            Entry::Vacant(entry) => {
                entry.insert(AgentRecord {
                    machine,

                    latest_sequence: None,

                    latest_snapshot: None,

                    last_seen: now,
                });
            }
        }
    }

    pub fn update_snapshot(
        &self,
        machine_id: &str,
        sequence: u64,
        snapshot: TelemetrySnapshot,
    ) -> SnapshotUpdate {
        let mut guard = self
            .inner
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let Some(record) = guard.get_mut(machine_id) else {
            return SnapshotUpdate::UnknownMachine;
        };

        if let Some(previous) = record.latest_sequence {
            if sequence <= previous {
                return SnapshotUpdate::StaleSequence { previous };
            }
        }

        record.latest_sequence = Some(sequence);

        record.latest_snapshot = Some(snapshot);

        record.last_seen = Instant::now();

        SnapshotUpdate::Accepted
    }

    pub fn machine_count(&self) -> usize {
        self.inner
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .len()
    }

    pub fn fleet_snapshot(&self) -> Vec<FleetMachineState> {
        let guard = self
            .inner
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let mut machines = guard
            .values()
            .map(|record| {
                let age = record.last_seen.elapsed();

                let status = if age < Duration::from_secs(3) {
                    MachineStatus::Online
                } else if age < Duration::from_secs(10) {
                    MachineStatus::Stale
                } else {
                    MachineStatus::Offline
                };

                FleetMachineState {
                    machine: record.machine.clone(),

                    status,

                    latest_sequence: record.latest_sequence,

                    last_seen_ms: age.as_millis().min(u64::MAX as u128) as u64,

                    snapshot: record.latest_snapshot.clone(),
                }
            })
            .collect::<Vec<_>>();

        // Deterministic ordering prevents machines from
        // dancing around the Fleet UI because HashMap
        // iteration order is intentionally unstable.
        machines.sort_by(|left, right| left.machine.machine_id.cmp(&right.machine.machine_id));

        machines
    }
}
