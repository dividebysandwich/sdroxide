#!/usr/bin/env bash
# Print the environment a reproducible build of sdroxide needs, as NAME=value
# lines. CI appends them to $GITHUB_ENV; to rebuild a release by hand, run this
# from the top of the checkout (after rustup, before cargo) and export what it
# prints (the values hold spaces, so not through `eval`):
#
#   while IFS= read -r v; do export "$v"; done < <(packaging/reproducible-env.sh)
#
# On Windows, run it from the Git Bash or MSYS2 shell, after CARGO_BUILD_TARGET
# is set to x86_64-pc-windows-gnu (.github/actions/setup-windows-build does it).
#
# It removes what differs between two builds of the same commit that are not
# the source's doing: the time of the build, and where the checkout and cargo's
# registry happen to live. The toolchain is pinned in
# .github/workflows/release.yml, as are the Linux build image, the macOS runner
# and Xcode, and (through the setup action) the Windows MSYS2 snapshot; every
# -compat, macOS and Windows release ships a .buildinfo naming them, plus the
# exact packages the build had.
#
# Written for the bash 3.2 macOS still ships as /bin/bash.
set -euo pipefail

case "$(uname -s)" in
  MINGW* | MSYS* | CYGWIN*) windows=1 ;;
  *) windows=0 ;;
esac

# Both spellings of the checkout, in case it is reached through a symlink:
# cargo passes paths on as it was given them, while the build scripts that
# canonicalize their vendor directories hand CMake the resolved one. (On macOS
# /tmp and /var are such symlinks, into /private.) On Windows the shell's
# /d/a/... is not what the compilers see: rustc gets D:\a\..., and CMake and
# MinGW gcc mostly D:/a/..., so both of those are added instead.
cargo_home=${CARGO_HOME:-$HOME/.cargo}
if [[ $windows == 1 ]]; then
  roots=("$(cygpath -w "$PWD")" "$(cygpath -m "$PWD")")
  cargo_homes=("$(cygpath -w "$cargo_home")" "$(cygpath -m "$cargo_home")")
else
  roots=("$PWD")
  [[ "$(pwd -P)" != "$PWD" ]] && roots+=("$(pwd -P)")
  cargo_homes=("$cargo_home")
fi

# The commit's own time, so a tool that stamps "now" — gcc's __DATE__, the
# .deb's file times, the tarball's, and GNU ld's PE header timestamp on
# Windows, which it takes from here when set — stamps the same thing on every
# rebuild. `safe.directory` because in a container the checkout is owned by
# another uid.
echo "SOURCE_DATE_EPOCH=$(git -c safe.directory="$PWD" log -1 --format=%ct)"

# rustc: panic locations, `file!()` and bindgen output name source files by
# absolute path. Workspace crates are already relative; the registry, git
# dependencies and everything under target/ (OUT_DIR) are not.
#
# The standard library's own paths are /rustc/<commit>/library/... in the
# shipped std — unless the rust-src component is installed, in which case
# rustc points them at that copy under the rustup home instead, so a rebuild
# with rust-src (rust-analyzer users have it) or from another home directory
# would differ. Mapping the copy back gives /rustc/<commit> either way.
rustflags=""
for r in "${roots[@]}"; do rustflags+="--remap-path-prefix=$r=. "; done
for c in "${cargo_homes[@]}"; do rustflags+="--remap-path-prefix=$c=/cargo "; done
sysroot=$(rustc --print sysroot)
commit=$(rustc -vV | sed -n 's/^commit-hash: //p')
rustflags+="--remap-path-prefix=$sysroot/lib/rustlib/src/rust=/rustc/$commit"

host=$(rustc -vV | sed -n 's/^host: //p')
target=${CARGO_BUILD_TARGET:-$host}

# The native link, on top of that: the linker hashes the whole output into the
# binary's ID — the GNU build ID on Linux, LC_UUID on macOS — before anything
# strips it, so whatever is stripped afterwards still decides the ID. And what
# is stripped names the build directory: the symbol table's file entries on
# Linux, and on macOS the debug map, which lists every object file with debug
# info (RADE's autotools-built Opus has it) by its absolute path under
# target/. Having the linker leave those out itself — --strip-all, -S — means
# the ID is a hash of only what ships. Every release binary is stripped of
# both anyway. Windows needs nothing: a PE file has no content hash, only a
# timestamp, which GNU ld takes from SOURCE_DATE_EPOCH.
native=$rustflags
case "$(uname -s)" in
  Linux) native+=" -C link-arg=-Wl,--strip-all" ;;
  Darwin) native+=" -C link-arg=-Wl,-S" ;;
esac

# Each set goes in its target's own variable rather than RUSTFLAGS. RUSTFLAGS
# would reach the wasm32 web build too, whose linker takes neither option, and
# it would replace `[target.*] rustflags` in .cargo/config.toml, where the
# Windows build's -static-libgcc has to survive; a target's variable is added
# to that list instead. Without --target, cargo applies the host's variable to
# everything, build scripts included, just as it would RUSTFLAGS.
upper=$(printf '%s' "$target" | tr '[:lower:]-' '[:upper:]_')
echo "CARGO_TARGET_${upper}_RUSTFLAGS=$native"
if [[ $windows == 0 ]]; then
  # The web client, which trunk builds with --target wasm32-unknown-unknown.
  echo "CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS=$rustflags"
fi

# C and C++: `__FILE__` in rtl_433's and mongoose's diagnostics, and whatever
# else a build script compiles. The `cc` crate reads the target-suffixed
# variables, and hands them on to the CMake builds through `cmake`; plain CFLAGS
# is left alone on purpose, because autoconf takes a set CFLAGS as a
# replacement for its `-g -O2` and RADE's Opus would lose its optimisation.
# (Opus is built in its own source tree, so its paths are relative already.)
# Apple clang takes -ffile-prefix-map as gcc does, and MinGW gcc compares the
# prefix with / and \ alike, so on Windows only the forward-slash spelling is
# passed: a backslash in CMAKE_C_FLAGS is an escape to CMake.
cflags=""
if [[ $windows == 1 ]]; then
  cflags+="-ffile-prefix-map=${roots[1]}=. -ffile-prefix-map=${cargo_homes[1]}=/cargo"
else
  for r in "${roots[@]}"; do cflags+="-ffile-prefix-map=$r=. "; done
  cflags+="-ffile-prefix-map=$cargo_home=/cargo"
fi
echo "CFLAGS_${target//-/_}=$cflags"
echo "CXXFLAGS_${target//-/_}=$cflags"

if [[ "$(uname -s)" == "Darwin" ]]; then
  # Apple's ar, ranlib and libtool write each member's mtime into a static
  # archive, and the C libraries that CMake and autotools build here (rtl_433,
  # RADE and its Opus, faad2, Dream) are archived with them, not with the `cc`
  # crate, which sets this for itself. GNU ar on Linux is deterministic by
  # default, so only macOS needs it.
  echo "ZERO_AR_DATE=1"
fi
