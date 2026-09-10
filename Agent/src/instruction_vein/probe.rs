unsafe extern "C" {
    fn wyn_vein_probe() -> u64;
}

pub fn probe() -> u64 {
    unsafe { wyn_vein_probe() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nasm_probe_returns_expected_signature() {
        assert_eq!(probe(), 0x574E);
    }
}
