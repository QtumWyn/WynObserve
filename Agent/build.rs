use std::{env, path::PathBuf, process::Command};

fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").expect("missing target OS");

    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").expect("missing target architecture");

    if target_arch != "x86_64" {
        panic!("WynCommand CPUID currently supports x86_64 only");
    }

    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("missing manifest directory"));

    let asm_dir = manifest_dir.join("asm");

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("missing OUT_DIR"));

    let (asm_file, object_format, object_name) = match target_os.as_str() {
        "linux" => (asm_dir.join("cpuid_sysv.asm"), "elf64", "cpuid.o"),

        "windows" => (asm_dir.join("cpuid_win64.asm"), "win64", "cpuid.obj"),

        other => {
            panic!("unsupported target OS: {other}");
        }
    };

    let object_path = out_dir.join(object_name);

    let status = Command::new("nasm")
        .arg("-f")
        .arg(object_format)
        .arg(&asm_file)
        .arg("-o")
        .arg(&object_path)
        .status()
        .expect("failed to run NASM");

    if !status.success() {
        panic!("NASM failed while assembling {}", asm_file.display());
    }

    cc::Build::new().object(&object_path).compile("wynasm");

    println!(
        "cargo:rerun-if-changed={}",
        asm_dir.join("cpuid_sysv.asm").display()
    );

    println!(
        "cargo:rerun-if-changed={}",
        asm_dir.join("cpuid_win64.asm").display()
    );
}
