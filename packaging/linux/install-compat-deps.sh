#!/usr/bin/env bash
# Install what a -compat build needs on top of the bare Ubuntu 22.04 image.
# Run as root inside the container, from the checkout:
#
#   packaging/linux/install-compat-deps.sh          # the standard build
#   packaging/linux/install-compat-deps.sh soapy    # the -soapysdr build
#
# Shared by release.yml and reproducible.yml so the check builds with exactly
# what the release does. git and ca-certificates are not here: they have to be
# in place before the checkout that brings this script (see release.yml).
#
# curl is for rustup, wget for the Opus model download that RADE's autogen.sh
# does, zstd for the cache archives, and build-essential also brings binutils
# (strip), dpkg-dev (dpkg-shlibdeps, which cargo-deb runs to compute the .deb's
# Depends) and the C++ compiler the vendored DRM receiver needs. Nothing about
# DRM is a *system* dependency: Dream, faad2 and the FFT are all built from
# source in-tree.
#
# The packages come from snapshot.ubuntu.com as the archive stood at the time
# of the commit being built, not from whatever jammy-updates has today: a newer
# gcc, clang or libc headers would make a rebuild months later differ from the
# release for no reason in the source. Every build of one commit therefore gets
# the same set, wherever and whenever it runs. (APT_SNAPSHOT overrides the
# timestamp, as YYYYMMDDTHHMMSSZ.)
set -euo pipefail

export DEBIAN_FRONTEND=noninteractive

if [[ -z "${APT_SNAPSHOT-}" ]]; then
  # safe.directory: in the container the checkout belongs to another uid.
  APT_SNAPSHOT=$(date -u -d "@$(git -c safe.directory="$PWD" log -1 --format=%ct)" +%Y%m%dT%H%M%SZ)
fi
# The image lists archive.ubuntu.com and security.ubuntu.com on amd64 and
# ports.ubuntu.com on arm64; the snapshot service serves all three, both
# architectures, under the one /ubuntu/ path. https, so this has to run after
# ca-certificates is in (release.yml installs it with git, before checkout).
sed -i -E "s#https?://(archive|security|ports)\.ubuntu\.com/ubuntu(-ports)?/?#https://snapshot.ubuntu.com/ubuntu/$APT_SNAPSHOT/#" \
  /etc/apt/sources.list
echo "apt snapshot: $APT_SNAPSHOT"
grep '^deb ' /etc/apt/sources.list
apt-get update
apt-get install -y --no-install-recommends \
  curl wget zstd build-essential \
  libasound2-dev pkg-config cmake autoconf automake libtool libclang-dev \
  file desktop-file-utils
if [[ "${1-}" == "soapy" ]]; then
  apt-get install -y --no-install-recommends libsoapysdr-dev
fi
