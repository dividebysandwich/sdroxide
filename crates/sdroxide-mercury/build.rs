//! Builds Mercury's daemon library from the vendored sources.
//!
//! `vendor/mercury` is a git submodule pinned to a release and used
//! unmodified. Upstream's `make libmercury_host.a MERCURY_HOST=daemon` is the
//! library a host program links: the modem, the ARQ and the TNC ports, with
//! audio and PTT handed in through `host/mercury_host.h`. Its object lists are
//! mirrored here and compiled with `cc` instead of running upstream's
//! Makefile, which writes its objects into the source tree and probes the host
//! with pkg-config for libraries the daemon build does not use.
//!
//! One change, made without touching the submodule: Mercury binds its TCP
//! ports to every interface, and sdroxide's copy must only ever be reachable
//! from this machine — the operator's clients reach it through sdroxide's own
//! host ports. `tcp_interfaces.c` is compiled through a wrapper in `OUT_DIR`
//! that redefines `INADDR_ANY` to the loopback address before including the
//! upstream file.

use std::fs;
use std::path::PathBuf;

/// `MERCURY_CORE_OBJS` for `MERCURY_HOST=daemon`, plus `MODEM_TOP_OBJS`,
/// `BCAST_FILE_OBJS`, `HOST_API_OBJS` and the bridge (top-level Makefile), and
/// the iniparser and hermes_shm rings the daemon build expects its host to
/// supply. `tcp_interfaces` is compiled through the wrapper instead.
const SOURCES: &[&str] = &[
    "common/cfg_utils",
    "datalink_arq/arq",
    "datalink_arq/arq_trace",
    "datalink_arq/arq_tnc",
    "datalink_arq/arith",
    "datalink_arq/arq_channels",
    "datalink_arq/arq_fsm",
    "datalink_arq/arq_protocol",
    "datalink_arq/arq_timing",
    "datalink_arq/arq_modem",
    "datalink_arq/carousel",
    "datalink_arq/rs_erasure",
    "datalink_broadcast/broadcast",
    "datalink_broadcast/kiss",
    "datalink_broadcast/bcast_file",
    "datalink_broadcast/bcast_carousel",
    "datalink_broadcast/bcast_rx_host",
    "modem/modem",
    "modem/modem_freedv",
    "modem/modem_mfsk",
    "modem/mfsk/mfsk",
    "modem/mfsk/mfsk_ofdm",
    "modem/mfsk/mfsk_sync",
    "modem/mfsk/mfsk_ldpc",
    "modem/mfsk/mfsk_ldpc_1_16",
    "modem/mfsk/mfsk_ldpc_2_16",
    "modem/mfsk/mfsk_ldpc_3_16",
    "modem/mfsk/mfsk_ldpc_5_16",
    "modem/mfsk/mfsk_ldpc_8_16",
    "modem/framer",
    "modem/channel_busy",
    "modem/monitor_frame",
    "common/os_interop",
    "common/ring_buffer_posix",
    "common/shm_posix",
    "common/crc6",
    "common/hermes_log",
    "common/virtual_clock",
    "common/chan",
    "common/queue",
    "common/mercury_engine",
    "common/mercury_cli",
    "common/mercury_modes",
    "common/message_store",
    "common/iniparser/iniparser",
    "common/iniparser/dictionary",
    "data_interfaces/net",
    "gui_interface/ui_communication",
    "gui_interface/ui_status",
    "gui_interface/ui_devices",
    "gui_interface/ui_history",
    "gui_interface/websocket/ws_json",
    "host/audio_host",
    "host/radio_host",
    "host/ui_host",
    "host/api_host",
    "gui_interface/fyne-ui/engine/mercury_bridge",
];

/// `OBJS_COMMON` in `modem/freedv/Makefile` (libfreedvdata.a).
const FREEDV: &[&str] = &[
    "freedv_api",
    "ofdm",
    "freedv_fsk",
    "cohpsk",
    "fsk",
    "kiss_fft",
    "freedv_1600",
    "fdmdv",
    "codec2_fft",
    "codec2",
    "pack",
    "kiss_fftr",
    "newamp1",
    "quantise",
    "lsp",
    "lpc",
    "mbest",
    "phase",
    "sine",
    "varicode",
    "freedv_700",
    "freedv_vhf_framing",
    "fmfsk",
    "filter",
    "freedv_data_channel",
    "ldpc_codes",
    "mpdecode_core",
    "interldpc",
    "golay23",
    "linreg",
    "nlp",
    "codebooknewamp1",
    "codebooknewamp1_energy",
    "codebooknewamp2",
    "codebooknewamp2_energy",
    "postfilter",
    "interp",
    "codebook",
    "codebookd",
    "codebookge",
    "codebookjmv",
    "ofdm_mode",
    "HRA_112_112",
    "HRA_56_56",
    "HRAb_396_504",
    "H_256_768_22",
    "H_256_512_4",
    "HRAa_1536_512",
    "H_128_256_5",
    "H_2064_516_sparse",
    "H_4096_8192_3d",
    "H_16200_9720",
    "H_1024_2048_4f",
    "H_212_158",
    "phi0",
    "gp_interleaver",
    "modem_stats",
];

