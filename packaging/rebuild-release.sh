#!/usr/bin/env bash
# Rebuild the standard (non-SoapySDR) sdroxide binary of a release, the way
# release.yml builds it, so it can be compared with the published one:
#
#   git clone --recurse-submodules --branch v1.8.0 https://github.com/dividebysandwich/sdroxide
#   cd sdroxide
#   packaging/rebuild-release.sh
#   packaging/verify-release.sh v1.8.0 linux-x86_64-compat=target/rebuild/sdroxide
#
# Run it from the top of the checkout, once the build environment is in place:
# the Rust toolchain named by RUST_TOOLCHAIN in .github/workflows/release.yml
# (`rustup default 1.98.1`, say, or RUSTUP_TOOLCHAIN), with the wasm32 target
# for the web client, the pinned trunk, and what the platform's own setup
# installs — packaging/linux/install-compat-deps.sh in the pinned Ubuntu image
# (packaging/linux/rebuild-in-docker.sh does all of it), on macOS the pinned
# Xcode and packaging/macos/setup-build-env.sh, on Windows the MSYS2 of
# .github/actions/setup-windows-build. docs/reproducible-builds.md has the
# details. A toolchain other than the release's cannot reproduce it, so that is
# checked rather than left to show up as a mismatch.
#
#   --no-web   use the web client already in crates/sdroxide-web/dist instead
#              of building it (CI builds it once on Linux, as the release does)
#
# The binary ends up in target/rebuild/ (sdroxide, or sdroxide.exe).
#
# Keep in step with the `web` and `build` jobs of release.yml; a drift between
# the two shows up as a mismatch when the release is verified.
set -euo pipefail

build_web=1
case "${1-}" in
  "") ;;
  --no-web) build_web=0 ;;
  *) echo "usage: $0 [--no-web]" >&2; exit 2 ;;
esac

die() { echo "rebuild-release: $*" >&2; exit 1; }
[[ -f packaging/reproducible-env.sh && -f .github/workflows/release.yml ]] \
  || die "run it from the top of an sdroxide checkout"

rel=.github/workflows/release.yml
want_rustc=$(sed -n 's/^  RUST_TOOLCHAIN: "\(.*\)"$/\1/p' "$rel")
have_rustc=$(rustc -V | cut -d' ' -f2)
[[ -n "$want_rustc" ]] || die "could not read RUST_TOOLCHAIN from $rel"
[[ "$have_rustc" == "$want_rustc" ]] \
  || die "rustc is $have_rustc, the release was built with $want_rustc (rustup toolchain install $want_rustc, then RUSTUP_TOOLCHAIN=$want_rustc)"

case "$(uname -s)" in
  MINGW* | MSYS* | CYGWIN*)
    # The Windows release is the MinGW target; the setup action sets this, and
    # reproducible-env.sh needs it set before it runs.
    export CARGO_BUILD_TARGET=${CARGO_BUILD_TARGET:-x86_64-pc-windows-gnu}
    bin="target/$CARGO_BUILD_TARGET/release/sdroxide.exe"
    strip_bin() { strip "$1"; }
    ;;
  Darwin)
    # The Xcode is the compiler, linker and SDK of everything but Rust; another
    # one builds a different binary. A warning rather than a stop, to allow a
    # look at how far apart they are. DEVELOPER_DIR picks one without sudo.
    want_xcode=$(sed -n 's/^  MACOS_XCODE: "\(.*\)"$/\1/p' "$rel")
    have_xcode=$(xcodebuild -version | sed -n 's/^Xcode //p')
    if [[ "$have_xcode" != "$want_xcode" && "$have_xcode" != "$want_xcode.0" ]]; then
      echo "rebuild-release: warning: Xcode is $have_xcode, the release was built with $want_xcode; the binary will differ (DEVELOPER_DIR=/Applications/Xcode_$want_xcode.app/Contents/Developer)" >&2
    fi
    bin=target/release/sdroxide
    # -x: a Rust binary's indirect symbol table makes a full strip fail.
    strip_bin() { strip -x "$1"; }
    ;;
  *)
    bin=target/release/sdroxide
    strip_bin() { strip "$1"; }
    ;;
esac

while IFS= read -r v; do export "$v"; done < <(packaging/reproducible-env.sh)

if [[ $build_web == 1 ]]; then
  want_trunk=$(grep -o 'trunk/releases/download/v[0-9.]*' "$rel" | head -n1 | sed 's|.*/v||')
  command -v trunk > /dev/null 2>&1 || die "trunk is not installed (the release uses $want_trunk)"
  have_trunk=$(trunk --version | cut -d' ' -f2)
  if [[ "$have_trunk" != "$want_trunk" ]]; then
    echo "rebuild-release: warning: trunk is $have_trunk, the release used $want_trunk" >&2
  fi
  rm -rf crates/sdroxide-web/dist
  (cd crates/sdroxide-web && trunk build --release)
else
  [[ -f crates/sdroxide-web/dist/index.html ]] \
    || die "--no-web, but crates/sdroxide-web/dist holds no web client"
fi

cargo build --release --locked -p sdroxide --no-default-features --features embed-web,rtl433
strip_bin "$bin"
mkdir -p target/rebuild
cp "$bin" target/rebuild/
out="target/rebuild/${bin##*/}"
if command -v sha256sum > /dev/null 2>&1; then
  sha256sum "$out"
else
  shasum -a 256 "$out"
fi
