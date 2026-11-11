#!/usr/bin/env bash
# Rebuild a release's linux-<arch>-compat binary and its web client from the
# tagged source, in the same pinned Ubuntu image the release was built in, and
# compare both with what the release published:
#
#   packaging/linux/rebuild-in-docker.sh v1.8.0            # this machine's arch
#   packaging/linux/rebuild-in-docker.sh v1.8.0 aarch64    # or name it
#
# Options, before the tag:
#   --repo OWNER/NAME  where the source and release are (default
#                      dividebysandwich/sdroxide)
#   --out DIR          where the rebuilt files go (default ./rebuild-<tag>-<arch>)
#
# Needs docker or podman (set DOCKER to pick one), curl, tar and unzip or
# python3; nothing else from this machine goes into the build. The other
# architecture works too, through the emulation Docker Desktop and
# binfmt/qemu provide, at a fraction of the speed. An hour or so natively.
#
# The recipe is the tag's own, not this checkout's: the image digest is read
# from the tag's release.yml, and inside the container the tag's
# install-compat-deps.sh and rebuild-release.sh do the work. This script and
# verify-release.sh only fetch, run and compare.
set -euo pipefail

repo=dividebysandwich/sdroxide
out=""
die() { echo "rebuild-in-docker: $*" >&2; exit 2; }
while [[ $# -gt 0 ]]; do
  case "$1" in
    --repo) repo=${2:?--repo needs OWNER/NAME}; shift 2 ;;
    --out) out=${2:?--out needs a directory}; shift 2 ;;
    -h | --help) sed -n '2,/^set -euo/p' "$0" | sed '$d; s/^# \{0,1\}//' >&2; exit 2 ;;
    -*) die "unknown option $1" ;;
    *) break ;;
  esac
done
[[ $# -ge 1 && $# -le 2 ]] || die "usage: $0 [--repo OWNER/NAME] [--out DIR] TAG [x86_64|aarch64]"
tag=$1
arch=${2:-$(uname -m)}
case "$arch" in
  x86_64 | amd64) arch=x86_64; docker_platform=linux/amd64 ;;
  aarch64 | arm64) arch=aarch64; docker_platform=linux/arm64 ;;
  *) die "no -compat release for $arch (x86_64 or aarch64)" ;;
esac
out=${out:-rebuild-$tag-$arch}

docker=${DOCKER:-}
if [[ -z "$docker" ]]; then
  if command -v docker > /dev/null 2>&1; then docker=docker
  elif command -v podman > /dev/null 2>&1; then docker=podman
  else die "needs docker or podman"
  fi
fi

here=$(cd "$(dirname "$0")/../.." && pwd)
[[ -x "$here/packaging/verify-release.sh" ]] || die "cannot find packaging/verify-release.sh beside this script"

rel=$(curl -fsSL "https://raw.githubusercontent.com/$repo/$tag/.github/workflows/release.yml") \
  || die "could not read release.yml of $repo at $tag"
image=$(printf '%s\n' "$rel" | grep -o 'ubuntu:22.04@sha256:[0-9a-f]*' | sort -u || true)
trunk=$(printf '%s\n' "$rel" | grep -o 'trunk/releases/download/v[0-9.]*' | head -n1 | sed 's|.*/v||' || true)
toolchain=$(printf '%s\n' "$rel" | sed -n 's/^  RUST_TOOLCHAIN: "\(.*\)"$/\1/p')
if [[ -z "$image" || "$image" == *$'\n'* || -z "$trunk" || -z "$toolchain" ]]; then
  die "$tag's release.yml pins no single image, trunk and RUST_TOOLCHAIN; it predates reproducible builds"
fi
echo "$repo $tag, linux-$arch-compat: image $image, rustc $toolchain, trunk $trunk"

mkdir -p "$out"
out=$(cd "$out" && pwd)

# In the container, as root. The checkout is in the container's own filesystem
# rather than a bind mount (much faster on macOS and Windows), and only the
# results are copied out. Its path differs from CI's on purpose: the build
# must not depend on it.
"$docker" run --rm -i --platform "$docker_platform" \
  -e REPO="$repo" -e TAG="$tag" -e TOOLCHAIN="$toolchain" -e TRUNK="$trunk" \
  -e HOST_UID="$(id -u)" -e HOST_GID="$(id -g)" \
  -v "$out:/out" "$image" bash -s <<'EOF'
set -euo pipefail
export DEBIAN_FRONTEND=noninteractive
apt-get update
apt-get install -y --no-install-recommends git ca-certificates
git clone --quiet --depth 1 --branch "$TAG" --recurse-submodules \
  "https://github.com/$REPO" /src/rebuild/sdroxide
cd /src/rebuild/sdroxide
[[ -x packaging/rebuild-release.sh ]] || { echo "$TAG has no packaging/rebuild-release.sh"; exit 1; }
packaging/linux/install-compat-deps.sh cat
curl -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path --profile minimal \
  --default-toolchain "$TOOLCHAIN" --target wasm32-unknown-unknown
export PATH="$HOME/.cargo/bin:$PATH"
# musl: the static build, since the glibc one may want a newer glibc than 22.04's.
mkdir -p /opt/trunk
curl -sSfL "https://github.com/trunk-rs/trunk/releases/download/v$TRUNK/trunk-$(uname -m)-unknown-linux-musl.tar.gz" \
  | tar -xzf - -C /opt/trunk
export PATH="/opt/trunk:$PATH"
packaging/rebuild-release.sh
cp target/rebuild/sdroxide /out/
rm -rf /out/web && cp -R crates/sdroxide-web/dist /out/web
chown -R "$HOST_UID:$HOST_GID" /out
EOF

"$here/packaging/verify-release.sh" --repo "$repo" "$tag" \
  "linux-$arch-compat=$out/sdroxide" "web=$out/web"
