//! Small diagnostic for the same transport used by the Tauri command.
use std::{
    thread,
    time::{Duration, Instant},
};
use wyn_observatory_core::FleetClient;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let endpoint = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:5769".into());
    let client = FleetClient::connect(endpoint);
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let feed = client.snapshot();
        if !feed.stale && feed.machines.iter().any(|m| m.snapshot.is_some()) {
            println!("{}", serde_json::to_string(&feed)?);
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(format!("No telemetry received: {:?}", feed.error).into());
        }
        thread::sleep(Duration::from_millis(100));
    }
}
