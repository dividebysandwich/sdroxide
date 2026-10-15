# Contributing to SDRoxide

This is the path from a fresh checkout to a running local build, then the
expectations for a change. Build details that are only needed for packaging
live in the [README](README.md#building).

The project is GPL-3.0-or-later. `sdroxide-deepcw` is AGPL-3.0-only and stays
in the native binary. A contribution is licensed the same way.

## 1. Define the host

Run this in the repository root. It prints the Rust target the rest of this
guide uses.

```sh
OS=$(uname -s 2>/dev/null || true)
ARCH=$(uname -m 2>/dev/null || true)

case "$OS:$ARCH" in
  Darwin:arm64)    TARGET=aarch64-apple-darwin ;;
  Darwin:x86_64)   TARGET=x86_64-apple-darwin ;;
  Linux:x86_64)    TARGET=x86_64-unknown-linux-gnu ;;
  Linux:aarch64|Linux:arm64) TARGET=aarch64-unknown-linux-gnu ;;
  MINGW64_NT*:x86_64|MSYS_NT*:x86_64) TARGET=x86_64-pc-windows-gnu ;;
  *) TARGET= ;;
esac

# Detection failed (empty uname, or a pair this repo does not build).
# The defined development host is macOS on Apple Silicon.
if [ -z "$TARGET" ]; then
  OS=Darwin
  ARCH=arm64
  TARGET=aarch64-apple-darwin
fi

echo "OS=$OS"
echo "ARCH=$ARCH"
echo "TARGET=$TARGET"
```

Defined hosts:

| `uname -s` | `uname -m` | Rust target | What you are on |
| --- | --- | --- | --- |
| `Darwin` | `arm64` | `aarch64-apple-darwin` | macOS Apple Silicon. Defined fallback when detection fails. Written against macOS 27.0.1 on an Apple M4. |
| `Darwin` | `x86_64` | `x86_64-apple-darwin` | macOS Intel |
| `Linux` | `x86_64` | `x86_64-unknown-linux-gnu` | Linux x86_64 |
| `Linux` | `aarch64` or `arm64` | `aarch64-unknown-linux-gnu` | Linux ARM, including Raspberry Pi 4/5 |
| `MINGW64_NT…` | `x86_64` | `x86_64-pc-windows-gnu` | Windows via MSYS2 MinGW. The MSVC target is not a supported build. |

There is no Android or iOS build. A phone uses the browser client in step 6.

Export the three values if you set them by hand, so later steps can refer to
them:

```sh
export SDROXIDE_OS=Darwin
export SDROXIDE_ARCH=arm64
export SDROXIDE_TARGET=aarch64-apple-darwin
```

## 2. Install Rust

Edition 2024 needs Rust **1.85 or newer**, installed with [rustup](https://rustup.rs/).
A distribution `cargo` package cannot add the wasm target used in step 6.

```sh
rustup show active-toolchain
rustc --version
```

If `rustc` is missing, install rustup and open a new shell. Then, once, pin
the host target from step 1:

```sh
rustup default stable
rustup target add "$SDROXIDE_TARGET"
```

On the defined Apple Silicon host that target is `aarch64-apple-darwin`, which
rustup already installs with the macOS toolchain. Add it explicitly only when
`rustup target list --installed` does not list it.

## 3. Install system packages

Pick the block for `$SDROXIDE_OS` from step 1.

### macOS (`Darwin`, both `arm64` and `x86_64`)

Install the Xcode Command Line Tools if `xcode-select -p` fails:

```sh
xcode-select --install
```

Then:

```sh
brew install pkg-config cmake autoconf automake libtool opus
```

A default build also links SoapySDR. Install it when you want that backend:

```sh
brew install soapysdr
```

This Apple Silicon machine already has CMake 4. If configure stops on
`Compatibility with CMake < 3.5 has been removed`, install `opus` as above
(the server then uses the system library) or export:

```sh
export CMAKE_POLICY_VERSION_MINIMUM=3.5
```

### Linux (`x86_64` and `aarch64`)

Debian / Ubuntu:

```sh
sudo apt install build-essential pkg-config cmake autoconf automake libtool \
                 libclang-dev libasound2-dev libopus-dev
```

SoapySDR, when you want the default `soapy` feature:

```sh
sudo apt install libsoapysdr-dev
```

Arch:

```sh
sudo pacman -S base-devel pkgconf cmake autoconf automake libtool clang alsa-lib opus
```

CMake 4 uses the same `CMAKE_POLICY_VERSION_MINIMUM=3.5` export as macOS when
system Opus is absent. ALSA is required on Linux.

### Windows (`x86_64-pc-windows-gnu`)

Use an MSYS2 MinGW64 shell, not a plain `cmd` or PowerShell MSVC prompt.

```sh
pacman -S base-devel mingw-w64-x86_64-toolchain mingw-w64-x86_64-cmake \
          mingw-w64-x86_64-clang autoconf automake libtool
```

```sh
rustup target add x86_64-pc-windows-gnu
```

SoapySDR on Windows comes from the PothosSDR installer. Skip it and build with
`--no-default-features` until you need that backend.

## 4. Fetch the submodules

RADE, rtl_433, faad2, and the HFDL decoder are git submodules. A clone without
them leaves `vendor/rade_c`, `vendor/rtl_433`, `vendor/faad2`, and `vendor/xng`
empty, and the default build stops in `build.rs`.

```sh
git submodule update --init --recursive
```

The first native build also compiles a patched Opus for RADE and needs network
access. Later builds reuse that tree.

## 5. Run the desktop app

From the repository root.

No radio attached (signal generator, full UI):

```sh
cargo run -- --siggen
```

That is the default feature set: SoapySDR plus the vendored rtl_433 decoders.
It is the right command on the defined Apple Silicon host once step 3 installed
`soapysdr`.

Without SoapySDR installed:

```sh
cargo run --no-default-features -- --siggen
```

A release binary, then a device listing:

```sh
cargo build --release
./target/release/sdroxide --probe
```

On Windows the binary is `target/x86_64-pc-windows-gnu/release/sdroxide.exe`
when `CARGO_BUILD_TARGET` is set, and `target/release/sdroxide.exe` otherwise.

Other useful launches, after that binary exists:

```sh
./target/release/sdroxide --freq 14074000 --mode ft8
./target/release/sdroxide --server --web-root crates/sdroxide-web/dist
./target/release/sdroxide --connect 192.168.1.10:4950
```

`--server` without `--web-root` still serves the WebSocket API. The page in a
browser needs step 6, or a build with `--features embed-web` after that step.

Dev profiles already optimize the DSP crates, so a UI-only rebuild stays
usable. A clean release build is much slower than `cargo run`.

### Radiosonde decode (`sonde` Cargo feature)

SONDE mode shells out to rs1729 `*mod` helpers (OpenWebRX style). With the
default feature set, `cargo build` compiles them from `vendor/rs1729` into
`target/sonde-mod/<profile>/`, and release packages copy them **beside** the
`sdroxide` binary so no manual PATH install is needed.

```sh
git submodule update --init --recursive   # pulls vendor/rs1729
cargo build --release                     # includes --features sonde (default)
# helpers appear in target/sonde-mod/release/ and are found via
# current_exe(), SDROXIDE_SONDE_MOD_DIR, or PATH
```

This repo does **not** use OpenWebRX’s `--enable-modulation` flag. The Cargo
equivalent is:

| Goal | Command |
| --- | --- |
| With sonde helpers (default) | `cargo build --release` |
| Without building/shipping `*mod` | `cargo build --release --no-default-features --features soapy,rtl433` |
| Explicit | `cargo build --release --features sonde` |

Needs a C compiler (`cc` crate drives clang/gcc/MinGW). Without the matching
helper the panel shows an unavailable message; the rest of SDRoxide still runs.
Release packages (Linux tarball/deb/AppImage, macOS tarball/app, Windows
zip/MSI) ship the helpers beside `sdroxide`.

## 6. Browser client (optional)

Same UI, compiled to WebAssembly and served by the native server. One-time
setup:

```sh
rustup target add wasm32-unknown-unknown
cargo install --locked trunk
```

CI uses Trunk 0.21.14. Any 0.21 or newer is fine.

```sh
cd crates/sdroxide-web && trunk build && cd ../..
cargo run --release -- --server --web-root crates/sdroxide-web/dist
```

Open `http://127.0.0.1:4950`. Rebuild the web crate and reload the browser
while iterating. `--features embed-web` is for a binary that carries `dist/`
inside itself. Debug builds with that feature still read `dist/` from disk.
Release builds require `trunk build --release` **before** `cargo build`.

## 7. Make a change

Work from a branch on a fork of [dividebysandwich/sdroxide](https://github.com/dividebysandwich/sdroxide).
Open a pull request against that repository. Bug reports and feature requests
use the templates in `.github/ISSUE_TEMPLATE/`.

One concern per pull request. Match the surrounding style: `rustfmt.toml` keeps
short expressions on one line (`use_small_heuristics = "Max"`, 100 columns).

```sh
cargo fmt -p sdroxide-ui -p sdroxide-types
cargo test -p sdroxide-types -p sdroxide-proto -p sdroxide-ui
```

Name every package you edited. `cargo fmt --all` walks into `vendor/xng` and
rewrites upstream sources. Do not commit that diff.

Run the UI you changed:

```sh
cargo run -- --siggen
```

A panel, mode, or layout change that remote operators see also needs the
browser client from step 6. Check phone width as well as the desktop window:
the same egui code picks a Desktop, Tablet, or Phone tier from the viewport.

Where a change goes:

| You are adding | Start in |
| --- | --- |
| A control, color, or panel | `crates/sdroxide-ui`. Draw with `chrome` and `theme`. Push `Command`s. The frame loop sends them after layout. |
| Behavior the radio must perform | `crates/sdroxide-types` (`Command`, `RadioEvent`, `Mode`), then the match in `crates/sdroxide-radio/src/engine.rs`. Digital modes get a controller in `sdroxide-digi` and a panel under `crates/sdroxide-ui/src/app/panels/`. |
| A new radio | A `sdroxide-<device>` crate, an opener in `src/<device>_source.rs`, a `Backend` variant, a probe arm in `src/devices.rs`, and a settings tab. |

`Command`, `Mode`, `RadioEvent`, and `ServerMsg` are postcard-encoded by
declaration order. Append variants. Put picker order in the `ALL` constant.
If a struct field that already crosses the WebSocket changes shape, bump
`PROTO_VERSION` in `crates/sdroxide-proto/src/lib.rs` and add a line to the
version notes at the top of that file.

Keep `sdroxide-types`, `sdroxide-proto`, and `sdroxide-ui` free of native-only
crates (`sdroxide-radio`, `sdroxide-digi`, device crates). Decode and encode
run in the engine. The browser only displays what the server already computed.

Before opening the pull request:

- `cargo test` the crates you touched, and `cargo run -- --siggen` if the UI moved.
- The diff contains no secrets, no `crates/sdroxide-web/dist/`, and no `vendor/` rewrite.
- The pull request says what an operator can do now, and how you ran it (host OS, arch, and the `cargo` command).
