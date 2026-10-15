# Provenance: rs1729/RS (`demod/mod`)

Upstream: <https://github.com/rs1729/RS>  
Vendored as submodule: `vendor/rs1729`  
Path used: `demod/mod` (`rs41mod`, `dfm09mod`, `m10mod`, `m10m20mod`, `mts01mod`, …)  
Licence: **GPL-3.0** (see `vendor/rs1729/LICENSE`)  
Author: zilog80 / rs1729

## How SDRoxide uses it

When the Cargo feature `sonde` is enabled (default), `build.rs` in this crate
builds the radiosonde `*mod` helper binaries from `vendor/rs1729/demod/mod` and
stages them under `target/sonde-mod/<profile>/`. At run time the engine prefers
those helpers beside the `sdroxide` executable (release packages copy them
there), then `SDROXIDE_SONDE_MOD_DIR`, then `PATH`.

The helpers are **spawned as separate processes** (OpenWebRX-style), not linked
into the Rust binary. Shipping them with the release avoids a manual host
install; turning off `--features sonde` skips the C build and omits them from
packages.

## Do not modify

Keep the upstream C sources in the submodule unmodified. Build flags and
packaging live in this crate’s `build.rs` and the release workflow.

On Windows, `build.rs` defines `CYGWIN` when compiling the helpers so their
existing `_setmode(stdin, _O_BINARY)` path runs under MinGW/MSVC as well —
without it, text-mode stdin treats `0x1A` in the IQ stream as EOF.
