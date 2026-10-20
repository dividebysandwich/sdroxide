//! Builds the vendored Dream DRM receiver and the C++ shim between it and Rust,
//! then generates bindings for the shim's C API.
//!
//! Everything is compiled straight with `cc` rather than through Dream's qmake
//! project (or, in `crates/sdroxide-faad2`, faad2's CMake). Neither upstream
//! build system is usable here — Dream's needs Qt even for its console build,
//! and going through a generated makefile is what broke the Windows CI job for
//! the other vendored C in this tree (see `vendor/rade_c`). Source lists and
//! `cc::Build`s avoid both.
//!
//! Three dependencies Dream normally takes from the system are not taken here:
//!
//! * **fftw3** — `include/fftw3.h` plus `src/fftw_compat.c` provide the six
//!   entry points Dream's matlib uses.
//! * **libsndfile, speexdsp, pcap, hamlib, gps, Qt** — all optional in Dream,
//!   all left out, which is what the missing `HAVE_*` defines below select.
//! * **faad2 at runtime** — Dream dlopens `libfaad_drm.so.2`, which most
//!   systems do not have. It is linked in from `crates/sdroxide-faad2` instead,
//!   so DRM audio decodes out of the box, which is the whole point of the
//!   feature. That crate builds the one faad2 the binary has — stock, with
//!   `DRM_SUPPORT` — and Dream is compiled against the headers it exports.

use std::path::{Path, PathBuf};

/// Dream's own source list for a console-mode receiver, taken from the
/// `SOURCES` block of upstream's `dream.pro`, minus `resample/speexresampler.cpp`
/// (guarded everywhere by `HAVE_SPEEX`, which would pull in libspeexdsp).
const DREAM_SOURCES: &[&str] = &[
    "AMDemodulation.cpp",
    "AMSSDemodulation.cpp",
    "chanest/ChanEstTime.cpp",
    "chanest/ChannelEstimation.cpp",
    "chanest/IdealChannelEstimation.cpp",
    "chanest/TimeLinear.cpp",
    "chanest/TimeWiener.cpp",
    "creceivedata.cpp",
    "ctransmitdata.cpp",
    "datadecoding/DABMOT.cpp",
    "datadecoding/DataDecoder.cpp",
    "datadecoding/DataEncoder.cpp",
    "datadecoding/epgutil.cpp",
    "datadecoding/Experiment.cpp",
    "datadecoding/Journaline.cpp",
    "datadecoding/journaline/crc_8_16.c",
    "datadecoding/journaline/dabdgdec_impl.c",
    "datadecoding/journaline/log.c",
    "datadecoding/journaline/newsobject.cpp",
    "datadecoding/journaline/newssvcdec_impl.cpp",
    "datadecoding/journaline/NML.cpp",
    "datadecoding/journaline/Splitter.cpp",
    "datadecoding/MOTSlideShow.cpp",
    "DataIO.cpp",
    "drmchannel/ChannelSimulation.cpp",
    "DrmReceiver.cpp",
    "DrmSimulation.cpp",
    "DrmTransmitter.cpp",
    "FAC/FAC.cpp",
    "InputResample.cpp",
    "interleaver/BlockInterleaver.cpp",
    "interleaver/SymbolInterleaver.cpp",
    "IQInputFilter.cpp",
    "matlib/MatlibSigProToolbox.cpp",
    "matlib/MatlibStdToolbox.cpp",
    "MDI/AFPacketGenerator.cpp",
    "MDI/MDIDecode.cpp",
    "MDI/MDIInBuffer.cpp",
    "MDI/MDIRSCI.cpp",
    "MDI/MDITagItemDecoders.cpp",
    "MDI/MDITagItems.cpp",
    "MDI/PacketSinkFile.cpp",
    "MDI/PacketSocket.cpp",
    "MDI/PacketSourceFile.cpp",
    "MDI/Pft.cpp",
    "MDI/RCITagItems.cpp",
    "MDI/RSCITagItemDecoders.cpp",
    "MDI/RSISubscriber.cpp",
    "MDI/TagPacketDecoder.cpp",
    "MDI/TagPacketDecoderMDI.cpp",
    "MDI/TagPacketDecoderRSCIControl.cpp",
    "MDI/TagPacketGenerator.cpp",
    "mlc/BitInterleaver.cpp",
    "mlc/ChannelCode.cpp",
    "mlc/ConvEncoder.cpp",
    "mlc/EnergyDispersal.cpp",
    "mlc/Metric.cpp",
    "mlc/MLC.cpp",
    "mlc/QAMMapping.cpp",
    "mlc/TrellisUpdateMMX.cpp",
    "mlc/TrellisUpdateSSE2.cpp",
    "mlc/ViterbiDecoder.cpp",
    "MSCMultiplexer.cpp",
    "ofdmcellmapping/CellMappingTable.cpp",
    "ofdmcellmapping/OFDMCellMapping.cpp",
    "OFDM.cpp",
    "Parameter.cpp",
    "PlotManager.cpp",
    "ReceptLog.cpp",
    "resample/Resample.cpp",
    "resample/ResampleFilter.cpp",
    "Scheduler.cpp",
    "SDC/SDCReceive.cpp",
    "SDC/SDCTransmit.cpp",
    "SDC/audioparam.cpp",
    "ServiceInformation.cpp",
    "SimulationParameters.cpp",
    "sound/audiofilein.cpp",
    "sourcedecoders/aac_codec.cpp",
    "sourcedecoders/AudioCodec.cpp",
    "sourcedecoders/fdk_aac_codec.cpp",
    "sourcedecoders/AudioSourceDecoder.cpp",
    "sourcedecoders/AudioSourceEncoder.cpp",
    "sourcedecoders/null_codec.cpp",
    "sourcedecoders/opus_codec.cpp",
    "spectrumanalyser.cpp",
    "sync/FreqSyncAcq.cpp",
    "sync/SyncUsingPil.cpp",
    "sync/TimeSync.cpp",
    "sync/TimeSyncFilter.cpp",
    "sync/TimeSyncTrack.cpp",
    "tables/TableCarMap.cpp",
    "tables/TableFAC.cpp",
    "tables/TableStations.cpp",
    "TextMessage.cpp",
    "util/CRC.cpp",
    "util/FileTyper.cpp",
    "util/LogPrint.cpp",
    "util/Reassemble.cpp",
    "util/Settings.cpp",
    "util/Utilities.cpp",
    "Version.cpp",
    "sound/soundnull.cpp",
    "DrmTransceiver.cpp",
    "sound/soundinterface.cpp",
    "sound/selectioninterface.cpp",
    "MSC/logicalframe.cpp",
    "MSC/audiosuperframe.cpp",
    "MSC/aacsuperframe.cpp",
    "MSC/xheaacsuperframe.cpp",
    "MSC/frameborderdescription.cpp",
    "resample/cspectrumresample.cpp",
    "resample/caudioresample.cpp",
    "sourcedecoders/reverb.cpp",
    "sourcedecoders/caudioreverb.cpp",
];

