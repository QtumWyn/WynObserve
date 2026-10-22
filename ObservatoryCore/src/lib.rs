//! UI-independent, read-only Observatory feed. The wire contract stays in Protocol.
use serde::Serialize;
use std::{
    io::{self, Read},
    net::{TcpStream, ToSocketAddrs},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use wyn_protocol::{FleetMachineState, HubMessage, PROTOCOL_VERSION, TELEMETRY_SCHEMA_VERSION};

const POLL: Duration = Duration::from_millis(100);
const MAX_PACKET: usize = 16 * 1024 * 1024;
pub const STALE_AFTER_MS: u64 = 5_000;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Connection {
    Connecting,
    Connected,
    Disconnected,
}

#[derive(Debug, Clone, Serialize)]
pub struct FleetFeed {
    pub endpoint: String,
    pub connection: Connection,
    pub error: Option<String>,
    pub received_age_ms: Option<u64>,
    pub stale: bool,
    pub machines: Vec<FleetMachineState>,
}

struct Shared {
    feed: FleetFeed,
    received: Option<Instant>,
}
pub struct FleetClient {
    shared: Arc<Mutex<Shared>>,
    stopping: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl FleetClient {
    pub fn connect(endpoint: String) -> Self {
        let shared = Arc::new(Mutex::new(Shared {
            feed: FleetFeed {
                endpoint: endpoint.clone(),
                connection: Connection::Connecting,
                error: None,
                received_age_ms: None,
                stale: true,
                machines: vec![],
            },
            received: None,
        }));
        let stopping = Arc::new(AtomicBool::new(false));
        let state = Arc::clone(&shared);
        let stop = Arc::clone(&stopping);
        let worker = thread::spawn(move || {
            while !stop.load(Ordering::Acquire) {
                let result =
                    connect(&endpoint).and_then(|stream| read_session(stream, &state, &stop));
                if stop.load(Ordering::Acquire) {
                    break;
                }
                let mut state = state.lock().unwrap_or_else(|e| e.into_inner());
                state.feed.connection = Connection::Disconnected;
                state.feed.error = result.err().map(|e| e.to_string());
                drop(state);
                for _ in 0..10 {
                    if stop.load(Ordering::Acquire) {
                        return;
                    }
                    thread::sleep(POLL);
                }
            }
        });
        Self {
            shared,
            stopping,
            worker: Some(worker),
        }
    }

    pub fn snapshot(&self) -> FleetFeed {
        let state = self.shared.lock().unwrap_or_else(|e| e.into_inner());
        let mut feed = state.feed.clone();
        feed.received_age_ms = state
            .received
            .map(|t| t.elapsed().as_millis().min(u64::MAX as u128) as u64);
        feed.stale = !matches!(feed.connection, Connection::Connected)
            || feed.received_age_ms.is_none_or(|age| age > STALE_AFTER_MS);
        for machine in &mut feed.machines {
            machine.last_seen_ms = machine
                .last_seen_ms
                .saturating_add(feed.received_age_ms.unwrap_or(0));
        }
        feed
    }
}

impl Drop for FleetClient {
    fn drop(&mut self) {
        self.stopping.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn connect(endpoint: &str) -> io::Result<TcpStream> {
    let mut last = io::Error::new(
        io::ErrorKind::AddrNotAvailable,
        "Hub address did not resolve",
    );
    for address in endpoint.to_socket_addrs()? {
        match TcpStream::connect_timeout(&address, Duration::from_secs(1)) {
            Ok(stream) => {
                stream.set_read_timeout(Some(POLL))?;
                return Ok(stream);
            }
            Err(error) => last = error,
        }
    }
    Err(last)
}

fn read_session(
    mut stream: TcpStream,
    shared: &Mutex<Shared>,
    stop: &AtomicBool,
) -> io::Result<()> {
    let mut pending = Vec::new();
    let mut bytes = [0; 8192];
    while !stop.load(Ordering::Acquire) {
        match stream.read(&mut bytes) {
            Ok(0) => {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "Hub disconnected",
                ));
            }
            Ok(count) => {
                pending.extend_from_slice(&bytes[..count]);
                while let Some(end) = pending.iter().position(|b| *b == b'\n') {
                    if end > MAX_PACKET {
                        return Err(io::Error::other("Hub packet too large"));
                    }
                    let packet = pending.drain(..=end).collect::<Vec<_>>();
                    if packet.iter().all(u8::is_ascii_whitespace) {
                        continue;
                    }
                    if let Some(machines) = decode_packet(&packet)? {
                        let mut state = shared.lock().unwrap_or_else(|e| e.into_inner());
                        state.feed.machines = machines;
                        state.feed.connection = Connection::Connected;
                        state.feed.error = None;
                        state.received = Some(Instant::now());
                    }
                }
                if pending.len() > MAX_PACKET {
                    return Err(io::Error::other("Hub packet too large"));
                }
            }
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::WouldBlock
                        | io::ErrorKind::TimedOut
                        | io::ErrorKind::Interrupted
                ) =>
            {
                ()
            }
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

fn decode_packet(bytes: &[u8]) -> io::Result<Option<Vec<FleetMachineState>>> {
    let message: HubMessage = serde_json::from_slice(bytes).map_err(io::Error::other)?;
    match message {
        HubMessage::FleetSnapshot {
            protocol_version,
            machines,
            ..
        } => {
            if protocol_version != PROTOCOL_VERSION {
                return Err(io::Error::other("Unsupported Hub protocol version"));
            }
            if machines
                .iter()
                .filter_map(|m| m.snapshot.as_ref())
                .any(|s| s.schema_version != TELEMETRY_SCHEMA_VERSION)
            {
                return Err(io::Error::other("Unsupported telemetry schema version"));
            }
            Ok(Some(machines))
        }
        HubMessage::Response { .. } => Ok(None),
    }
}

/// An explicit CLI/environment configuration takes precedence over the local-dev default.
/// Invalid explicit configuration is an error, never a silent connection to another Hub.
pub fn configured_endpoint() -> Result<String, String> {
    let mut args = std::env::args().skip(1);
    let mut path = None;
    while let Some(arg) = args.next() {
        if arg == "--config" {
            path = Some(args.next().ok_or("--config requires a path")?);
            break;
        }
    }
    if let Some(path) = path.or_else(|| std::env::var("WYNCOMMAND_OBSERVATORY_CONFIG").ok()) {
        let contents = std::fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))?;
        let config: toml::Value = toml::from_str(&contents).map_err(|e| e.to_string())?;
        return config
            .get("hub")
            .and_then(|c| c.get("endpoint"))
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
            .map(str::to_owned)
            .ok_or_else(|| "Missing hub.endpoint".into());
    }
    Ok("127.0.0.1:5769".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::Write, net::TcpListener};

    #[test]
    fn rejects_protocol_mismatch() {
        let message = HubMessage::FleetSnapshot {
            protocol_version: PROTOCOL_VERSION + 1,
            generated_at_unix_ms: 0,
            machines: vec![],
        };
        assert!(decode_packet(&serde_json::to_vec(&message).unwrap()).is_err());
    }

    #[test]
    fn preserves_offline_entries_and_ages_retained_data() {
        let machine = FleetMachineState {
            machine: wyn_protocol::MachineIdentity {
                machine_id: "offline".into(),
                machine_name: "Offline machine".into(),
                role: "workstation".into(),
                os: "windows".into(),
                architecture: "x86_64".into(),
                agent_version: "test".into(),
            },
            status: wyn_protocol::MachineStatus::Offline,
            latest_sequence: None,
            last_seen_ms: 100,
            snapshot: None,
        };
        let packet = HubMessage::FleetSnapshot {
            protocol_version: PROTOCOL_VERSION,
            generated_at_unix_ms: 0,
            machines: vec![machine],
        };
        let machines = decode_packet(&serde_json::to_vec(&packet).unwrap())
            .unwrap()
            .unwrap();
        let client = FleetClient {
            shared: Arc::new(Mutex::new(Shared {
                feed: FleetFeed {
                    endpoint: "test".into(),
                    connection: Connection::Connected,
                    error: None,
                    received_age_ms: None,
                    stale: false,
                    machines,
                },
                received: Some(Instant::now() - Duration::from_secs(6)),
            })),
            stopping: Arc::new(AtomicBool::new(false)),
            worker: None,
        };
        let feed = client.snapshot();
        assert!(feed.stale);
        assert_eq!(feed.machines.len(), 1);
        assert!(feed.machines[0].snapshot.is_none());
        assert!(feed.machines[0].last_seen_ms >= 6100);
    }

    #[test]
    fn preserves_fragmented_packets_and_reports_disconnect() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let client = FleetClient::connect(listener.local_addr().unwrap().to_string());
        let (mut peer, _) = listener.accept().unwrap();
        let packet = serde_json::to_vec(&HubMessage::FleetSnapshot {
            protocol_version: PROTOCOL_VERSION,
            generated_at_unix_ms: 0,
            machines: vec![],
        })
        .unwrap();
        peer.write_all(&packet[..10]).unwrap();
        thread::sleep(POLL * 2); // a read timeout in the middle of a JSON packet
        peer.write_all(&packet[10..]).unwrap();
        peer.write_all(b"\n").unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        while !matches!(client.snapshot().connection, Connection::Connected)
            && Instant::now() < deadline
        {
            thread::sleep(Duration::from_millis(10));
        }
        assert!(!client.snapshot().stale);
        drop(peer);
        while !matches!(client.snapshot().connection, Connection::Disconnected)
            && Instant::now() < deadline
        {
            thread::sleep(Duration::from_millis(10));
        }
        assert!(client.snapshot().stale);
        assert!(client.snapshot().error.is_some());
        let (mut peer, _) = listener.accept().unwrap();
        peer.write_all(&packet).unwrap();
        peer.write_all(b"\n").unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        while client.snapshot().stale && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(10));
        }
        assert!(
            !client.snapshot().stale,
            "client should reconnect after EOF"
        );
        assert!(client.snapshot().error.is_none());
    }
}
