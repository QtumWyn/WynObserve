use std::{
    io::{self, Write},
    net::{TcpListener, TcpStream},
    thread,
    time::Duration,
};

use crate::runtime::TelemetryRuntime;

const POLL_INTERVAL: Duration = Duration::from_millis(25);

pub fn run_server(runtime: TelemetryRuntime, listen_address: &str) -> io::Result<()> {
    let listener = TcpListener::bind(listen_address)?;

    println!("Telemetry server listening on {listen_address}");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("Observatory client connected");

                if let Err(error) = stream_snapshots(stream, &runtime) {
                    eprintln!("Observatory client disconnected: {error}");
                }
            }

            Err(error) => {
                eprintln!("Failed to accept Observatory connection: {error}");
            }
        }
    }

    Ok(())
}

fn stream_snapshots(mut stream: TcpStream, runtime: &TelemetryRuntime) -> io::Result<()> {
    let mut last_sequence: Option<u64> = None;

    loop {
        let Some(published) = runtime.latest() else {
            thread::sleep(POLL_INTERVAL);

            continue;
        };

        if last_sequence == Some(published.sequence) {
            thread::sleep(POLL_INTERVAL);

            continue;
        }

        let json = serde_json::to_string(&published.snapshot).map_err(io::Error::other)?;

        writeln!(stream, "{json}")?;

        stream.flush()?;

        last_sequence = Some(published.sequence);
    }
}
