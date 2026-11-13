# Reproducible builds

Some sdroxide release binaries are *reproducible*: anyone who builds the
release's tagged source again, in the same pinned environment, gets exactly the
same bytes. If your rebuild matches, the binary you downloaded was built from
that source and nothing else. You do not have to trust the machine that built
it.

## What reproduces

The **standard** build (no `-soapysdr` suffix) of these platforms, and the web
client that every binary embeds:

| Platform | Binary inside |
|---|---|
| `linux-x86_64-compat` | `sdroxide-<tag>-linux-x86_64-compat.tar.gz` → `sdroxide/sdroxide` |
| `linux-aarch64-compat` | `sdroxide-<tag>-linux-aarch64-compat.tar.gz` → `sdroxide/sdroxide` |
| `macos-aarch64` | `sdroxide-<tag>-macos-aarch64.tar.gz` → `sdroxide/sdroxide` |
| `macos-x86_64` | `sdroxide-<tag>-macos-x86_64.tar.gz` → `sdroxide/sdroxide` |
| `windows-x86_64` | `sdroxide-<tag>-windows-x86_64.zip` → `sdroxide/sdroxide.exe` |
| `web` | `sdroxide-web-<tag>.zip` (every file) |

These are the builds whose environment is pinned. All of them use the Rust
toolchain set by `RUST_TOOLCHAIN` in `.github/workflows/release.yml`. On top of
that:

- **Linux -compat:** an Ubuntu 22.04 image pinned by digest, with packages from
  the snapshot.ubuntu.com snapshot taken at the commit's time.
- **macOS:** a fixed runner label and Xcode (`MACOS_XCODE`).
- **Windows:** a fixed MSYS2 snapshot (`.github/actions/setup-windows-build`).

Every build removes the build time and the checkout's location from its output
(`packaging/reproducible-env.sh`).

**Compare the binaries, not the downloads.** The `.tar.gz` and `.zip` files are
also built reproducibly, but the `.dmg`, `.msi`, `.AppImage` and `.deb` are
not, because the tools that make them write the time and a random ID into each
one. All of them carry the same binary. The `-soapysdr` builds and the regular
(non-compat) Linux builds are not covered.

Each release has a `SHA256SUMS` file listing every download, and a
`.buildinfo` file per platform naming the exact environment it was built in.

## 1. Print a release's binary hashes

From a checkout of the release's tag (or any recent checkout):

```sh
packaging/verify-release.sh v1.8.0
```

This downloads the archives, checks them against `SHA256SUMS`, opens them and
prints the SHA-256 of each binary. To compare with a binary you built, name the
platform and your file:

```sh
packaging/verify-release.sh v1.8.0 linux-x86_64-compat=target/rebuild/sdroxide
packaging/verify-release.sh v1.8.0 web=crates/sdroxide-web/dist
```

Each line says `MATCH` or `DIFFER`, and the script exits with status 1 if
anything differs. `--repo OWNER/NAME` selects another repository (the default
is `dividebysandwich/sdroxide`). `--keep DIR` keeps the released binaries for a
closer look.

## 2. Rebuild it

Three ways, from the least to the most setup.

### On GitHub, from a fork

This needs only a GitHub account. A fork of a public repository is public too,
so its GitHub-hosted runners cost nothing, macOS included.

1. **Fork** `dividebysandwich/sdroxide`.
2. **Turn on Actions in the fork.** GitHub disables workflows in a new fork:
   open the fork's **Actions** tab once and click the button that enables them.
3. **Run it.** Open **Actions → Reproducibility → Run workflow**, enter the tag
   (for example `v1.8.0`) and leave *repo* empty, which means the repository
   your fork came from.
4. **Read the result.** The run's summary lists `MATCH` or `DIFFER` for each
   platform. The `cmp-<platform>` artifacts hold the rebuilt binary next to the
   released one, so you can hash them yourself. When something differs, a
   `diffoscope` artifact shows where.

The workflow that runs is your fork's copy, and it builds the upstream tagged
source and compares the result with the upstream downloads. None of this
depends on the maintainer's account, tokens or CI runs. A binary uploaded to
the release that was not built from the tagged source shows up as `DIFFER`.

The same workflow also runs automatically after every release, as the last job
of the upstream **Release** run. Its `verify release` summary is the quickest
check, since it needs no fork, but it is the project's own CI vouching for
itself. Your fork's run is the independent one.

