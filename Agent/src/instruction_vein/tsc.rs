unsafe extern "C" {
    fn wyn_rdtscp() -> u64;
}

pub fn read() -> u64 {
    unsafe { wyn_rdtscp() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tsc_advances() {
        if !crate::cpu_identity::supports_rdtscp() {
            return;
        }
        let first = read();
        let second = read();

        assert!(second >= first);
    }
}
