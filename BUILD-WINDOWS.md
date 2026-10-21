# Building sdroxide-brown on Windows (headless / no admin)

This is a headless (no-GUI) dev setup for preparing PRs on Windows, without
administrator rights. It mirrors what CI does (`windows-msi.yml`) and adds a
fast MSVC path for the crates that do not need the vendored C toolchain.

## Two entry points

| Script | Toolchain | Use for |
| --- | --- | --- |
| `dev-gnu.cmd` | MinGW (`x86_64-pc-windows-gnu`) | The **full workspace**, including `sdroxide-rade` (RADE/Opus). This is the PR gate. |
| `dev.cmd` | MSVC (`x86_64-pc-windows-msvc`) | Fast iteration on the pure-Rust crates: `sdroxide-ui` (incl. the wasm check), `-types`, `-proto`, `-config`. |

Both wrap `cargo` with the environment already set (PATH, `OPUS_URL`, target).
They forward all cargo args, e.g. `dev-gnu.cmd check --workspace --all-targets`.

### Why two toolchains

RADE's Opus dependency is built with **autotools**, which MSVC cannot use, so
`sdroxide-rade` only builds under MinGW (this is the same reason CI builds the
Windows binary with MSYS2/MinGW rather than MSVC). Everything else builds under
either, and MSVC is much faster for the UI/wasm loop.

## What was set up

- **Rust targets installed:** `x86_64-pc-windows-gnu`, `x86_64-pc-windows-msvc`,
  `wasm32-unknown-unknown`.
- **User `PATH`** extended (no admin needed) with: MSVC + Windows SDK binaries
  (for `cl`/`link`), PothosSDR `bin` (SoapySDR runtime), and MSYS2 — MSYS2 added
  *after* MSVC so `rustc` never picks up MSYS2's `link.exe`.
- **`sh.exe`, `patch.exe` and the required MSYS DLLs** copied to
  `~\.cargo\bin` — cmake's patch step needs a POSIX `sh`/`patch` on the PATH.
- **Offline Opus archive** for RADE's CMake `ExternalProject`, so the build does
  not reach the network:
  `C:/Users/bigle/AppData/Local/Temp/opencode/opus-pinned.zip`
  (pinned commit `940d4e5`, served via the `OPUS_URL` env var that
  `crates/sdroxide-rade/build.rs` passes to CMake).
  *Note:* `OPUS_URL` must be a **plain Windows path**, not `file:///C:/...` —
  CMake's ExternalProject strips the `file://` scheme and fails on a native path.

## Small source patches (uncommitted, Windows-only build fixes)

Two vendored-C build scripts needed Windows fixes that do not affect Linux CI:

- `crates/sdroxide-rade/build.rs` — reads the `OPUS_URL` env var and forwards
  it to CMake (enables the offline Opus build above).
- `crates/sdroxide-drm/build.rs` — `.std("gnu++11")` → `.std("c++11")` and a
  `WIN32` define, fixing a `netinet/in.h` error under MSVC.

## Known baseline (not regressions)

- **3 `dead_code` warnings** in the workspace check, on **Windows targets only**
  (`sdroxide-audio`'s ALSA `prettify_longname`, `sdroxide-ui`'s evdev keyer
  fields in `morse.rs`). They are Linux-only code compiled for Windows, pre-exist
  on MSVC as well, and the files are unmodified — Linux CI stays silent.
- **277 warnings** in the `wasm32-unknown-unknown` check of `sdroxide-ui`
  (upstream baseline).

## PR gate

Per the repo's contributing rules, a PR needs
`cargo check --workspace --all-targets` clean and the touched crates' tests
passing. On this box that is:

```
dev-gnu.cmd check --workspace --all-targets
cargo test --release -p <touched crates>
```
