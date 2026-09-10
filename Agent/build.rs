use std::{env, path::PathBuf, process::Command};

fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").expect("missing target OS");

    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").expect("missing target architecture");

    if target_arch != "x86_64" {
        panic!("WynObserve assembly currently supports x86_64 only");
    }

    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("missing manifest directory"));

    let asm_dir = manifest_dir.join("asm");

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("missing OUT_DIR"));

    let (cpuid_file, vein_tsc_file, object_format, cpuid_object_name, vein_tsc_object_name) =
        match target_os.as_str() {
            "linux" => (
                asm_dir.join("cpuid_sysv.asm"),
                asm_dir.join("instruction_vein/vein_tsc_sysv.asm"),
                "elf64",
                "cpuid.o",
                "vein_tsc.o",
            ),

            "windows" => (
                asm_dir.join("cpuid_win64.asm"),
                asm_dir.join("instruction_vein/vein_tsc_win64.asm"),
                "win64",
                "cpuid.obj",
                "vein_tsc.obj",
            ),

            other => {
                panic!("unsupported target OS: {other}");
            }
        };

    let cpuid_object_path = out_dir.join(cpuid_object_name);

    let vein_tsc_object_path = out_dir.join(vein_tsc_object_name);

    /*
     * Assemble  NASM source files.
     */
    assemble(&cpuid_file, &cpuid_object_path, object_format);

    assemble(&vein_tsc_file, &vein_tsc_object_path, object_format);

    /*
     * Bundle  object files into the native
     * library Cargo will link into wyn-agent.
     */
    cc::Build::new()
        .object(&cpuid_object_path)
        .object(&vein_tsc_object_path)
        .compile("wynasm");

    /*
     * Rebuild if any assembly source changes.
     */
    println!(
        "cargo:rerun-if-changed={}",
        asm_dir.join("cpuid_sysv.asm").display()
    );

    println!(
        "cargo:rerun-if-changed={}",
        asm_dir.join("cpuid_win64.asm").display()
    );

    println!(
        "cargo:rerun-if-changed={}",
        asm_dir.join("instruction_vein/vein_tsc_sysv.asm").display()
    );

    println!(
        "cargo:rerun-if-changed={}",
        asm_dir
            .join("instruction_vein/vein_tsc_win64.asm")
            .display()
    );
}

fn assemble(source: &std::path::Path, output: &std::path::Path, format: &str) {
    let status = Command::new("nasm")
        .arg("-f")
        .arg(format)
        .arg(source)
        .arg("-o")
        .arg(output)
        .status()
        .expect("failed to run NASM");

    if !status.success() {
        panic!("NASM failed while assembling {}", source.display());
    }
}
