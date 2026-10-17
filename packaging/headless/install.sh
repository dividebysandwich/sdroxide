#!/usr/bin/env bash
# Headless install of SDR Oxide Brown: the package, the device rules and a
# station's configuration — with **nothing asked**. Put this script beside what
# it should install and run it:
#
#   headless-install/
#   ├── install.sh                 ← this file
#   ├── install.py                 (same job, macOS and Windows as well)
#   ├── sdroxide-2.0.13.1-linux-x86_64.deb     ← the package, any of the shapes below
#   └── config/                    ← a copy of a *configured* ~/.config/sdroxide-brown
#
# It is written for the club's software manager: configure one machine properly,
# copy its config directory here, and every other machine is one command with no
# questions. Running it twice is safe — the package installs over itself and the
# old configuration is kept beside the new one.
#
#   sudo ./install.sh                    # the machine's own user, system-wide install
#   sudo ./install.sh --user f4nrm       # a named user's configuration
#   ./install.sh --dry-run               # say what it would do, change nothing
#   ./install.sh --help
#
# Exit codes: 0 everything done, 1 a step failed, 2 the arguments were wrong.

set -euo pipefail

PROGRAM="sdroxide"
CONFIG_NAME="sdroxide-brown"
DRY_RUN=0
VERBOSE=0
TARGET_USER=""
SERVER_UNIT=0
PACKAGE=""
CONFIG_SRC=""
HERE="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"

say()  { printf '%s\n' "$*"; }
step() { printf '  %s\n' "$*"; }
warn() { printf 'warning: %s\n' "$*" >&2; }
die()  { printf 'error: %s\n' "$*" >&2; exit 1; }

run() {
    # Every system change goes through here, so --dry-run is honest and the
    # transcript is readable afterwards.
    if [ "$VERBOSE" = 1 ] || [ "$DRY_RUN" = 1 ]; then
        printf '+ %s\n' "$*"
    fi
    if [ "$DRY_RUN" = 0 ]; then
        "$@"
    fi
}

usage() {
    cat <<'USAGE'
Headless install of SDR Oxide Brown. Nothing is asked; everything is taken
from this directory.

  --package PATH   the package to install (default: the first sdroxide-* file
                   beside this script — .deb, .rpm, .pkg.tar.*, .AppImage,
                   .tar.gz)
  --config PATH    the configuration to install (default: ./config, or the
                   first sdroxide-config*.tar.gz beside this script)
  --user NAME      whose configuration to replace (default: the user who ran
                   sudo, or the current user)
  --server-unit    also install and enable a systemd unit running
                   `sdroxide --server` (a headless station; off by default)
  --dry-run        print what would be done and change nothing
  --verbose        print every command as it runs
  --help           this text

The configuration is *copied*, and an existing one is moved aside first as
<config>-<date>.bak, so a failed run never costs a station its settings.
USAGE
}

while [ $# -gt 0 ]; do
    case "$1" in
        --package) PACKAGE="${2:-}"; shift 2 ;;
        --config)  CONFIG_SRC="${2:-}"; shift 2 ;;
        --user)    TARGET_USER="${2:-}"; shift 2 ;;
        --server-unit) SERVER_UNIT=1; shift ;;
        --dry-run) DRY_RUN=1; shift ;;
        --verbose) VERBOSE=1; shift ;;
        --help|-h) usage; exit 0 ;;
        *) usage >&2; exit 2 ;;
    esac
done

[ "$(id -u)" = 0 ] || [ "$DRY_RUN" = 1 ] || warn "not root: a system-wide package install needs sudo"

# ── the package ────────────────────────────────────────────────────────────
if [ -z "$PACKAGE" ]; then
    for f in "$HERE"/sdroxide-*.deb "$HERE"/sdroxide-*.rpm "$HERE"/sdroxide-*.pkg.tar.* \
             "$HERE"/sdroxide-*.AppImage "$HERE"/sdroxide-*.tar.gz; do
        [ -e "$f" ] && { PACKAGE="$f"; break; }
    done
fi
[ -n "$PACKAGE" ] && [ -e "$PACKAGE" ] || die "no package here. Put the release file beside this script, or use --package PATH"
PACKAGE="$(cd -- "$(dirname -- "$PACKAGE")" && pwd)/$(basename -- "$PACKAGE")"

# ── the configuration ──────────────────────────────────────────────────────
if [ -z "$CONFIG_SRC" ]; then
    if [ -d "$HERE/config" ]; then
        CONFIG_SRC="$HERE/config"
    else
        for f in "$HERE"/sdroxide-config*.tar.gz "$HERE"/sdroxide-config*.tgz; do
            [ -e "$f" ] && { CONFIG_SRC="$f"; break; }
        done
    fi
fi

# ── whose configuration ────────────────────────────────────────────────────
if [ -z "$TARGET_USER" ]; then
    TARGET_USER="${SUDO_USER:-$(id -un)}"
fi
[ "$TARGET_USER" = root ] && warn "installing the configuration for root; a desktop station usually wants --user <name>"
HOME_DIR="$(getent passwd "$TARGET_USER" | cut -d: -f6)"
[ -n "$HOME_DIR" ] || die "no such user: $TARGET_USER"

# `directories::ProjectDirs::from("org", "sdroxide", "sdroxide")` puts the config
# in `$XDG_CONFIG_HOME/sdroxide` (usually `~/.config/sdroxide`), and this fork
# appends `-brown` to the last component. The program's own `migrate_shared_config_once`
# does the same, so a copy made anywhere else is simply not found.
CONFIG_HOME="${XDG_CONFIG_HOME:-$HOME_DIR/.config}"
CONFIG_DIR="$CONFIG_HOME/$CONFIG_NAME"

