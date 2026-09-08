use super::NetworkSnapshot;

#[derive(Debug, Default)]
pub(super) struct PlatformNetworkCollector;

impl PlatformNetworkCollector {
    pub(super) fn new() -> Self {
        Self
    }

    pub(super) fn sample(&mut self) -> NetworkSnapshot {
        NetworkSnapshot {
            available: false,

            interface: "Windows network telemetry pending".into(),

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
}
