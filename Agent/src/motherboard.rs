//! Read-only firmware inventory. Shared parser follows DMTF SMBIOS 3.9.0,
//! DSP0134, Types 1/2/3/16/17. We deliberately do not retain serials or UUIDs.
use wyn_protocol::{MemoryDeviceSnapshot, MotherboardSnapshot};

const MAX_TABLE: usize = 4 * 1024 * 1024;

fn identity(value: &str) -> Option<String> {
    let value = value.trim_matches(|c: char| c.is_whitespace() || c == '\0');
    let lower = value.to_ascii_lowercase();
    if value.is_empty()
        || matches!(
            lower.as_str(),
            "unknown"
                | "none"
                | "not specified"
                | "not applicable"
                | "n/a"
                | "default string"
                | "system product name"
                | "system manufacturer"
                | "base board product name"
                | "base board manufacturer"
                | "base board version"
                | "system version"
                | "to be filled by o.e.m."
                | "to be filled by oem"
                | "x.x"
        )
        || value.chars().all(|c| c.is_ascii_punctuation() || c == ' ')
    {
        None
    } else {
        Some(
            value
                .chars()
                .filter(|c| !c.is_control())
                .take(160)
                .collect(),
        )
    }
}

struct Record<'a> {
    data: &'a [u8],
    strings: &'a [u8],
}
impl Record<'_> {
    fn byte(&self, offset: usize) -> Option<u8> {
        self.data.get(offset).copied()
    }
    fn word(&self, offset: usize) -> Option<u16> {
        Some(u16::from_le_bytes(
            self.data.get(offset..offset + 2)?.try_into().ok()?,
        ))
    }
    fn string(&self, offset: usize) -> Option<String> {
        let index = self.byte(offset)?.checked_sub(1)? as usize;
        identity(&String::from_utf8_lossy(
            self.strings.split(|b| *b == 0).nth(index)?,
        ))
    }
}

fn records(table: &[u8]) -> Option<Vec<Record<'_>>> {
    if table.len() > MAX_TABLE {
        return None;
    }
    let mut rest = table;
    let mut result = Vec::new();
    while !rest.is_empty() {
        let length = *rest.get(1)? as usize;
        if length < 4 {
            return None;
        }
        let data = rest.get(..length)?;
        let strings = rest.get(length..)?;
        let end = strings.windows(2).position(|w| w == [0, 0])?;
        result.push(Record {
            data,
            strings: &strings[..end],
        });
        if data[0] == 127 {
            break;
        }
        if result.len() > 4096 {
            return None;
        }
        rest = &strings[end + 2..];
    }
    Some(result)
}

fn memory_size(record: &Record<'_>) -> Option<u64> {
    let size = record.word(0x0c)?;
    match size {
        0xffff => None,
        0x7fff => {
            let extended =
                u32::from_le_bytes(record.data.get(0x1c..0x20)?.try_into().ok()?) & 0x7fff_ffff;
            (extended > 0).then_some(u64::from(extended) * 1024 * 1024)
        }
        _ if size & 0x8000 != 0 => Some(u64::from(size & 0x7fff) * 1024),
        _ => Some(u64::from(size) * 1024 * 1024),
    }
}

fn parse(table: &[u8]) -> Option<MotherboardSnapshot> {
    let records = records(table)?;
    let mut board = MotherboardSnapshot::default();
    // A Type 2 can describe a daughterboard. Prefer the explicitly identified motherboard.
    let baseboards: Vec<_> = records.iter().filter(|r| r.byte(0) == Some(2)).collect();
    if let Some(r) = baseboards
        .iter()
        .find(|r| r.byte(0x0d) == Some(10))
        .or_else(|| {
            baseboards
                .iter()
                .find(|r| r.byte(9).is_some_and(|b| b & 1 != 0))
        })
        .or_else(|| {
            (baseboards.len() == 1 && baseboards[0].byte(0x0d).is_none_or(|v| v <= 2))
                .then(|| &baseboards[0])
        })
    {
        board.manufacturer = r.string(4);
        board.model = r.string(5);
        board.version = r.string(6);
    }
    if let Some(r) = records.iter().find(|r| r.byte(0) == Some(1)) {
        board.system_manufacturer = r.string(4);
        board.system_model = r.string(5);
    }
    if let Some(r) = records.iter().find(|r| r.byte(0) == Some(3)) {
        board.chassis_type = r.byte(5).map(|v| v & 0x7f).filter(|v| *v > 2);
    }
    // Type 17 is also used for video/cache memory: only use Type 16 system-memory arrays.
    let arrays: Vec<_> = records
        .iter()
        .filter(|r| r.byte(0) == Some(16) && r.byte(5) == Some(3))
        .collect();
    board.memory_slot_count = arrays
        .iter()
        .try_fold(0u16, |sum, r| sum.checked_add(r.word(0x0d)?))
        .filter(|n| *n > 0);
    for r in records.iter().filter(|r| r.byte(0) == Some(17)) {
        if !arrays
            .iter()
            .any(|a| r.word(4).is_some() && a.word(2) == r.word(4))
        {
            continue;
        }
        board.memory_devices.push(MemoryDeviceSnapshot {
            locator: r.string(0x10),
            bank: r.string(0x11),
            size_bytes: memory_size(r),
            form_factor: r.byte(0x0e).filter(|v| *v > 2),
            memory_type: r.byte(0x12).filter(|v| *v > 2),
        });
    }
    Some(board)
}

