use std::env;

fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rustc-check-cfg=cfg(fixed_turbo)");
    println!("cargo::rustc-check-cfg=cfg(arm_simd32)");
    let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target = env::var("TARGET").unwrap_or_default();
    let cpu = target.split('-').next().unwrap_or_default();
    let simd32 = arch == "arm"
        && ((cpu == "arm" && os == "linux")
            || cpu.starts_with("armv6")
            || cpu.starts_with("armv7")
            || cpu == "thumbv7neon");
    if simd32 {
        println!("cargo::rustc-cfg=arm_simd32");
    }
    let fixed = env::var_os("CARGO_FEATURE_FIXED_TURBO").is_some();
    let float = env::var_os("CARGO_FEATURE_FLOAT_TURBO").is_some();
    let target_default = os == "linux" && (arch == "aarch64" || arch == "arm");
    if fixed && float {
        let kind = if target_default { "fixed-point" } else { "floating-point" };
        println!("cargo::warning=fixed-turbo and float-turbo are both enabled; using the target's {kind} default");
    }
    let use_fixed = if fixed == float { target_default } else { fixed };
    if use_fixed {
        println!("cargo::rustc-cfg=fixed_turbo");
    }
}