/// Whether the *target* is the MinGW-w64 flavour of Windows — the
/// `x86_64-pc-windows-gnu` triple the release build uses — rather than MSVC.
///
/// Unlike the `cfg!` tests elsewhere in this file this one has to come from the
/// environment: a build script is compiled for the host, and the Windows CI
/// host toolchain is the MSVC one while the target it builds is the GNU one.
fn target_is_windows_gnu() -> bool {
    let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    os == "windows" && env == "gnu"
}

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let dream = manifest.join("../../vendor/dream");
    // The faad2 headers, exported by `sdroxide-faad2`'s build script.
    let faad2_include = PathBuf::from(
        std::env::var("DEP_FAAD2_INCLUDE").expect("sdroxide-faad2 exports its include directory"),
    );

    if !dream.join("src/DrmReceiver.h").exists() {
        panic!("vendored Dream sources are missing at {}", dream.display());
    }

    // Link order, and it runs both ways: GNU ld resolves static archives in
    // command-line order and never rescans, so every archive has to appear
    // after everything that uses it. cc prints `rustc-link-lib=static=` in
    // `.compile()` order, which is the order the archives reach the linker.
    // (1) The shim before Dream — an archive that *uses* Dream's symbols has
    // to appear *before* the archive that defines them: `libsdroxide_drm_shim.a`
    // behind `libsdroxide_dream.a` leaves every `CDRMReceiver`/`CParameter`
    // reference in the shim unresolved when nothing else pulls Dream in (the
    // test binaries; the full `sdroxide` binary links because other objects
    // reference the same archives). (2) The system libraries last of all —
    // the Dream and shim objects reference libstdc++/winpthread/ws2_32 (and
    // on unix libm/libdl), so printing those inside `build_shim` put the
    // runtime *before* Dream's objects and the same no-rescan rule failed the
    // link from the other side. `emit_system_libs` below is what keeps them
    // after both archives.
    build_shim(&manifest, &dream, &faad2_include);
    build_dream(&manifest, &dream, &faad2_include);
    emit_system_libs();
    generate_bindings(&manifest, &out);

    println!("cargo:rerun-if-changed=src/drm_shim.cpp");
    println!("cargo:rerun-if-changed=src/drm_shim.h");
    println!("cargo:rerun-if-changed=src/fftw_compat.c");
    println!("cargo:rerun-if-changed=include");
    println!("cargo:rerun-if-changed={}", dream.join("src").display());
    println!("cargo:rerun-if-changed={}", manifest.join("../../vendor/fdk-aac/include").display());
}