#### What the GitHub check proves, and for how long

A fork run is still built on GitHub's machines, like the release itself. It
proves that the downloads are exactly what the tagged source builds into. It
does not prove anything about GitHub's build machines. To take GitHub out of
the picture, rebuild locally: the Docker route below does that fully for Linux.

On GitHub, **macOS** is the platform that benefits most. A local macOS rebuild
only matches if your Mac has the exact Xcode the release used. The fork run
reads the runner label and the Xcode version from the tag's `release.yml` and
selects that Xcode, so the compiler, linker and SDK are the release's own. The
Homebrew autotools come from the runner image, and binaries built on different
image versions have come out identical.

That holds for as long as GitHub keeps the environment:

- **Xcode removed:** GitHub removes older Xcode versions from its macOS images
  over time. Once the pinned one is gone, the macOS job fails and lists the
  Xcode versions the image still has. It fails openly rather than reporting a
  false `DIFFER`.
- **Runner retired:** the same happens when a runner label is retired. The
  Intel runner (`macos-x86_64`) is likely to go first, as GitHub phases out
  Intel Macs.

In practice, a macOS release can be checked on GitHub for months after it is
published, but not for years. Linux (through the pinned image and package
snapshot) and Windows (through the pinned MSYS2 snapshot) depend far less on
what GitHub's images contain.

| Platform | Fork run on GitHub | Without GitHub |
|---|---|---|
| Linux -compat | yes | yes: `rebuild-in-docker.sh` uses the same image and packages |
| Windows | yes | by hand, with some effort |
| macOS | yes, while the runner and Xcode exist | only with the exact Xcode installed |

### Linux, with Docker or Podman

```sh
packaging/linux/rebuild-in-docker.sh v1.8.0            # this machine's architecture
packaging/linux/rebuild-in-docker.sh v1.8.0 aarch64
```

This reads the image digest from the tag's `release.yml`, then clones and
builds the tag inside that image (packages, toolchain, web client and binary)
and compares the result with the release. It needs nothing from your machine
except Docker or Podman, `curl`, `tar` and `unzip`. Building the other
architecture works through emulation, but slowly.

### macOS, by hand

Install the Xcode that `MACOS_XCODE` names in the tag's `release.yml`, then:

```sh
git clone --recurse-submodules --branch v1.8.0 https://github.com/dividebysandwich/sdroxide
cd sdroxide
export DEVELOPER_DIR=/Applications/Xcode_26.3.app/Contents/Developer   # instead of xcode-select
brew install cmake autoconf automake libtool
rustup toolchain install 1.98.1 --target wasm32-unknown-unknown       # RUST_TOOLCHAIN
export RUSTUP_TOOLCHAIN=1.98.1
# trunk, the version release.yml downloads (0.21.14):
#   https://github.com/trunk-rs/trunk/releases
packaging/rebuild-release.sh
packaging/verify-release.sh v1.8.0 macos-aarch64=target/rebuild/sdroxide web=crates/sdroxide-web/dist
```

`rebuild-release.sh` refuses to run with the wrong `rustc`, and warns when the
Xcode is not the release's. Another Xcode builds a different binary, so a newer
one does not match. A macOS rebuild also depends on the Homebrew autotools.
Different GitHub runner images have produced identical binaries, but an unusual
local setup can still differ. While the pinned Xcode is still available on
GitHub, the fork workflow above is the more reliable check.

Windows is easiest through the fork workflow. To build by hand, set up MSYS2
with the packages listed in the release's `.buildinfo`, then run the same two
scripts from its MINGW64 shell.

## When it differs

Run [diffoscope](https://diffoscope.org) on the two binaries:

```sh
packaging/verify-release.sh --keep released v1.8.0 linux-x86_64-compat=target/rebuild/sdroxide
diffoscope released/linux-x86_64-compat/sdroxide target/rebuild/sdroxide
```

A difference in a path (`/home/you/...`) or a date points at the build
environment. Check it against the `.buildinfo`. Anything else is worth an
issue: include the diffoscope output and both hashes.

Releases made before reproducible builds (v1.7.0 and earlier) have no
`SHA256SUMS` and were not built to reproduce. `verify-release.sh` still prints
their hashes, but a rebuild will not match.
