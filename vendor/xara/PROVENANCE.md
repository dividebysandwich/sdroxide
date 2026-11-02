# xara — provenance

`vendor/xara/` carries the modem core of **xara**, a VARA-compatible HF/FM
modem in Rust, and `crates/sdroxide-vara` carries a port of xara's station and
TNC glue.

| | |
|---|---|
| Upstream | <https://github.com/x0rro/xara> |
| Commit | `09a9201381b00499ada9d9c816bd05bb8f175705` (2026-10-31, v0.1.3) |
| Author | x0rro |
| Licence | MIT (`LICENSE`, copied verbatim from upstream) |

Upstream describes xara as "primarily meant to be a reference implementation of
VARA's protocol", tested against VARA HF 4.9.0 and VARA FM 4.4.0, and it is
offered here as a clean-room implementation. That claim is upstream's: nothing
in this tree was derived from VARA itself, and nothing here was checked against
VARA by us. A first link against a real VARA station is still to come; the
tests here are xara against xara.

The vendored crates and `crates/sdroxide-vara` stay MIT, unlike the rest of
this workspace (GPL-3.0-or-later). MIT is inbound-compatible with GPL, so the
built binary is GPL as before; keeping upstream's code under upstream's terms
is the same rule `vendor/rax25` follows.

## What was taken

| Upstream | Here | Changes |
|---|---|---|
| `crates/xara-phy/src`, `build.rs` | `vendor/xara/xara-phy` | none |
| `crates/xara-arq/src` | `vendor/xara/xara-arq` | none |
| `crates/xara-host/src/{command,response}.rs` | `vendor/xara/xara-host/src` | none; `lib.rs` declares only these two |
| `crates/xara/src/station/{mod,hf,scheduler}.rs` | `crates/sdroxide-vara/src/station` | FM station dropped; `xara_ui::telemetry::Link` replaced by a local copy of the struct |
| `crates/xara/src/tnc.rs` | `crates/sdroxide-vara/src/tnc.rs` | telemetry, web-UI controls, restart, KISS port and logging removed; replies also fed to the operator's panel |
| `rustfmt.toml`, `LICENSE` | `vendor/xara/` | none |

Every `Cargo.toml` was rewritten: upstream inherits version, edition and
dependency versions from its workspace, and these crates are not members of it
any more (nor of ours — `vendor/xara` is in the workspace `exclude`). The
values are upstream's own.

## What was not taken, and why

- **`xara-host`'s TCP server** (`server.rs`, `bouncer.rs`, `writer.rs`,
  `kiss.rs`). It pauses its listener with Linux `shutdown(SHUT_RD)`/`listen`
  through `libc` and `std::os::fd`, which does not build on Windows. sdroxide
  has its own std-only server in `crates/sdroxide-vara/src/host.rs`, which also
  arbitrates between a TCP host and the in-process ones (the Winlink window, the
  operator's panel).
- **`xara-ui`** (the web UI and telemetry) — sdroxide has its own UI.
- **`xara`** the program: audio backends, PTT, recording, the passive monitor.
  sdroxide owns the radio, the audio and the transmitter.
- **The test vectors** (`vectors/`, ~5 MB) and upstream's tests. Our own tests
  in `crates/sdroxide-vara/tests` run two stations against each other.
- **VARA FM.** Its code is inside the vendored crates (the session types share
  modules), but nothing in sdroxide drives it yet.

## The "registered" switch

Upstream's binary gates `--registered` behind a cargo feature; the libraries
themselves already take it at run time as `HostSettings::registered`, which
lifts the session's speed cap from level 4 to the bandwidth's top level. No
vendored code had to change for sdroxide to offer it as a setting.
