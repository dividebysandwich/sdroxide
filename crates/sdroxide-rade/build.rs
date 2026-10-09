//! Builds the vendored RADE C library and the FARGAN/LPCNet shim, then
//! generates raw bindings for both.
//!
//! `vendor/rade_c` is a git submodule, used unmodified. Its CMake downloads and
//! autotools-builds the patched Opus that carries the vocoder
//! (`--enable-osce --enable-dred`, plus rade_c's two `opus-nnet.*.diff`
//! patches), so the first build needs network access and
//! autoconf/automake/libtool; later builds reuse the `OUT_DIR` tree.
//!
//! Upstream builds `librade` as a shared object. We want it static — a shared
//! library would have to be shipped and found next to the executable on three
//! platforms, each with its own rules. So a small generated wrapper project
//! adds a second target over *the same source list*, read back off upstream's
//! own target rather than copied, so it cannot drift when the submodule moves.

use std::path::PathBuf;

/// On Windows `canonicalize` returns a verbatim `\\?\D:\...` path. libclang
/// cannot resolve a header's relative `#include`s against one (rade_api.h's
/// `rade_tx.h` goes missing), so a plain drive path is handed on instead.
/// UNC shares (`\\?\UNC\...`) are left as they are.
fn strip_verbatim(p: PathBuf) -> PathBuf {
    let plain = p
        .to_str()
        .and_then(|s| s.strip_prefix(r"\\?\"))
        .filter(|rest| !rest.starts_with("UNC\\"))
        .map(PathBuf::from);
    plain.unwrap_or(p)
}

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());

    let rade_c = manifest.join("../../vendor/rade_c");
    if !rade_c.join("src/rade_api.h").exists() {
        panic!(
            "vendored RADE sources are missing at {}\n\
             run: git submodule update --init --recursive",
            rade_c.display()
        );
    }
    let rade_c = strip_verbatim(rade_c.canonicalize().expect("canonicalize vendor/rade_c"));

    let wrapper = write_wrapper_project(&out, &rade_c);

    // Always Release: the neural encoder/decoder is unusably slow unoptimized,
    // including under a plain `cargo build`.
    let dst = cmake::Config::new(&wrapper)
        .define("RADE_C_DIR", rade_c.to_string_lossy().as_ref())
        .profile("Release")
        .build_target("rade_static")
        .build();

    let build = dst.join("build");
    // ExternalProject's default layout for rade_c's `build_opus` target, which
    // is configured BUILD_IN_SOURCE so its source and binary dirs coincide.
    let opus_src = build.join("build_opus-prefix/src/build_opus");
    let opus_lib_dir = opus_src.join(".libs");
    assert!(
        opus_lib_dir.is_dir(),
        "expected the Opus build at {} — did rade_c's ExternalProject change layout?",
        opus_lib_dir.display()
    );

    let includes: Vec<PathBuf> = vec![
        rade_c.join("src"),
        opus_src.clone(),
        opus_src.join("dnn"),
        opus_src.join("celt"),
        opus_src.join("include"),
    ];

    // The vocoder half: RADE's own API stops at feature vectors, so speech <->
    // features goes through a small C shim over Opus's FARGAN/LPCNet.
    let mut shim = cc::Build::new();
    shim.file(manifest.join("src/shim.c")).define("HAVE_CONFIG_H", None).opt_level(2);
    for inc in &includes {
        shim.include(inc);
    }
    shim.compile("sdroxide_rade_shim");

    println!("cargo:rustc-link-search=native={}", build.display());
    println!("cargo:rustc-link-lib=static=rade_static");
    println!("cargo:rustc-link-search=native={}", opus_lib_dir.display());
    println!("cargo:rustc-link-lib=static=opus");
    println!("cargo:rustc-link-lib=dylib=m");

    let mut builder = bindgen::Builder::default()
        .header(rade_c.join("src/rade_api.h").to_string_lossy())
        .header(manifest.join("src/shim.h").to_string_lossy())
        .allowlist_function("rade_.*")
        .allowlist_function("sdrx_voc_.*")
        .allowlist_type("RADE_COMP")
        .allowlist_var("RADE_.*")
        // `struct rade` is fully defined in the header — it embeds the V1 and
        // V2 transmit and receive state inline — but we only ever hold a
        // pointer to one. Blocklisting it keeps Rust off that layout entirely;
        // see the opaque declaration in sys.rs.
        .blocklist_type("rade")
        .layout_tests(false)
        .derive_debug(false)
        .clang_arg("-DHAVE_CONFIG_H");
    for inc in &includes {
        builder = builder.clang_arg(format!("-I{}", inc.display()));
    }
    builder
        .generate()
        .expect("bindgen rade_api.h + shim.h")
        .write_to_file(out.join("bindings.rs"))
        .expect("write bindings.rs");

    println!("cargo:rerun-if-changed=src/shim.c");
    println!("cargo:rerun-if-changed=src/shim.h");
    println!("cargo:rerun-if-changed={}", rade_c.join("src").display());
    println!("cargo:rerun-if-changed={}", rade_c.join("cmake").display());
}

