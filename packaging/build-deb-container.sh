#!/bin/bash
# Build a Linux x86_64 SoapySDR .deb inside a glibc-2.36 container.
#
# Why a container: this box runs glibc 2.44, and a binary linked here asks for
# GLIBC_2.44 — which Debian 13 (2.41) cannot give it. The release CI builds on
# Ubuntu 24.04 (2.39); the Debian 12 image below is 2.36, so the package runs on
# anything from Debian 12 / Ubuntu 22.04 upwards.
set -euo pipefail

apt-get update -qq
DEBIAN_FRONTEND=noninteractive apt-get install -y -qq --no-install-recommends \
    git ca-certificates curl wget zstd build-essential libasound2-dev pkg-config \
    cmake autoconf automake libtool libclang-dev file desktop-file-utils \
    dpkg-dev libsoapysdr-dev

# Build as the host user, so nothing in the mounted tree ends up root-owned.
groupadd -g "$HOSTGID" builder 2>/dev/null || true
useradd -u "$HOSTUID" -g "$HOSTGID" -m builder 2>/dev/null || true
chown "$HOSTUID:$HOSTGID" /work/out 2>/dev/null || true

su builder -s /bin/bash -c '
set -euo pipefail
export PATH="$HOME/.cargo/bin:$PATH"
cd /work
echo "== toolchain"; rustc --version; cargo --version
echo "== build"; cargo build --release --locked --features embed-web
echo "== cargo-deb"
command -v cargo-deb >/dev/null || cargo install cargo-deb --locked
echo "== package"
mkdir -p out
cargo deb --no-build -p sdroxide --variant soapysdr -o out/
ls -la out/
'