say "SDR Oxide Brown — headless install"
step "package:      $(basename -- "$PACKAGE")"
step "config:       ${CONFIG_SRC:-<none given - the program will use its built-in defaults>}"
step "for user:     $TARGET_USER  ($CONFIG_DIR)"
[ "$DRY_RUN" = 1 ] && step "dry run:      nothing will be changed"

# ── 1. the package ─────────────────────────────────────────────────────────
say
say "1. Installing the program"
case "$PACKAGE" in
    *.deb)
        if command -v apt-get >/dev/null 2>&1; then
            run env DEBIAN_FRONTEND=noninteractive apt-get install -y "$PACKAGE"
        else
            run dpkg -i "$PACKAGE"
            run apt-get -f install -y || true
        fi
        ;;
    *.rpm)
        if command -v dnf >/dev/null 2>&1; then
            run dnf install -y "$PACKAGE"
        elif command -v zypper >/dev/null 2>&1; then
            run zypper --non-interactive install --allow-unsigned-rpm "$PACKAGE"
        else
            run rpm -U --replacepkgs "$PACKAGE"
        fi
        ;;
    *.pkg.tar.*)
        run pacman -U --noconfirm --needed "$PACKAGE"
        ;;
    *.AppImage)
        run install -Dm755 "$PACKAGE" "/usr/local/bin/$PROGRAM"
        ;;
    *.tar.gz)
        # The release tarball: unpack under /opt and link the binary, so an
        # uninstall is one directory.
        run mkdir -p /opt/sdroxide
        run tar -xzf "$PACKAGE" -C /opt/sdroxide --strip-components=1
        if [ -x /opt/sdroxide/sdroxide ] || [ -x "/opt/sdroxide/$(basename -- "${PACKAGE%.tar.gz}")/sdroxide" ]; then
            BIN="$(find /opt/sdroxide -maxdepth 2 -type f -name "$PROGRAM" -perm -u+x | head -1)"
            [ -n "$BIN" ] && run ln -sf "$BIN" "/usr/local/bin/$PROGRAM"
        fi
        ;;
    *) die "unknown package shape: $PACKAGE" ;;
esac

# Device rules: the .deb and .rpm carry them; a tarball does not.
if [ -d "$HERE/rules" ]; then
    say
    say "2. Device rules (so an unprivileged station can open the hardware)"
    run install -Dm644 "$HERE"/rules/*.rules /usr/lib/udev/rules.d/
    run udevadm control --reload-rules
    run udevadm trigger
    step "the station's user must be in the group those rules name (usually plugdev)"
    run usermod -aG plugdev "$TARGET_USER" || true
else
    say
    say "2. Device rules — the package carries them, nothing to do"
fi

# ── 3. the configuration ───────────────────────────────────────────────────
say
say "3. The configuration"
if [ -n "$CONFIG_SRC" ]; then
    TMP_CONFIG=""
    if [ -f "$CONFIG_SRC" ]; then
        TMP_CONFIG="$(mktemp -d)"
        run tar -xzf "$CONFIG_SRC" -C "$TMP_CONFIG"
        # A tarball may hold the directory or its contents; find the layer that
        # has `config.toml` or `radios.json` in it.
        if [ ! -e "$TMP_CONFIG/config.toml" ] && [ ! -e "$TMP_CONFIG/radios.json" ]; then
            INNER="$(find "$TMP_CONFIG" -maxdepth 2 \( -name config.toml -o -name radios.json \) -printf '%h\n' | head -1)"
            [ -n "$INNER" ] && CONFIG_SRC="$INNER"
        else
            CONFIG_SRC="$TMP_CONFIG"
        fi
    fi
    [ -d "$CONFIG_SRC" ] || die "the configuration is neither a directory nor a tarball: $CONFIG_SRC"

    if [ -d "$CONFIG_DIR" ]; then
        BACKUP="$CONFIG_DIR.$(date +%Y%m%d-%H%M%S).bak"
        step "keeping the existing configuration as $(basename -- "$BACKUP")"
        run mv "$CONFIG_DIR" "$BACKUP"
    fi
    run install -d -m700 -o "$TARGET_USER" "$CONFIG_DIR"
    # `/.` copies the contents, whatever the source directory is named.
    run cp -a "$CONFIG_SRC/." "$CONFIG_DIR/"
    # Only when root is writing into somebody else's home.
    if [ "$(id -u)" = 0 ] && [ "$TARGET_USER" != root ]; then
        run chown -R "$TARGET_USER" "$CONFIG_DIR"
    fi
    step "installed $(find "$CONFIG_DIR" -type f | wc -l) files"
    [ -n "$TMP_CONFIG" ] && run rm -rf "$TMP_CONFIG"
else
    warn "no configuration beside this script: the station will start with the program's own defaults"
fi

# ── 4. a headless server, if asked for ─────────────────────────────────────
if [ "$SERVER_UNIT" = 1 ]; then
    say
    say "4. The server unit"
    UNIT=/etc/systemd/system/sdroxide.service
    run tee "$UNIT" >/dev/null <<UNIT_EOF
[Unit]
Description=SDR Oxide Brown (headless station)
After=network-online.target

[Service]
Type=simple
User=$TARGET_USER
ExecStart=/usr/bin/$PROGRAM --server
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
UNIT_EOF
    run systemctl daemon-reload
    run systemctl enable --now sdroxide
    step "serving on the port in $CONFIG_DIR/net.json (4950 unless changed)"
fi

say
if [ "$DRY_RUN" = 1 ]; then
    say "Dry run: nothing was changed."
else
    say "Done. Start it with '$PROGRAM', or as '$PROGRAM --server' on a station with no screen."
fi
