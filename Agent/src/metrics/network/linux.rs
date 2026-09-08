use std::{fs, io, time::Instant};

use super::NetworkSnapshot;

const MIB_BYTES: f64 = 1024.0 * 1024.0;

#[derive(Debug, Clone, Copy)]
struct InterfaceCounters {
    rx_bytes: u64,
    rx_packets: u64,
    rx_errors: u64,
    rx_drops: u64,

    tx_bytes: u64,
    tx_packets: u64,
    tx_errors: u64,
    tx_drops: u64,
}

#[derive(Debug)]
pub(super) struct PlatformNetworkCollector {
    interface: Option<String>,

    previous: Option<InterfaceCounters>,
    previous_sample_at: Option<Instant>,
}

impl PlatformNetworkCollector {
    pub(super) fn new() -> Self {
        Self {
            interface: discover_default_interface(),
            previous: None,
            previous_sample_at: None,
        }
    }

    pub(super) fn sample(&mut self) -> NetworkSnapshot {
        let discovered = discover_default_interface();

        if discovered != self.interface && discovered.is_some() {
            self.interface = discovered;

            // Interface changed. Old counters are
            // meaningless against the new NIC.
            self.previous = None;
            self.previous_sample_at = None;
        }

        let Some(interface) = self.interface.clone() else {
            return unavailable_snapshot();
        };

        let counters = match read_interface_counters(&interface) {
            Ok(value) => value,

            Err(_) => {
                return unavailable_snapshot();
            }
        };

        let now = Instant::now();

        let mut rx_mib_s = 0.0;
        let mut tx_mib_s = 0.0;

        let mut rx_packets_per_second = 0;
        let mut tx_packets_per_second = 0;

        let mut rx_errors_per_second = 0;
        let mut tx_errors_per_second = 0;

        let mut rx_drops_per_second = 0;
        let mut tx_drops_per_second = 0;

        let mut rates_available = false;

        if let (Some(previous), Some(previous_at)) = (self.previous, self.previous_sample_at) {
            let elapsed = now.duration_since(previous_at);

            let seconds = elapsed.as_secs_f64();

            let deltas = (
                counters.rx_bytes.checked_sub(previous.rx_bytes),
                counters.tx_bytes.checked_sub(previous.tx_bytes),
                counters.rx_packets.checked_sub(previous.rx_packets),
                counters.tx_packets.checked_sub(previous.tx_packets),
                counters.rx_errors.checked_sub(previous.rx_errors),
                counters.tx_errors.checked_sub(previous.tx_errors),
                counters.rx_drops.checked_sub(previous.rx_drops),
                counters.tx_drops.checked_sub(previous.tx_drops),
            );

            if let (
                Some(rx_bytes),
                Some(tx_bytes),
                Some(rx_packets),
                Some(tx_packets),
                Some(rx_errors),
                Some(tx_errors),
                Some(rx_drops),
                Some(tx_drops),
            ) = deltas
            {
                if seconds > 0.0 {
                    rx_mib_s = (rx_bytes as f64 / MIB_BYTES / seconds) as f32;

                    tx_mib_s = (tx_bytes as f64 / MIB_BYTES / seconds) as f32;

                    rx_packets_per_second = (rx_packets as f64 / seconds) as u64;

                    tx_packets_per_second = (tx_packets as f64 / seconds) as u64;

                    rx_errors_per_second = (rx_errors as f64 / seconds) as u64;

                    tx_errors_per_second = (tx_errors as f64 / seconds) as u64;

                    rx_drops_per_second = (rx_drops as f64 / seconds) as u64;

                    tx_drops_per_second = (tx_drops as f64 / seconds) as u64;

                    rates_available = true;
                }
            }
        }

        self.previous = Some(counters);
        self.previous_sample_at = Some(now);

        let connections = count_established_tcp().ok();

        NetworkSnapshot {
            available: true,
            interface,

            rx_mib_s,
            tx_mib_s,

            rx_packets_per_second,
            tx_packets_per_second,

            rx_errors_per_second,
            tx_errors_per_second,

            rx_drops_per_second,
            tx_drops_per_second,

            connections: connections.unwrap_or(0),

            rates_available,

            connections_available: connections.is_some(),
        }
    }
}

fn read_interface_counters(interface: &str) -> io::Result<InterfaceCounters> {
    let contents = fs::read_to_string("/proc/net/dev")?;

    for line in contents.lines() {
        let Some((name, statistics)) = line.split_once(':') else {
            continue;
        };

        if name.trim() != interface {
            continue;
        }

        let values = statistics
            .split_whitespace()
            .map(|value| {
                value
                    .parse::<u64>()
                    .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
            })
            .collect::<Result<Vec<_>, _>>()?;

        if values.len() < 16 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid /proc/net/dev row",
            ));
        }

        return Ok(InterfaceCounters {
            rx_bytes: values[0],
            rx_packets: values[1],
            rx_errors: values[2],
            rx_drops: values[3],

            tx_bytes: values[8],
            tx_packets: values[9],
            tx_errors: values[10],
            tx_drops: values[11],
        });
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "network interface not found",
    ))
}

fn discover_default_interface() -> Option<String> {
    let contents = fs::read_to_string("/proc/net/route").ok()?;

    contents
        .lines()
        .skip(1)
        .filter_map(|line| {
            let fields = line.split_whitespace().collect::<Vec<_>>();

            if fields.len() < 8 {
                return None;
            }

            let interface = fields[0];
            let destination = fields[1];

            if destination != "00000000" {
                return None;
            }

            let flags = u32::from_str_radix(fields[3], 16).ok()?;

            // RTF_UP
            if flags & 0x1 == 0 {
                return None;
            }

            let metric = fields[6].parse::<u32>().unwrap_or(u32::MAX);

            Some((metric, interface.to_string()))
        })
        .min_by_key(|(metric, _)| *metric)
        .map(|(_, interface)| interface)
}

fn count_established_tcp() -> io::Result<usize> {
    let ipv4 = count_established_in("/proc/net/tcp")?;

    let ipv6 = count_established_in("/proc/net/tcp6").unwrap_or(0);

    Ok(ipv4 + ipv6)
}

fn count_established_in(path: &str) -> io::Result<usize> {
    let contents = fs::read_to_string(path)?;

    Ok(contents
        .lines()
        .skip(1)
        .filter(|line| line.split_whitespace().nth(3) == Some("01"))
        .count())
}

fn unavailable_snapshot() -> NetworkSnapshot {
    NetworkSnapshot {
        available: false,
        interface: "Network unavailable".into(),

        rx_mib_s: 0.0,
        tx_mib_s: 0.0,

        rx_packets_per_second: 0,
        tx_packets_per_second: 0,

        rx_errors_per_second: 0,
        tx_errors_per_second: 0,

        rx_drops_per_second: 0,
        tx_drops_per_second: 0,

        connections: 0,

        rates_available: false,
        connections_available: false,
    }
}