/// The `HAVE_*` set upstream's `dream.pro` defines on unix, which is really a
/// statement about the C library rather than the platform — the same set is
/// right for every target here. What is *not* defined matters more: no
/// `HAVE_LIBSNDFILE`, `HAVE_SPEEX`, `HAVE_LIBPCAP`, `HAVE_LIBHAMLIB`,
/// `HAVE_LIBGPS`, `HAVE_LIBZ`, and no Qt, so every optional dependency
/// compiles out.
fn common_defines(build: &mut cc::Build) {
    for def in [
        "HAVE_DLFCN_H",
        "HAVE_MEMORY_H",
        "HAVE_STDINT_H",
        "HAVE_STDLIB_H",
        "HAVE_STRINGS_H",
        "HAVE_STRING_H",
        "STDC_HEADERS",
        "HAVE_INTTYPES_H",
        "HAVE_SYS_STAT_H",
        "HAVE_SYS_TYPES_H",
        "HAVE_UNISTD_H",
    ] {
        build.define(def, None);
    }
    // Selects the sound shim in `include/sdrx_sound.h` over a sound card, and
    // links faad2 directly instead of dlopening it.
    build.define("WIN32", None);
    build.define("USE_SDROXIDE_SOUND", None);
    build.define("USE_FAAD2_LIBRARY", None);
    build.define("SDROXIDE_NO_AAC_ENCODER", None);
    // xHE-AAC, the codec most surviving DRM30 broadcasters moved to. Dream's
    // FDK-AAC codec is compiled in and `HAVE_USAC` turns on its USAC branches,
    // but the library itself is *not* linked: the Fraunhofer licence carries
    // restrictions GPL-3.0-or-later cannot absorb, so `SDROXIDE_FDK_DLOPEN`
    // makes `fdk_aac_dll.h` look it up at run time instead. Without it
    // installed, `FdkAacCodec::CanDecode` returns false and an xHE-AAC service
    // falls back to the null codec, which is where this build stood before.
    build.define("HAVE_LIBFDK_AAC", None);
    build.define("HAVE_USAC", None);
    build.define("SDROXIDE_FDK_DLOPEN", None);
    // The EPG decoder is the one place in the receiver that writes to disk, and
    // it names the file from the broadcast. Nothing here reads those files back.
    build.define("SDROXIDE_NO_DATA_FILES", None);
    // Dream announces every codec library it probes for on stderr; the
    // host has its own logging and a GUI has no terminal to print to.
    build.define("SDROXIDE_QUIET", None);
    build.define("EXECUTABLE_NAME", "sdroxide");
}

fn build_dream(manifest: &Path, dream: &Path, faad2_include: &Path) {
    let src = dream.join("src");

    // Dream is C++ but three of the journaline files are C, and their
    // declarations are `extern "C"` — compiling them as C++ leaves `logit` and
    // friends undefined at link time.
    let mut cxx = cc::Build::new();
    let mut c = cc::Build::new();
    for build in [&mut cxx, &mut c] {
        build
            .include(&src)
            // `sourcedecoders/fdk_aac_codec.cpp` is the one file in the tree
            // that spells an include from the project root (`src/SDC/SDC.h`),
            // which is what upstream's qmake `INCLUDEPATH` gives it. Nothing
            // at this level shadows a header.
            .include(dream)
            .include(manifest.join("include"))
            .include(faad2_include)
            // Headers only — see `vendor/fdk-aac/PROVENANCE.md`. Nothing from
            // this directory reaches the linker.
            .include(manifest.join("../../vendor/fdk-aac/include"))
            .opt_level(2)
            .warnings(false);
        common_defines(build);
    }
    cxx.cpp(true).std("c++11");
    if target_is_windows_gnu() {
        // Suppresses cc's own `-lstdc++`; `build_shim` links the C++ runtime
        // statically instead. See the note there.
        cxx.cpp_link_stdlib(None::<&str>);
    }

    for rel in DREAM_SOURCES {
        if rel.ends_with(".c") {
            c.file(src.join(rel));
        } else {
            cxx.file(src.join(rel));
        }
    }
    // CPacer, which `CAudioFileIn` paces file input with.
    if cfg!(target_os = "windows") {
        cxx.file(src.join("windows/Pacer.cpp"));
        cxx.file(src.join("windows/platform_util.cpp"));
    } else {
        cxx.file(src.join("linux/Pacer.cpp"));
    }

    // `dream` before `dream_c`: dream's objects reference DAB_DATAGROUP_DECODER_*
    // from dream_c, and GNU ld does not rescan.
    cxx.compile("sdroxide_dream");
    c.compile("sdroxide_dream_c");

    // The FFTW stand-in. Its own translation unit so it stays independent of
    // Dream's build flags.
    let mut fft = cc::Build::new();
    fft.file(manifest.join("src/fftw_compat.c"))
        .include(manifest.join("include"))
        .opt_level(3)
        .warnings(false)
        .compile("sdroxide_fftw_compat");
}

