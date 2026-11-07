#!/usr/bin/env bash
# Print the environment a reproducible Linux or macOS build of sdroxide needs,
# as NAME=value lines. CI appends them to $GITHUB_ENV; to rebuild a release by
# hand, run this from the top of the checkout (after rustup, before cargo) and
# export what it prints (the values hold spaces, so not through `eval`):
#
#   while IFS= read -r v; do export "$v"; done < <(packaging/reproducible-env.sh)
#
# It removes what differs between two builds of the same commit that are not
# the source's doing: the time of the build, and where the checkout and cargo's
# registry happen to live. The toolchain is pinned in
# .github/workflows/release.yml, as are the Linux build image and, for macOS,
# the runner image and the Xcode; every -compat and macOS release ships a
# .buildinfo naming them, plus the exact packages the build had.
#
# Written for the bash 3.2 macOS still ships as /bin/bash.
set -euo pipefail

# Both spellings of the checkout, in case it is reached through a symlink:
# cargo passes paths on as it was given them, while the build scripts that
# canonicalize their vendor directories hand CMake the resolved one. (On macOS
# /tmp and /var are such symlinks, into /private.)
roots=("$PWD")
[[ "$(pwd -P)" != "$PWD" ]] && roots+=("$(pwd -P)")
cargo_home=${CARGO_HOME:-$HOME/.cargo}

# The commit's own time, so a tool that stamps "now" — gcc's __DATE__, the
# .deb's file times, the tarball's — stamps the same thing on every rebuild.
# `safe.directory` because in a container the checkout is owned by another uid.
echo "SOURCE_DATE_EPOCH=$(git -c safe.directory="$PWD" log -1 --format=%ct)"

# rustc: panic locations, `file!()` and bindgen output name source files by
# absolute path. Workspace crates are already relative; the registry, git
# dependencies and everything under target/ (OUT_DIR) are not. This lands in
# RUSTFLAGS, which overrides `build.rustflags` in .cargo/config.toml — there is
# none, and its only [target] entry is for Windows, which never runs this.
#
# The standard library's own paths are /rustc/<commit>/library/... in the
# shipped std — unless the rust-src component is installed, in which case
# rustc points them at that copy under the rustup home instead, so a rebuild
# with rust-src (rust-analyzer users have it) or from another home directory
# would differ. Mapping the copy back gives /rustc/<commit> either way.
rustflags=""
for r in "${roots[@]}"; do rustflags+="--remap-path-prefix=$r=. "; done
rustflags+="--remap-path-prefix=$cargo_home=/cargo "
sysroot=$(rustc --print sysroot)
commit=$(rustc -vV | sed -n 's/^commit-hash: //p')
rustflags+="--remap-path-prefix=$sysroot/lib/rustlib/src/rust=/rustc/$commit"
echo "RUSTFLAGS=$rustflags"

# C and C++: `__FILE__` in rtl_433's and mongoose's diagnostics, and whatever
# else a build script compiles. The `cc` crate reads the target-suffixed
# variables, and hands them on to the CMake builds through `cmake`; plain CFLAGS
# is left alone on purpose, because autoconf takes a set CFLAGS as a
# replacement for its `-g -O2` and RADE's Opus would lose its optimisation.
# (Opus is built in its own source tree, so its paths are relative already.)
# Apple clang takes -ffile-prefix-map as gcc does.
cflags=""
for r in "${roots[@]}"; do cflags+="-ffile-prefix-map=$r=. "; done
cflags+="-ffile-prefix-map=$cargo_home=/cargo"
host=$(rustc -vV | sed -n 's/^host: //p')
echo "CFLAGS_${host//-/_}=$cflags"
echo "CXXFLAGS_${host//-/_}=$cflags"

if [[ "$(uname -s)" == "Darwin" ]]; then
  # Apple's ar, ranlib and libtool write each member's mtime into a static
  # archive, and the C libraries that CMake and autotools build here (rtl_433,
  # RADE and its Opus, faad2, Dream) are archived with them, not with the `cc`
  # crate, which sets this for itself. GNU ar on Linux is deterministic by
  # default, so only macOS needs it.
  echo "ZERO_AR_DATE=1"
fi
