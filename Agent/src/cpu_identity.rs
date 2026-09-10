#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
struct CpuidRegisters {
    eax: u32,
    ebx: u32,
    ecx: u32,
    edx: u32,
}

#[derive(Debug, Clone)]
pub struct CpuIdentity {
    pub vendor: String,
    pub brand: String,
}

unsafe extern "C" {
    fn wyn_cpuid(leaf: u32, subleaf: u32, output: *mut CpuidRegisters);
}

fn cpuid(leaf: u32, subleaf: u32) -> CpuidRegisters {
    let mut registers = CpuidRegisters::default();

    unsafe {
        wyn_cpuid(leaf, subleaf, &mut registers);
    }

    registers
}

pub fn read_cpu_identity() -> CpuIdentity {
    let leaf_zero = cpuid(0, 0);
    let mut vendor_bytes = [0_u8; 12];

    vendor_bytes[0..4].copy_from_slice(&leaf_zero.ebx.to_le_bytes());
    vendor_bytes[4..8].copy_from_slice(&leaf_zero.edx.to_le_bytes());
    vendor_bytes[8..12].copy_from_slice(&leaf_zero.ecx.to_le_bytes());

    let vendor = String::from_utf8_lossy(&vendor_bytes)
        .trim_end_matches('\0')
        .to_string();

    let brand = read_brand_string();

    CpuIdentity { vendor, brand }
}

fn read_brand_string() -> String {
    let extended = cpuid(0x8000_0000, 0);

    if extended.eax < 0x8000_0004 {
        return "unknown CPU".to_string();
    }

    let mut bytes = Vec::with_capacity(48);

    for leaf in 0x8000_0002..=0x8000_0004 {
        let registers = cpuid(leaf, 0);

        bytes.extend_from_slice(&registers.eax.to_le_bytes());

        bytes.extend_from_slice(&registers.ebx.to_le_bytes());

        bytes.extend_from_slice(&registers.ecx.to_le_bytes());

        bytes.extend_from_slice(&registers.edx.to_le_bytes());
    }

    String::from_utf8_lossy(&bytes)
        .trim_matches('\0')
        .trim()
        .to_string()
}

pub fn supports_rdtscp() -> bool {
    let max_extended = cpuid(0x8000_0000, 0);

    if max_extended.eax < 0x8000_0001 {
        return false;
    }

    let features = cpuid(0x8000_0001, 0);

    features.edx & (1 << 27) != 0
}