#[cfg(any(windows, test))]
fn parse_windows_buffer(raw: &[u8]) -> Option<MotherboardSnapshot> {
    // RawSMBIOSData has four version bytes, then a little-endian DWORD length.
    let length = u32::from_le_bytes(raw.get(4..8)?.try_into().ok()?) as usize;
    if length > MAX_TABLE {
        return None;
    }
    parse(raw.get(8..8 + length)?)
}

#[cfg(windows)]
fn platform() -> MotherboardSnapshot {
    use windows_sys::Win32::System::SystemInformation::{GetSystemFirmwareTable, RSMB};
    // SAFETY: size query has a null buffer; the second call receives a live byte buffer
    // of the declared size. No firmware writes or privileged driver access are used.
    let size = unsafe { GetSystemFirmwareTable(RSMB, 0, std::ptr::null_mut(), 0) };
    if !(8..=MAX_TABLE as u32 + 8).contains(&size) {
        return MotherboardSnapshot::default();
    }
    let mut raw = vec![0; size as usize];
    let written = unsafe { GetSystemFirmwareTable(RSMB, 0, raw.as_mut_ptr(), size) };
    if written < 8 || written > size {
        return MotherboardSnapshot::default();
    }
    let mut board = parse_windows_buffer(&raw[..written as usize]).unwrap_or_default();
    board.source = "Windows SMBIOS".into();
    board
}

#[cfg(target_os = "linux")]
fn platform() -> MotherboardSnapshot {
    use std::io::Read;
    let raw = std::fs::File::open("/sys/firmware/dmi/tables/DMI")
        .ok()
        .and_then(|file| {
            let mut bytes = Vec::new();
            file.take(MAX_TABLE as u64 + 1)
                .read_to_end(&mut bytes)
                .ok()?;
            Some(bytes)
        });
    let mut board = raw.as_deref().and_then(parse).unwrap_or_default();
    let read = |name: &str| {
        std::fs::read_to_string(format!("/sys/class/dmi/id/{name}"))
            .ok()
            .and_then(|s| identity(&s))
    };
    board.manufacturer = board.manufacturer.or_else(|| read("board_vendor"));
    board.model = board.model.or_else(|| read("board_name"));
    board.version = board.version.or_else(|| read("board_version"));
    board.system_manufacturer = board.system_manufacturer.or_else(|| read("sys_vendor"));
    board.system_model = board.system_model.or_else(|| read("product_name"));
    board.chassis_type = board.chassis_type.or_else(|| {
        read("chassis_type")?
            .parse::<u8>()
            .ok()
            .map(|v| v & 0x7f)
            .filter(|v| *v > 2)
    });
    board.source = "Linux DMI".into();
    board
}

#[cfg(not(any(windows, target_os = "linux")))]
fn platform() -> MotherboardSnapshot {
    MotherboardSnapshot::default()
}

