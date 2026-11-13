#!/usr/bin/env bash
# Prepare a GitHub macOS runner for a release build, from the checkout:
#
#   packaging/macos/setup-build-env.sh 26.3          # the standard build
#   packaging/macos/setup-build-env.sh 26.3 soapy    # the -soapysdr build
#
# Shared by release.yml and reproducible.yml so the check builds with exactly
# what the release does. The argument is the Xcode version (MACOS_XCODE in
# release.yml): a runner image carries several, and its default moves with the
# image, so the one picked here is what fixes clang, the linker and the SDK.
#
# The Homebrew packages are what RADE's autotools-built Opus needs (cmake also
# drives rtl_433 and the others). Homebrew is told not to update itself first,
# so it installs the formula versions the runner image was made with rather
# than whatever is newest that day: with that, everything the build sees is
# named by the image version, which the .buildinfo records.
set -euo pipefail

xcode=${1:?usage: $0 XCODE_VERSION [soapy]}
variant=${2-}

app=""
for candidate in "/Applications/Xcode_$xcode.app" "/Applications/Xcode_$xcode.0.app"; do
  if [[ -d "$candidate" ]]; then
    app=$candidate
    break
  fi
done
if [[ -z "$app" ]]; then
  echo "::error::Xcode $xcode is not on this runner image. Installed:"
  ls -d /Applications/Xcode*.app
  exit 1
fi
sudo xcode-select -s "$app/Contents/Developer"
xcodebuild -version

export HOMEBREW_NO_AUTO_UPDATE=1 HOMEBREW_NO_INSTALL_UPGRADE=1 \
  HOMEBREW_NO_INSTALLED_DEPENDENTS_CHECK=1 HOMEBREW_NO_INSTALL_CLEANUP=1
brew install cmake autoconf automake libtool gnu-tar
if [[ "$variant" == "soapy" ]]; then
  brew install pkg-config soapysdr
fi
