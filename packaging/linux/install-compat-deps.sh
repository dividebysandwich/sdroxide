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
set -euo pipefail

export DEBIAN_FRONTEND=noninteractive
apt-get update
apt-get install -y --no-install-recommends \
  curl wget zstd build-essential \
  libasound2-dev pkg-config cmake autoconf automake libtool libclang-dev \
  file desktop-file-utils
if [[ "${1-}" == "soapy" ]]; then
  apt-get install -y --no-install-recommends libsoapysdr-dev
fi