pub fn collect() -> Option<MotherboardSnapshot> {
    let board = platform();
    (board.model.is_some()
        || board.manufacturer.is_some()
        || board.system_model.is_some()
        || board.chassis_type.is_some()
        || !board.memory_devices.is_empty())
    .then_some(board)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn record(
        kind: u8,
        length: usize,
        handle: u16,
        fields: &[(usize, u8)],
        strings: &[&str],
    ) -> Vec<u8> {
        let mut r = vec![0; length];
        r[0] = kind;
        r[1] = length as u8;
        r[2..4].copy_from_slice(&handle.to_le_bytes());
        for &(offset, value) in fields {
            r[offset] = value;
        }
        for s in strings {
            r.extend(s.as_bytes());
            r.push(0);
        }
        if strings.is_empty() {
            r.push(0);
        }
        r.push(0);
        r
    }
    fn fixture() -> Vec<u8> {
        [
            record(
                2,
                15,
                1,
                &[(4, 1), (5, 2), (6, 3), (7, 4), (13, 9)],
                &["Wrong", "Daughterboard", "1", "PRIVATE"],
            ),
            record(
                2,
                15,
                2,
                &[(4, 1), (5, 2), (6, 3), (7, 4), (13, 10)],
                &[
                    "Gigabyte Technology Co., Ltd.",
                    "B860M DS3H WIFI6E",
                    "x.x",
                    "PRIVATE",
                ],
            ),
            record(3, 9, 3, &[(5, 0x83)], &[]),
            record(16, 15, 10, &[(5, 3), (13, 3)], &[]),
            record(16, 15, 11, &[(5, 4), (13, 1)], &[]),
            record(
                17,
                32,
                12,
                &[
                    (4, 10),
                    (12, 0xff),
                    (13, 0x7f),
                    (14, 9),
                    (16, 1),
                    (18, 0x22),
                    (30, 1),
                ],
                &["DIMM_A2"],
            ),
            record(17, 21, 13, &[(4, 10), (14, 9), (16, 1)], &["DIMM_A1"]),
            record(
                17,
                21,
                14,
                &[(4, 10), (12, 0xff), (13, 0xff), (16, 1)],
                &["DIMM_B1"],
            ),
            record(
                17,
                21,
                15,
                &[(4, 11), (12, 0), (13, 0x10), (16, 1)],
                &["VRAM"],
            ),
            record(127, 4, 16, &[], &[]),
        ]
        .concat()
    }
    #[test]
    fn selects_motherboard_and_system_memory_without_private_identifiers() {
        let board = parse(&fixture()).unwrap();
        assert_eq!(board.model.as_deref(), Some("B860M DS3H WIFI6E"));
        assert_eq!(board.version, None);
        assert_eq!(board.chassis_type, Some(3));
        assert_eq!(board.memory_slot_count, Some(3));
        assert_eq!(board.memory_devices.len(), 3);
        assert_eq!(
            board.memory_devices[0].size_bytes,
            Some(64 * 1024 * 1024 * 1024)
        );
        assert_eq!(board.memory_devices[1].size_bytes, Some(0));
        assert_eq!(board.memory_devices[2].size_bytes, None);
        assert!(!serde_json::to_string(&board).unwrap().contains("PRIVATE"));
    }
    #[test]
    fn malformed_records_and_windows_lengths_are_bounded() {
        for data in [
            vec![2, 0, 0, 0],
            vec![2, 255, 0, 0, 0, 0],
            vec![2, 4, 0, 0, 65],
            vec![2, 4, 0, 0, 0],
        ] {
            assert!(parse(&data).is_none());
        }
        let table = fixture();
        let mut raw = vec![0, 3, 9, 0];
        raw.extend((table.len() as u32).to_le_bytes());
        raw.extend(&table);
        assert!(parse_windows_buffer(&raw).unwrap().model.is_some());
        raw.pop();
        assert!(parse_windows_buffer(&raw).is_none());
        raw[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(parse_windows_buffer(&raw).is_none());
        // Every truncated input must remain safe, including inside string sets.
        for end in 0..table.len() {
            let _ = parse(&table[..end]);
        }
    }
    #[test]
    fn missing_string_and_unknown_sizes_do_not_invent_inventory() {
        assert_eq!(identity(" To be filled by O.E.M. "), None);
        let r = record(2, 8, 0, &[(4, 8), (5, 1)], &["Real model"]);
        let board = parse(&r).unwrap();
        assert_eq!(board.manufacturer, None);
        assert_eq!(board.model.as_deref(), Some("Real model"));
        assert_eq!(board.memory_slot_count, None);
        assert!(board.memory_devices.is_empty());
        let r = record(17, 21, 0, &[(12, 0), (13, 0x81)], &[]);
        assert_eq!(memory_size(&records(&r).unwrap()[0]), Some(256 * 1024));
        let r = record(17, 21, 0, &[(12, 0xff), (13, 0x7f)], &[]);
        assert_eq!(memory_size(&records(&r).unwrap()[0]), None);
    }
}