fn build_shim(manifest: &Path, dream: &Path, faad2_include: &Path) {
    let mut build = cc::Build::new();
    build
        .cpp(true)
        .std("c++11")
        .file(manifest.join("src/drm_shim.cpp"))
        .include(dream.join("src"))
        .include(manifest.join("include"))
        .include(faad2_include)
        .opt_level(2)
        .warnings(false);
    common_defines(&mut build);
    if target_is_windows_gnu() {
        build.cpp_link_stdlib(None::<&str>);
    }
    build.compile("sdroxide_drm_shim");
}

/// The system libraries the Dream and shim objects reference, printed **after**
/// both archives and never before them: GNU ld resolves left to right and
/// never rescans, so a library has to appear after everything that uses it.
/// Printing them inside `build_shim` put the C++ runtime *before* Dream's
/// objects, and the same no-rescan rule then failed the link from the other
/// side (`std::less`, `_Rb_tree`, … undefined out of `newssvcdec_impl.cpp`).
fn emit_system_libs() {
    // cc links the C++ runtime for the objects it builds; the Dream and shim
    // archives both need it, and on unix Dream's LibraryLoader needs dlopen.
    if cfg!(unix) {
        // Both upstreams link the maths library themselves (`dream.pro`'s
        // `-lm`, faad2's CMake `MATH_LIBRARY`), and this build has to as well.
        // It happens to be redundant on a current glibc, where libm was folded
        // into libc in 2.34 — which is exactly why it is easy to leave out and
        // only notice on an older target.
        println!("cargo:rustc-link-lib=dylib=m");
    }
    if cfg!(unix) && !cfg!(target_os = "macos") {
        println!("cargo:rustc-link-lib=dylib=dl");
    }
    // MinGW keeps its C++ runtime in libstdc++-6.dll and cc asks for it by
    // name, so the shipped .exe refused to start on any Windows box without
    // MSYS2 or PothosSDR on PATH ("libstdc++-6.dll not found"). Link it in
    // instead — this is the only C++ in the tree, so it is the only thing that
    // pulls the DLL in.
    //
    // `static:-bundle` rather than plain `static` is the load-bearing part:
    // both make rustc bracket the library with `-Wl,-Bstatic` at the final
    // link, but `static` alone would also try to copy libstdc++.a into this
    // crate's rlib, and rustc has no `-L` that finds it. libwinpthread follows
    // because MSYS2 builds libstdc++ against it, and it is the next DLL the
    // loader would ask for.
    if target_is_windows_gnu() {
        println!("cargo:rustc-link-lib=static:-bundle=stdc++");
        println!("cargo:rustc-link-lib=static:-bundle=winpthread");
    }
    if cfg!(target_os = "windows") {
        println!("cargo:rustc-link-lib=dylib=ws2_32");
    }
}

fn generate_bindings(manifest: &Path, out: &Path) {
    bindgen::Builder::default()
        .header(manifest.join("src/drm_shim.h").to_string_lossy())
        .allowlist_function("sdrx_drm_.*")
        .allowlist_type("sdrx_drm_.*")
        .allowlist_var("SDRX_DRM_.*")
        .layout_tests(false)
        .derive_debug(false)
        .generate()
        .expect("bindgen drm_shim.h")
        .write_to_file(out.join("bindings.rs"))
        .expect("write bindings.rs");
}
