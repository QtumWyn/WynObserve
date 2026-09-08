use std::{
    io::{self, Write},
    net::{TcpListener, TcpStream},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use wyn_protocol::{HubMessage, PROTOCOL_VERSION};

use crate::state::FleetRegistry;

const FLEET_INTERVAL: Duration = Duration::from_millis(500);

pub fn run(registry: FleetRegistry, listen_address: &str) -> io::Result<()> {
    let listener = TcpListener::bind(listen_address)?;

    println!("Hub // Observatory egress listening on {listen_address}");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let registry = registry.clone();

                thread::spawn(move || {
                    if let Err(error) = stream_fleet(stream, registry) {
                        eprintln!("Hub // Observatory client disconnected: {error}");
                    }
                });
            }

            Err(error) => {
                eprintln!("Hub // failed to accept Observatory client: {error}");
            }
        }
    }

    Ok(())
}

fn stream_fleet(mut stream: TcpStream, registry: FleetRegistry) -> io::Result<()> {
    let peer = stream.peer_addr()?;

    println!("Hub // Observatory client connected // {peer}");

    loop {
        let message = HubMessage::FleetSnapshot {
            protocol_version: PROTOCOL_VERSION,

            generated_at_unix_ms: unix_timestamp_ms(),

            machines: registry.fleet_snapshot(),
        };

        let json = serde_json::to_string(&message).map_err(io::Error::other)?;

        writeln!(stream, "{json}")?;

        stream.flush()?;

        thread::sleep(FLEET_INTERVAL);
    }
}

fn unix_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u64::MAX as u128) as u64
}
