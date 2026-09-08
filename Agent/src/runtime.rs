use std::{
    sync::{Arc, RwLock},
    thread,
    time::Duration,
};

use wyn_protocol::TelemetrySnapshot;

use crate::{protocol, telemetry::TelemetryCollector};

const SAMPLE_INTERVAL: Duration = Duration::from_millis(500);

#[derive(Debug, Clone)]
pub struct PublishedSnapshot {
    pub sequence: u64,
    pub snapshot: TelemetrySnapshot,
}

#[derive(Clone, Default)]
pub struct TelemetryRuntime {
    latest: Arc<RwLock<Option<PublishedSnapshot>>>,
}

impl TelemetryRuntime {
    pub fn start() -> Self {
        let runtime = Self::default();

        let publisher = runtime.clone();

        thread::spawn(move || {
            sampling_loop(publisher);
        });

        runtime
    }

    pub fn latest(&self) -> Option<PublishedSnapshot> {
        self.latest
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    fn publish(&self, published: PublishedSnapshot) {
        let mut guard = self
            .latest
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        *guard = Some(published);
    }
}

fn sampling_loop(runtime: TelemetryRuntime) {
    let mut telemetry = TelemetryCollector::new();

    let mut sequence = 0_u64;

    loop {
        let internal = telemetry.sample();

        let snapshot = protocol::to_wire(&internal);

        sequence = sequence.wrapping_add(1);

        runtime.publish(PublishedSnapshot { sequence, snapshot });

        thread::sleep(SAMPLE_INTERVAL);
    }
}
