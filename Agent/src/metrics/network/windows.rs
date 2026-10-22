use super::NetworkSnapshot;
use crate::windows::text;
use std::{
    io,
    mem::{offset_of, size_of},
    ptr,
    time::Instant,
};
use windows_sys::Win32::{
    NetworkManagement::{IpHelper::*, Ndis::IfOperStatusUp},
    Networking::WinSock::{AF_INET, AF_INET6, SOCKADDR_IN},
};
#[derive(Debug)]
pub(super) struct PlatformNetworkCollector {
    previous: Option<(u32, [u64; 8], Instant)>,
}
impl PlatformNetworkCollector {
    pub(super) fn new() -> Self {
        Self { previous: None }
    }
    pub(super) fn sample(&mut self) -> NetworkSnapshot {
        let mut result = NetworkSnapshot {
            available: false,
            interface: String::new(),
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
        };
        // Route lookup only, no packet is sent. Match Linux's default-route interface.
        let mut destination: SOCKADDR_IN = unsafe { std::mem::zeroed() };
        destination.sin_family = AF_INET;
        destination.sin_addr.S_un.S_addr = u32::from_ne_bytes([1, 1, 1, 1]);
        let mut index = 0;
        if unsafe { GetBestInterfaceEx((&destination as *const SOCKADDR_IN).cast(), &mut index) }
            != 0
        {
            self.previous = None;
            return result;
        }
        let mut row = MIB_IF_ROW2 {
            InterfaceIndex: index,
            ..Default::default()
        };
        if unsafe { GetIfEntry2(&mut row) } != 0 || row.OperStatus != IfOperStatusUp {
            self.previous = None;
            return result;
        }
        result.available = true;
        result.interface = text(&row.Alias);
        let now = Instant::now();
        let counters = [
            row.InOctets,
            row.OutOctets,
            row.InUcastPkts + row.InNUcastPkts,
            row.OutUcastPkts + row.OutNUcastPkts,
            row.InErrors,
            row.OutErrors,
            row.InDiscards,
            row.OutDiscards,
        ];
        if let Some((previous_index, old, time)) = self.previous {
            let seconds = now.duration_since(time).as_secs_f64();
            if previous_index == index
                && seconds > 0.0
                && counters.iter().zip(old).all(|(new, old)| *new >= old)
            {
                let rate: Vec<_> = counters
                    .iter()
                    .zip(old)
                    .map(|(new, old)| (*new - old) as f64 / seconds)
                    .collect();
                result.rx_mib_s = (rate[0] / 1048576.0) as f32;
                result.tx_mib_s = (rate[1] / 1048576.0) as f32;
                result.rx_packets_per_second = rate[2].round() as u64;
                result.tx_packets_per_second = rate[3].round() as u64;
                result.rx_errors_per_second = rate[4].round() as u64;
                result.tx_errors_per_second = rate[5].round() as u64;
                result.rx_drops_per_second = rate[6].round() as u64;
                result.tx_drops_per_second = rate[7].round() as u64;
                result.rates_available = true;
            }
        }
        self.previous = Some((index, counters, now));
        if let Ok(ipv4) = established_connections(AF_INET as u32) {
            result.connections_available = true;
            result.connections = ipv4 + established_connections(AF_INET6 as u32).unwrap_or(0);
        }
        result
    }
}

// Match Linux's ESTABLISHED count; dwCurrEstab also includes CLOSE_WAIT, so the
// summary TCP statistics are not an equivalent substitute for these tables.
fn established_connections(family: u32) -> io::Result<usize> {
    let mut size = 0;
    let mut storage = Vec::<u64>::new(); // alignment for both native table layouts
    for _ in 0..3 {
        let buffer = if storage.is_empty() {
            ptr::null_mut()
        } else {
            storage.as_mut_ptr().cast()
        };
        let status = unsafe {
            GetExtendedTcpTable(buffer, &mut size, 0, family, TCP_TABLE_OWNER_PID_ALL, 0)
        };
        if status == 122 {
            // ERROR_INSUFFICIENT_BUFFER; the table may grow between queries
            if size > 16 * 1024 * 1024 {
                return Err(io::Error::other("TCP table too large"));
            }
            storage.resize((size as usize).div_ceil(8), 0);
            continue;
        }
        if status != 0 {
            return Err(io::Error::from_raw_os_error(status as i32));
        }
        if storage.is_empty() {
            return Ok(0);
        }
        let count = unsafe { *storage.as_ptr().cast::<u32>() } as usize;
        let (offset, stride) = if family == AF_INET as u32 {
            (
                offset_of!(MIB_TCPTABLE_OWNER_PID, table),
                size_of::<MIB_TCPROW_OWNER_PID>(),
            )
        } else {
            (
                offset_of!(MIB_TCP6TABLE_OWNER_PID, table),
                size_of::<MIB_TCP6ROW_OWNER_PID>(),
            )
        };
        if count > (storage.len() * 8).saturating_sub(offset) / stride {
            return Err(io::Error::other("Invalid TCP table size"));
        }
        let mut established = 0;
        for index in 0..count {
            let row = unsafe { storage.as_ptr().cast::<u8>().add(offset + index * stride) };
            let state = unsafe {
                if family == AF_INET as u32 {
                    (*row.cast::<MIB_TCPROW_OWNER_PID>()).dwState
                } else {
                    (*row.cast::<MIB_TCP6ROW_OWNER_PID>()).dwState
                }
            };
            if state == MIB_TCP_STATE_ESTAB as u32 {
                established += 1;
            }
        }
        return Ok(established);
    }
    Err(io::Error::other(
        "TCP table kept changing during collection",
    ))
}