/// Every directory any of the sub-Makefiles puts on the include path.
const INCLUDES: &[&str] = &[
    ".",
    "host",
    "common",
    "common/iniparser",
    "modem",
    "modem/freedv",
    "modem/mfsk",
    "datalink_arq",
    "datalink_broadcast",
    "datalink_broadcast/raptorq/include",
    "datalink_broadcast/raptorq/deps",
    "data_interfaces",
    "audioio",
    "audioio/ffaudio",
    "gui_interface",
    "radio_io",
    "third_party/chan",
];

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let target = std::env::var("TARGET").unwrap();
    // Mercury is POSIX C (pthreads, POSIX sockets); upstream's Windows build is
    // MinGW-only. Under MSVC the crate builds without it and says so at run time.
    if target.contains("msvc") {
        println!("cargo:rustc-cfg=mercury_unavailable");
        println!("cargo:rustc-check-cfg=cfg(mercury_unavailable)");
        return;
    }
    println!("cargo:rustc-check-cfg=cfg(mercury_unavailable)");

    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let vendor = manifest.join("../../vendor/mercury");
    if !vendor.join("host/mercury_host.h").exists() {
        panic!(
            "vendored Mercury sources are missing at {}\n\
             run: git submodule update --init --recursive",
            vendor.display()
        );
    }
    let vendor = without_verbatim_prefix(
        vendor.canonicalize().expect("canonicalize vendor/mercury"),
    );
    for dir in ["common", "datalink_arq", "datalink_broadcast", "modem", "data_interfaces", "host"]
    {
        println!("cargo:rerun-if-changed={}", vendor.join(dir).display());
    }

    let tcp = vendor.join("data_interfaces/tcp_interfaces.c");
    let wrapper = out.join("tcp_interfaces_loopback.c");
    fs::write(
        &wrapper,
        format!(
            "/* Generated by sdroxide-mercury's build.rs: Mercury's TNC ports, bound\n \
             * to the loopback interface only. */\n\
             #ifdef _WIN32\n\
             #include <winsock2.h>\n\
             #else\n\
             #include <netinet/in.h>\n\
             #include <arpa/inet.h>\n\
             #endif\n\
             #undef INADDR_ANY\n\
             #define INADDR_ANY htonl(INADDR_LOOPBACK)\n\
             #include \"{}\"\n",
            // Forward slashes: GCC reads an #include path literally, and
            // MinGW accepts either separator.
            tcp.display().to_string().replace('\\', "/")
        ),
    )
    .expect("write tcp_interfaces wrapper");

    let raptorq: Vec<PathBuf> =
        ["datalink_broadcast/raptorq/lib", "datalink_broadcast/raptorq/deps/obl"]
            .iter()
            .flat_map(|d| fs::read_dir(vendor.join(d)).unwrap_or_else(|e| panic!("read {d}: {e}")))
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|x| x == "c"))
            .collect();

    let mut b = cc::Build::new();
    b.std("gnu11")
        .define("_GNU_SOURCE", None)
        .define("GIT_HASH", Some("\"sdroxide\""))
        .flag_if_supported("-pthread")
        .flag_if_supported("-Wno-strict-overflow")
        .flag_if_supported("-Wno-stringop-truncation")
        .warnings(false)
        .opt_level(2);
    for inc in INCLUDES {
        b.include(vendor.join(inc));
    }
    for s in SOURCES {
        b.file(vendor.join(format!("{s}.c")));
    }
    for s in FREEDV {
        b.file(vendor.join(format!("modem/freedv/{s}.c")));
    }
    b.files(&raptorq);
    b.file(&wrapper);
    b.compile("mercury_host");

    println!("cargo:rustc-link-lib=dylib=m");
    println!("cargo:rustc-link-lib=dylib=pthread");
    if target.contains("linux") {
        // POSIX shared memory (shm_open) lives in librt on older glibc.
        println!("cargo:rustc-link-lib=dylib=rt");
    }
    if target.contains("windows") {
        println!("cargo:rustc-link-lib=dylib=ws2_32");
    }
}

/// On Windows `canonicalize` returns a verbatim `\\?\D:\…` path, which MinGW
/// gcc cannot open ("\\cfg_utils.c: No such file or directory"). Drop the
/// prefix for plain drive paths; anything else is left as it is.
fn without_verbatim_prefix(p: PathBuf) -> PathBuf {
    let s = p.to_string_lossy();
    match s.strip_prefix(r"\\?\") {
        Some(rest) if rest.as_bytes().get(1) == Some(&b':') => PathBuf::from(rest),
        _ => p,
    }
}