/// Generate the wrapper CMake project into `OUT_DIR` and return its path.
///
/// It mirrors rade_c's own top-level `CMakeLists.txt` (include `BuildOpus`,
/// then add `src`) and adds one static target built from upstream's own source
/// list. `BuildOpus.cmake` resolves its Opus patches relative to
/// `CMAKE_SOURCE_DIR`, which is now *this* project, so the two `.diff` files
/// are copied in alongside. On Windows the C sources are copied into the build
/// tree as well — see the comment in the generated file.
fn write_wrapper_project(out: &std::path::Path, rade_c: &std::path::Path) -> PathBuf {
    let wrapper = out.join("wrapper");
    std::fs::create_dir_all(wrapper.join("src")).expect("create wrapper dir");
    for diff in ["opus-nnet.h.diff", "opus-nnet.c.diff"] {
        std::fs::copy(rade_c.join("src").join(diff), wrapper.join("src").join(diff))
            .unwrap_or_else(|e| panic!("copy {diff}: {e}"));
    }
    std::fs::write(
        wrapper.join("CMakeLists.txt"),
        r#"# Generated by sdroxide-rade/build.rs — do not edit.
cmake_minimum_required(VERSION 3.16)
project(sdroxide_rade C)
set(CMAKE_C_STANDARD 11)
set(CMAKE_POSITION_INDEPENDENT_CODE ON)

# Fetches and patches the FARGAN/LPCNet-enabled Opus, and defines the imported
# `opus` target plus its include directories for everything below.
include(${RADE_C_DIR}/cmake/BuildOpus.cmake)

# Upstream's own targets, unmodified. We never build its shared `rade`; we only
# borrow the target's source list.
add_subdirectory(${RADE_C_DIR}/src rade_c_src)

get_target_property(RADE_SOURCES rade SOURCES)
get_target_property(RADE_SRC_DIR rade SOURCE_DIR)

if(WIN32)
    # The windows-gnu build runs through MSYS make, whose makefile parser has
    # no notion of drive letters. For a source outside the build tree CMake
    # emits the rule line
    #     CMakeFiles/rade_static.dir/....obj: D:/.../rade_api.c
    # whose second colon makes it a malformed static pattern rule — make stops
    # with "target pattern contains no '%'". Compiling from copies inside the
    # binary directory keeps every path in the generated makefiles relative.
    set(RADE_COPY_DIR ${CMAKE_CURRENT_BINARY_DIR}/rade_src)
    set(RADE_ABS_SOURCES ${RADE_SOURCES})
    list(TRANSFORM RADE_ABS_SOURCES PREPEND "${RADE_SRC_DIR}/")
    file(GLOB RADE_HEADERS "${RADE_SRC_DIR}/*.h")
    file(COPY ${RADE_ABS_SOURCES} ${RADE_HEADERS} DESTINATION ${RADE_COPY_DIR})
    list(TRANSFORM RADE_SOURCES PREPEND "${RADE_COPY_DIR}/")
else()
    list(TRANSFORM RADE_SOURCES PREPEND "${RADE_SRC_DIR}/")
endif()

add_library(rade_static STATIC ${RADE_SOURCES})
target_include_directories(rade_static PRIVATE ${RADE_SRC_DIR})
target_compile_definitions(rade_static PRIVATE IS_BUILDING_RADE_API=1)
target_link_libraries(rade_static opus m)
"#,
    )
    .expect("write wrapper CMakeLists.txt");
    wrapper
}
