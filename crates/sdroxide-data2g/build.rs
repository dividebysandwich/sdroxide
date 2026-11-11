//! Builds Data2G's native core and the shim that drives it.
//!
//! `vendor/data2g` is a git submodule, used unmodified. Upstream's
//! `native/CMakeLists.txt` already defines `data2g_core` — the modem, the ARQ
//! engine and the host command language, with no Qt, no sound card and no
//! sockets — so this builds that one target with every app, binding and test
//! switched off, rather than curating a source list that would drift.

use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=shim/data2g_shim.cpp");
    println!("cargo:rerun-if-changed=shim/data2g_shim.h");

    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let vendor = manifest.join("../../vendor/data2g/native");
    if !vendor.join("core/arq/engine.hpp").exists() {
        panic!(
            "vendored Data2G sources are missing at {}\n\
             run: git submodule update --init --recursive",
            vendor.display()
        );
    }
    let vendor = vendor.canonicalize().expect("canonicalize vendor/data2g/native");
    println!("cargo:rerun-if-changed={}", vendor.join("core").display());

    let target = std::env::var("TARGET").unwrap();
    let msvc = target.contains("msvc");
    let windows = target.contains("windows");

    // The windows-gnu build runs through MSYS make, which cannot take a
    // drive-letter path in a rule; building from a copy in place keeps every
    // path relative (sdroxide-ism carries the same workaround).
    let cmake_out = out.join("data2g");
    let src = if windows && !msvc {
        let in_tree = cmake_out.join("build");
        if stale_cache(&in_tree) {
            fs::remove_dir_all(&in_tree)
                .unwrap_or_else(|e| panic!("clear {}: {e}", in_tree.display()));
        }
        copy_tree(&vendor, &in_tree);
        in_tree
    } else {
        vendor.clone()
    };

    let dst = cmake::Config::new(&src)
        .out_dir(&cmake_out)
        .define("DATA2G_BUILD_PYMODULE", "OFF")
        .define("DATA2G_BUILD_TESTS", "OFF")
        .define("DATA2G_BUILD_RIG", "OFF")
        .define("DATA2G_BUILD_QTAUDIO", "OFF")
        .define("DATA2G_BUILD_HOST", "OFF")
        .define("DATA2G_BUILD_GUI", "OFF")
        // A modem decode in an unoptimised build cannot keep up with the air.
        .profile("Release")
        .build_target("data2g_core")
        .build();

    let build = dst.join("build");
    // Single-config generators put the archive in the build root; MSVC's in
    // a per-config folder.
    for dir in [build.clone(), build.join("Release")] {
        println!("cargo:rustc-link-search=native={}", dir.display());
    }
    println!("cargo:rustc-link-lib=static=data2g_core");

    // zlib: the system's where CMake found one, else upstream's pinned copy.
    let fetched_zlib = build.join("Release").join("data2g_zlib.lib").exists()
        || build.join("libdata2g_zlib.a").exists()
        || build.join("data2g_zlib.lib").exists();
    if fetched_zlib {
        println!("cargo:rustc-link-lib=static=data2g_zlib");
    } else {
        println!("cargo:rustc-link-lib=dylib=z");
    }

    let mut shim = cc::Build::new();
    shim.cpp(true)
        .file(manifest.join("shim/data2g_shim.cpp"))
        .include(vendor.join("core"))
        .include(vendor.join("third_party/pocketfft"))
        .define("DATA2G_VERSION", Some(&*format!("\"{}\"", upstream_version(&vendor))))
        .opt_level(2);
    if msvc {
        shim.flag("/std:c++20").flag("/fp:precise").flag("/EHsc");
    } else {
        shim.flag("-std=c++20").flag("-fno-fast-math").flag("-ffp-contract=off");
    }
    // The shim (the last archive on the line) links the C++ runtime for the
    // core too.
    shim.cpp_link_stdlib(cpp_stdlib(&target));
    shim.compile("sdroxide_data2g_shim");

    if !windows {
        println!("cargo:rustc-link-lib=dylib=pthread");
    }
}

/// `project(data2g_native VERSION x.y.z ...)` in upstream's CMakeLists.
fn upstream_version(native: &Path) -> String {
    let cm = fs::read_to_string(native.join("CMakeLists.txt")).expect("read CMakeLists.txt");
    cm.lines()
        .find_map(|l| {
            let l = l.trim();
            let rest = l.strip_prefix("project(data2g_native VERSION ")?;
            rest.split_whitespace().next().map(str::to_string)
        })
        .unwrap_or_else(|| "unknown".into())
}

fn cpp_stdlib(target: &str) -> Option<&'static str> {
    if target.contains("msvc") {
        None
    } else if target.contains("apple") || target.contains("freebsd") {
        Some("c++")
    } else {
        Some("stdc++")
    }
}

/// Whether `build` holds a CMake cache from a build of some *other* source
/// directory, which `cmake::Config` answers by deleting the whole directory.
fn stale_cache(build: &Path) -> bool {
    let Ok(cache) = fs::read_to_string(build.join("CMakeCache.txt")) else {
        return false;
    };
    !cache.lines().any(|line| {
        line.strip_prefix("CMAKE_HOME_DIRECTORY:INTERNAL=")
            .is_some_and(|home| Path::new(home) == build)
    })
}

/// Copy `src` into `dst` recursively, skipping dot-entries; files already up
/// to date are left alone.
fn copy_tree(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).unwrap_or_else(|e| panic!("create {}: {e}", dst.display()));
    for entry in fs::read_dir(src).unwrap_or_else(|e| panic!("read {}: {e}", src.display())) {
        let entry = entry.expect("read dir entry");
        let name = entry.file_name();
        if name.to_string_lossy().starts_with('.') {
            continue;
        }
        let (from, to) = (entry.path(), dst.join(&name));
        if from.is_dir() {
            copy_tree(&from, &to);
            continue;
        }
        let stale = match (
            entry.metadata().and_then(|m| m.modified()),
            fs::metadata(&to).and_then(|m| m.modified()),
        ) {
            (Ok(s), Ok(d)) => s > d,
            _ => true,
        };
        if stale {
            fs::copy(&from, &to).unwrap_or_else(|e| panic!("copy {}: {e}", from.display()));
        }
    }
}
