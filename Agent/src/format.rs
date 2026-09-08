pub fn format_bytes(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * 1024.0;
    const GIB: f64 = MIB * 1024.0;
    const TIB: f64 = GIB * 1024.0;

    let value = bytes as f64;

    if value >= TIB {
        format!("{:.2} TiB", value / TIB)
    } else if value >= GIB {
        format!("{:.2} GiB", value / GIB)
    } else if value >= MIB {
        format!("{:.2} MiB", value / MIB)
    } else if value >= KIB {
        format!("{:.2} KiB", value / KIB)
    } else {
        format!("{bytes} B")
    }
}

#[cfg(test)]
mod tests {
    use super::format_bytes;

    #[test]
    fn formats_bytes_without_scaling() {
        assert_eq!(format_bytes(500), "500 B");
    }

    #[test]
    fn formats_kibibytes() {
        assert_eq!(format_bytes(2 * 1024), "2.00 KiB");
    }

    #[test]
    fn formats_mebibytes() {
        assert_eq!(format_bytes(5 * 1024 * 1024), "5.00 MiB");
    }

    #[test]
    fn formats_gibibytes() {
        assert_eq!(format_bytes(32 * 1024 * 1024 * 1024), "32.00 GiB");
    }
}
