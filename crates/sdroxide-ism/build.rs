//! Builds the vendored rtl_433 and the shim that drives it.
//!
//! Does nothing at all unless the `rtl433` feature is on, so the crate's native
//! decoders still build with no C toolchain and no submodule checked out.
//!
//! `vendor/rtl_433` is a git submodule, used unmodified. Upstream's own
//! `src/CMakeLists.txt` already defines a static `r_433` target holding
//! everything except `main()`, which is exactly the library an embedder wants —
//! so this builds that target rather than curating a source list that would
//! drift every time upstream adds a decoder.

use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    if std::env::var_os("CARGO_FEATURE_RTL433").is_none() {
        return;
    }

    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let vendor = manifest.join("../../vendor/rtl_433");

    if !vendor.join("include/r_flow.h").exists() {
        panic!(
            "vendored rtl_433 sources are missing at {}\n\
             run: git submodule update --init --recursive\n\
             (or build with --no-default-features to leave the rtl_433 decoders out)",
            vendor.display()
        );
    }
    let vendor = vendor.canonicalize().expect("canonicalize vendor/rtl_433");

    // `cmake::Config` builds in `<out_dir>/build`; named here rather than left
    // to default to `<OUT_DIR>/build` so the in-tree copy below cannot land on
    // a build tree an earlier version of this script left there.
    let cmake_out = out.join("rtl_433");

    // The windows-gnu build runs through MSYS make, whose makefile parser has no
    // notion of drive letters. For a source outside the build tree CMake emits
    // the rule line
    //     src/CMakeFiles/r_433.dir/abuf.c.o: D:/.../src/abuf.c
    // whose second colon makes it a malformed static pattern rule — make stops
    // with "target pattern contains no '%'" before compiling anything. Building
    // from a copy of the tree in place keeps every path in the generated
    // makefiles relative, because the source directory then *is* the binary
    // directory. sdroxide-rade carries the same workaround, for the same reason.
    //
    // This also copies the tree to prevent inadvertant rebuilds, as the CMake
    // project generates a header file as part of the build process. If the tree
    // is not copied, this means that every build sees this directory as stale,
    // triggering a rebuild.
    let rtl433 = {
        let in_tree = cmake_out.join("build");
        // The crate wipes a build directory whose cache names a source
        // directory other than the one it is handed — which here is the copy
        // itself, so a cache left by an out-of-source build would take the
        // copied sources with it. Clear it before copying, not after.
        if stale_cache(&in_tree) {
            fs::remove_dir_all(&in_tree)
                .unwrap_or_else(|e| panic!("clear {}: {e}", in_tree.display()));
        }
        copy_tree(&vendor, &in_tree);
        in_tree
    };

    let dst = cmake::Config::new(&rtl433)
        .out_dir(&cmake_out)
        // We hand rtl_433 samples ourselves, so none of its own inputs are
        // wanted. ENABLE_RTLSDR in particular defaults to ON *and is fatal* when
        // librtlsdr is absent, which would make the whole build depend on a
        // library sdroxide deliberately does not use.
        .define("ENABLE_RTLSDR", "OFF")
        .define("ENABLE_SOAPYSDR", "OFF")
        .define("ENABLE_OPENSSL", "OFF")
        .define("BUILD_TESTING", "OFF")
        // Upstream still declares `cmake_minimum_required(VERSION 2.6...3.10)`,
        // which CMake 4 refuses outright.
        .define("CMAKE_POLICY_VERSION_MINIMUM", "3.5")
        // Upstream bakes `git describe` into `version_string()`, which the shim
        // hands to the UI, and looks for the repository by walking up from the
        // source directory until it finds a `.git` — the submodule's own, or,
        // for the Windows copy under target/, sdroxide's, whose branch name it
        // then reports. Either way the same commit names a different version
        // from a full clone, a shallow CI checkout or a source tarball, and no
        // two of those binaries can match. A git directory with no HEAD in it
        // turns the lookup off, and the version then comes from CHANGELOG.md,
        // which is part of the source.
        .define("FORCED_GIT_DIR", cmake_out.join("no-git"))
        // The decoders run on every IQ block; an unoptimised build of them is
        // not worth having even under `cargo build`.
        .profile("Release")
        .build_target("r_433")
        .build();

    let build = dst.join("build");
    println!("cargo:rustc-link-search=native={}", build.join("src").display());
    println!("cargo:rustc-link-lib=static=r_433");

    if cfg!(target_os = "windows") {
        // mongoose, which r_api.c's network outputs pull in.
        println!("cargo:rustc-link-lib=dylib=ws2_32");
    } else {
        println!("cargo:rustc-link-lib=dylib=m");
        println!("cargo:rustc-link-lib=dylib=pthread");
    }

    let includes = [rtl433.join("include"), build.join("include")];

    println!("cargo:rerun-if-changed=src/rtl433/shim.c");
    println!("cargo:rerun-if-changed=src/rtl433/shim.h");
    println!("cargo:rerun-if-changed={}", vendor.join("src").display());
    println!("cargo:rerun-if-changed={}", vendor.join("include").display());

    let mut shim = cc::Build::new();
    shim.file(manifest.join("src/rtl433/shim.c")).opt_level(2).std("c99");
    for inc in &includes {
        shim.include(inc);
    }
    shim.compile("sdroxide_rtl433_shim");

    // Only our own header: rtl_433's would drag mongoose and the pulse detector
    // into Rust, none of which the bridge names.
    let bindings = bindgen::Builder::default()
        .header(manifest.join("src/rtl433/shim.h").to_string_lossy())
        .allowlist_function("sdrx_rtl433_.*")
        .allowlist_type("sdrx_rtl433_.*")
        .allowlist_var("SDRX_RTL433_.*")
        .layout_tests(false)
        .generate()
        .expect("bindgen over shim.h");

    bindings.write_to_file(out.join("bindings.rs")).expect("write bindings.rs");
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

/// Copy `src` into `dst` recursively, skipping the dot-entries — `.git` above
/// all, which for a submodule is a *pointer* to the gitdir and would dangle in
/// the copy. Files already up to date are left alone, so editing the shim does
/// not rebuild three hundred decoders.
fn copy_tree(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).unwrap_or_else(|e| panic!("create {}: {e}", dst.display()));
    let entries = fs::read_dir(src).unwrap_or_else(|e| panic!("read {}: {e}", src.display()));
    for entry in entries {
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
            (Ok(src_time), Ok(dst_time)) => src_time > dst_time,
            _ => true,
        };
        if stale {
            fs::copy(&from, &to).unwrap_or_else(|e| panic!("copy {}: {e}", from.display()));
        }
    }
}
