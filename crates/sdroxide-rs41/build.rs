//! When the `sonde` feature is on, compile rs1729 `*mod` helpers from
//! `vendor/rs1729/demod/mod` into `target/sonde-mod/<profile>/` and expose that
//! directory via `SDROXIDE_SONDE_MOD_BUILD`.
//!
//! Uses the `cc` crate (not Make) so Linux, macOS, and Windows/MinGW all build
//! the same way in CI and on developer machines.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let vendor = manifest_dir.join("../../vendor/rs1729/demod/mod");
    for f in [
        "rs41mod.c",
        "dfm09mod.c",
        "m10mod.c",
        "m10m20mod.c",
        "mts01mod.c",
        "demod_mod.c",
        "demod_mod.h",
        "bch_ecc_mod.c",
        "bch_ecc_mod.h",
    ] {
        println!("cargo:rerun-if-changed={}", vendor.join(f).display());
    }

    if env::var("CARGO_FEATURE_SONDE").is_err() {
        println!("cargo:rustc-env=SDROXIDE_SONDE_MOD_BUILD=");
        println!("cargo:warning=sdroxide-rs41: feature `sonde` off — not building *mod helpers");
        return;
    }

    if !vendor.join("rs41mod.c").is_file() {
        println!(
            "cargo:warning=sdroxide-rs41: vendor/rs1729 missing — run \
             `git submodule update --init --recursive` (feature `sonde` needs it)"
        );
        println!("cargo:rustc-env=SDROXIDE_SONDE_MOD_BUILD=");
        return;
    }

    let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".into());
    let stage = manifest_dir.join("../../target/sonde-mod").join(&profile);
    fs::create_dir_all(&stage).expect("create target/sonde-mod");

    let target = env::var("TARGET").unwrap_or_default();
    let windows = target.contains("windows");
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());

    // Shared objects used by several helpers.
    compile_obj(&vendor, &out, "demod_mod", &["demod_mod.c"], false);
    compile_obj(&vendor, &out, "bch_ecc_mod", &["bch_ecc_mod.c"], false);

    let demod = out.join(obj_name("demod_mod"));
    let bch = out.join(obj_name("bch_ecc_mod"));

    link_program(&vendor, &out, &stage, "rs41mod", &["rs41mod.c"], &[&demod, &bch], windows);
    link_program(&vendor, &out, &stage, "dfm09mod", &["dfm09mod.c"], &[&demod], windows);
    link_program(&vendor, &out, &stage, "m10mod", &["m10mod.c"], &[&demod], windows);
    link_program(&vendor, &out, &stage, "m10m20mod", &["m10m20mod.c"], &[&demod], windows);
    link_program(&vendor, &out, &stage, "mts01mod", &["mts01mod.c"], &[&demod], windows);

    // OpenWebRX name for M20; rs1729 ships the combined binary.
    let m20 = stage.join(exe_name("m20mod", windows));
    let m10m20 = stage.join(exe_name("m10m20mod", windows));
    if m10m20.is_file() {
        let _ = fs::copy(&m10m20, &m20);
        set_executable(&m20);
    }

    let abs = stage.canonicalize().unwrap_or(stage);
    println!("cargo:rustc-env=SDROXIDE_SONDE_MOD_BUILD={}", abs.display());
    println!("cargo:warning=sdroxide-rs41: *mod helpers staged in {}", abs.display());
}

fn compile_obj(vendor: &Path, out: &Path, name: &str, sources: &[&str], fast: bool) {
    let mut build = cc::Build::new();
    build.opt_level(3).warnings(false).include(vendor).out_dir(out).cargo_metadata(false);
    if fast {
        // demod_mod upstream uses -Ofast; stick to -O3 for portability.
    }
    for s in sources {
        build.file(vendor.join(s));
    }
    // Emit a single object we can pass to the linker, not a staticlib archive
    // with a mangled name — `cc` always builds a staticlib; we link via `ar` extract
    // is awkward. Instead compile with -c through the C compiler directly.
    let _ = fast;
    let compiler = build.get_compiler();
    let mut cmd = compiler.to_command();
    let obj = out.join(obj_name(name));
    cmd.arg("-c").arg("-O3");
    apply_windows_cflags(&mut cmd);
    if compiler.is_like_msvc() {
        cmd.arg(format!("-Fo{}", obj.display()));
    } else {
        cmd.arg("-o").arg(&obj);
    }
    for s in sources {
        cmd.arg(vendor.join(s));
    }
    cmd.arg("-I").arg(vendor);
    run(&mut cmd, &format!("compile {name}"));
}

fn link_program(
    vendor: &Path,
    out: &Path,
    stage: &Path,
    name: &str,
    sources: &[&str],
    objs: &[&Path],
    windows: bool,
) {
    // Compile the program's own .c to an object, then link with shared objs.
    let prog_obj = out.join(obj_name(name));
    {
        let build = cc::Build::new();
        let compiler = build.get_compiler();
        let mut cmd = compiler.to_command();
        cmd.arg("-c").arg("-O3");
        apply_windows_cflags(&mut cmd);
        if compiler.is_like_msvc() {
            cmd.arg(format!("-Fo{}", prog_obj.display()));
        } else {
            cmd.arg("-o").arg(&prog_obj);
        }
        for s in sources {
            cmd.arg(vendor.join(s));
        }
        cmd.arg("-I").arg(vendor);
        run(&mut cmd, &format!("compile {name}.c"));
    }

    let exe = stage.join(exe_name(name, windows));
    let build = cc::Build::new();
    let compiler = build.get_compiler();
    let mut cmd = compiler.to_command();
    if compiler.is_like_msvc() {
        cmd.arg(format!("-Fe{}", exe.display()));
        cmd.arg(&prog_obj);
        for o in objs {
            cmd.arg(o);
        }
    } else {
        cmd.arg("-O3").arg("-o").arg(&exe);
        cmd.arg(&prog_obj);
        for o in objs {
            cmd.arg(o);
        }
        cmd.arg("-lm");
    }
    run(&mut cmd, &format!("link {name}"));
    set_executable(&exe);
}

/// rs1729 only enables `_setmode(stdin, _O_BINARY)` under `#ifdef CYGWIN`.
/// Native MinGW/MSVC Windows builds need the same: without it, a `0x1A` byte
/// in the float IQ stream is treated as EOF and the helper exits 0 with a
/// broken stdin pipe (exactly the Windows SONDE failure mode).
fn apply_windows_cflags(cmd: &mut Command) {
    let target = env::var("TARGET").unwrap_or_default();
    if target.contains("windows") {
        cmd.arg("-DCYGWIN");
    }
}

fn run(cmd: &mut Command, what: &str) {
    let status = cmd.status().unwrap_or_else(|e| panic!("{what}: failed to spawn: {e}"));
    if !status.success() {
        panic!("{what}: {cmd:?} failed: {status}");
    }
}

fn obj_name(name: &str) -> String {
    let target = env::var("TARGET").unwrap_or_default();
    if target.contains("msvc") {
        format!("{name}.obj")
    } else {
        format!("{name}.o")
    }
}

fn exe_name(name: &str, windows: bool) -> String {
    if windows {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

fn set_executable(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = fs::metadata(path) {
            let mut perms = meta.permissions();
            perms.set_mode(0o755);
            let _ = fs::set_permissions(path, perms);
        }
    }
    let _ = path;
}
