#!/usr/bin/env python3
"""Headless install of SDR Oxide Brown — Linux, macOS and Windows.

The same job as `install.sh`, in Python so a club can hand out one script for
every machine it owns. Put it beside what it should install and run it:

    headless-install/
    ├── install.sh                 Linux, bash
    ├── install.py                 ← this file (any of the three systems)
    ├── sdroxide-2.0.13.1-linux-x86_64.deb     ← the package, any of the
    │                                              shapes listed below
    └── config/                    ← a copy of a *configured* config directory

**Nothing is asked.** Configure one machine properly, copy its configuration
directory in here, and every other machine is one command:

    sudo python3 install.py                 system-wide, the calling user's settings
    sudo python3 install.py --user f4nrm    a named user's settings
    python3 install.py --dry-run            say what it would do, change nothing

Running it twice is safe: the package installs over itself and the previous
configuration is moved aside rather than overwritten.

Exit codes: 0 all done, 1 a step failed, 2 the arguments were wrong.
"""

from __future__ import annotations

import argparse
import os
import platform
import shutil
import subprocess
import sys
import tarfile
import tempfile
from datetime import datetime
from pathlib import Path

PROGRAM = "sdroxide"
CONFIG_NAME = "sdroxide-brown"
HERE = Path(__file__).resolve().parent

# The package shapes this understands, most specific first. A `.deb` is Linux,
# a `.dmg` is macOS, an `.msi` is Windows; the archives are unpacked instead.
PACKAGE_GLOBS = [
    "sdroxide-*.deb",
    "sdroxide-*.rpm",
    "sdroxide-*.pkg.tar.*",
    "sdroxide-*.AppImage",
    "sdroxide-*.msi",
    "sdroxide-*.dmg",
    "sdroxide-*.zip",
    "sdroxide-*.tar.gz",
]


def say(*parts: object) -> None:
    print(" ".join(str(p) for p in parts))


def step(text: str) -> None:
    say("  " + text)


def warn(text: str) -> None:
    print("warning: " + text, file=sys.stderr)


def die(text: str) -> "NoReturn":  # type: ignore[name-defined]
    print("error: " + text, file=sys.stderr)
    raise SystemExit(1)


class Installer:
    def __init__(self, args: argparse.Namespace) -> None:
        self.dry_run: bool = args.dry_run
        self.verbose: bool = args.verbose
        self.package: Path = self._find_package(args.package)
        self.config_src: Path | None = self._find_config(args.config)
        self.user: str = args.user or _default_user()
        self.server_unit: bool = args.server_unit
        self.system: str = platform.system()

    # ── what to install ────────────────────────────────────────────────────
    def _find_package(self, given: str | None) -> Path:
        if given:
            path = Path(given).expanduser()
            if not path.exists():
                die(f"no such package: {path}")
            return path.resolve()
        for pattern in PACKAGE_GLOBS:
            found = sorted(HERE.glob(pattern))
            if found:
                return found[-1].resolve()
        die(
            "no package here. Put the release file beside this script "
            "(sdroxide-*.deb, .rpm, .pkg.tar.*, .AppImage, .msi, .dmg, .zip, "
            ".tar.gz) or use --package PATH"
        )

    def _find_config(self, given: str | None) -> Path | None:
        if given:
            path = Path(given).expanduser()
            if not path.exists():
                die(f"no such configuration: {path}")
            return path.resolve()
        if (HERE / "config").is_dir():
            return HERE / "config"
        for pattern in ("sdroxide-config*.tar.gz", "sdroxide-config*.tgz"):
            found = sorted(HERE.glob(pattern))
            if found:
                return found[-1].resolve()
        return None

    # ── where the configuration goes ───────────────────────────────────────
    def config_dir(self) -> Path:
        """The directory the program itself uses.

        It is `directories::ProjectDirs::from("org", "sdroxide", "sdroxide")`
        with `-brown` appended to the last component — see
        `sdroxide_config::config_dir`. Getting this wrong is the one mistake
        that silently does nothing: the program would not read what was
        written.
        """
        if self.system == "Darwin":
            base = Path.home() / "Library" / "Application Support" / "org.sdroxide.sdroxide"
        elif self.system == "Windows":
            base = Path(os.environ.get("APPDATA", Path.home())) / "sdroxide" / "sdroxide" / "config"
        else:
            base = Path(os.environ.get("XDG_CONFIG_HOME", Path.home() / ".config")) / "sdroxide"
        return base.with_name(base.name + "-brown")

    # ── running things ─────────────────────────────────────────────────────
    def run(self, *cmd: str, check: bool = True, env: dict | None = None) -> None:
        shown = " ".join(str(c) for c in cmd)
        if self.verbose or self.dry_run:
            say("+ " + shown)
        if self.dry_run:
            return
        full_env = {**os.environ, **env} if env else None
        result = subprocess.run(cmd, env=full_env)  # noqa: S603
        if check and result.returncode != 0:
            die(f"failed: {shown}")

    def sudo(self) -> list[str]:
        """Root prefix: nothing on Windows, `sudo` when not already root."""
        if self.system == "Windows":
            return []
        if hasattr(os, "geteuid") and os.geteuid() == 0:
            return []
        return ["sudo"] if shutil.which("sudo") else []

    # ── the steps ──────────────────────────────────────────────────────────
    def install_package(self) -> None:
        say()
        say("1. Installing the program")
        name = self.package.name
        root = self.sudo()
        if name.endswith(".deb"):
            if shutil.which("apt-get"):
                self.run(
                    *root,
                    "env",
                    "DEBIAN_FRONTEND=noninteractive",
                    "apt-get",
                    "install",
                    "-y",
                    str(self.package),
                )
            else:
                self.run(*root, "dpkg", "-i", str(self.package))
                self.run(*root, "apt-get", "-f", "install", "-y", check=False)
        elif name.endswith(".rpm"):
            if shutil.which("dnf"):
                self.run(*root, "dnf", "install", "-y", str(self.package))
            elif shutil.which("zypper"):
                self.run(*root, "zypper", "--non-interactive", "install", str(self.package))
            else:
                self.run(*root, "rpm", "-U", "--replacepkgs", str(self.package))
        elif ".pkg.tar." in name:
            self.run(*root, "pacman", "-U", "--noconfirm", "--needed", str(self.package))
        elif name.endswith(".AppImage"):
            self.run(*root, "install", "-Dm755", str(self.package), f"/usr/local/bin/{PROGRAM}")
        elif name.endswith(".msi"):
            self.run("msiexec", "/i", str(self.package), "/qn", "/norestart")
        elif name.endswith(".dmg"):
            self.install_dmg(root)
        elif name.endswith(".zip"):
            target = self.windows_dir() if self.system == "Windows" else Path.home() / "Applications"
            say(f"  unpacking into {target}")
            if not self.dry_run:
                target.mkdir(parents=True, exist_ok=True)
            self._unzip(self.package, target)
        elif name.endswith(".tar.gz"):
            self.install_tarball(root)
        else:
            die(f"unknown package shape: {name}")

    def windows_dir(self) -> Path:
        base = Path(os.environ.get("LOCALAPPDATA", Path.home())) / "Programs"
        return base / "SDR Oxide Brown"

    def _unzip(self, archive: Path, target: Path) -> None:
        import zipfile

        shown = f"unzip {archive.name} -> {target}"
        if self.verbose or self.dry_run:
            say("+ " + shown)
        if self.dry_run:
            return
        with zipfile.ZipFile(archive) as zf:
            zf.extractall(target)  # noqa: S202 - a release we shipped

    def install_dmg(self, root: list[str]) -> None:
        """macOS: mount, copy the app, unmount."""
        mount = Path("/Volumes/SDR Oxide Brown")
        self.run(*root, "hdiutil", "attach", str(self.package), "-nobrowse", "-mountpoint", str(mount))
        try:
            apps = list(mount.glob("*.app"))
            if not apps:
                die("no .app inside the disk image")
            self.run(*root, "cp", "-R", str(apps[0]), "/Applications/")
        finally:
            self.run(*root, "hdiutil", "detach", str(mount), check=False)

    def install_tarball(self, root: list[str]) -> None:
        """Linux: unpack under /opt and link the binary; anywhere else: ~/Applications."""
        if self.system == "Linux":
            target = Path("/opt/sdroxide")
            say(f"  unpacking into {target}")
            self.run(*root, "mkdir", "-p", str(target))
            # `tarfile` rather than `tar`, so this works in a minimal image with
            # no tar on it; extraction is a Python job either way.
            if not self.dry_run:
                with tarfile.open(self.package) as tf:
                    _extract_stripped(tf, target)
            binary = next(
                (p for p in sorted(target.rglob(PROGRAM)) if p.is_file() and os.access(p, os.X_OK)),
                None,
            )
            if binary is not None:
                self.run(*root, "ln", "-sf", str(binary), f"/usr/local/bin/{PROGRAM}")
            else:
                warn("no executable found in the tarball: link it by hand")
        else:
            target = Path.home() / "Applications"
            say(f"  unpacking into {target}")
            if not self.dry_run:
                target.mkdir(parents=True, exist_ok=True)
                with tarfile.open(self.package) as tf:
                    _extract_stripped(tf, target)

    def install_rules(self) -> None:
        say()
        say("2. Device rules (so an unprivileged station can open the hardware)")
        rules = sorted((HERE / "rules").glob("*.rules"))
        if not rules:
            say("  the package carries them, nothing to do")
            return
        root = self.sudo()
        for rule in rules:
            self.run(*root, "install", "-Dm644", str(rule), f"/usr/lib/udev/rules.d/{rule.name}")
        self.run(*root, "udevadm", "control", "--reload-rules", check=False)
        self.run(*root, "udevadm", "trigger", check=False)
        step("the station's user must be in the group those rules name (usually plugdev)")
        self.run(*root, "usermod", "-aG", "plugdev", self.user, check=False)

    def install_config(self) -> None:
        say()
        say("3. The configuration")
        target = self.config_dir()
        if self.config_src is None:
            warn(
                "no configuration beside this script: the station will start "
                "with the program's own defaults"
            )
            return
        source = self.config_src
        unpacked: tempfile.TemporaryDirectory | None = None
        if source.is_file():
            unpacked = tempfile.TemporaryDirectory(prefix="sdroxide-config-")
            with tarfile.open(source) as tf:
                _extract_stripped(tf, Path(unpacked.name))
            root_dir = Path(unpacked.name)
            # A tarball may hold the directory or its contents: find the layer
            # that carries the settings themselves.
            if not (root_dir / "config.toml").exists() and not (root_dir / "radios.json").exists():
                inner = [
                    p.parent
                    for p in root_dir.rglob("*")
                    if p.name in ("config.toml", "radios.json")
                ]
                if inner:
                    root_dir = inner[0]
            source = root_dir

        try:
            if target.exists():
                backup = target.with_name(
                    f"{target.name}.{datetime.now():%Y%m%d-%H%M%S}.bak"
                )
                step(f"keeping the existing configuration as {backup.name}")
                if not self.dry_run:
                    shutil.move(str(target), str(backup))
            if not self.dry_run:
                target.mkdir(parents=True, exist_ok=True)
                for item in source.iterdir():
                    dest = target / item.name
                    if item.is_dir():
                        shutil.copytree(item, dest, dirs_exist_ok=True)
                    else:
                        shutil.copy2(item, dest)
                # Only when root is writing into somebody else's home: a
                # user-local install already owns what it wrote, and asking for
                # a password there would be noise, not a fix.
                if (
                    self.system != "Windows"
                    and self.user
                    and hasattr(os, "geteuid")
                    and os.geteuid() == 0
                    and self.user != "root"
                ):
                    self.run("chown", "-R", self.user, str(target), check=False)
                count = sum(1 for _ in target.rglob("*") if _.is_file())
                step(f"installed {count} files")
            else:
                step("would copy every file in place of the one there now")
        finally:
            if unpacked is not None:
                unpacked.cleanup()

    def install_server_unit(self) -> None:
        say()
        say("4. The server unit")
        if self.system != "Linux":
            warn("a systemd unit is a Linux thing; nothing to do here")
            return
        root = self.sudo()
        unit = Path("/etc/systemd/system/sdroxide.service")
        text = (
            "[Unit]\n"
            "Description=SDR Oxide Brown (headless station)\n"
            "After=network-online.target\n\n"
            "[Service]\n"
            "Type=simple\n"
            f"User={self.user}\n"
            f"ExecStart=/usr/bin/{PROGRAM} --server\n"
            "Restart=on-failure\n"
            "RestartSec=5\n\n"
            "[Install]\n"
            "WantedBy=multi-user.target\n"
        )
        if self.verbose or self.dry_run:
            say(f"+ write {unit}")
        if not self.dry_run:
            with tempfile.NamedTemporaryFile("w", delete=False) as tmp:
                tmp.write(text)
                tmp_path = tmp.name
            self.run(*root, "install", "-Dm644", tmp_path, str(unit))
            os.unlink(tmp_path)
        self.run(*root, "systemctl", "daemon-reload")
        self.run(*root, "systemctl", "enable", "--now", "sdroxide")

    def go(self) -> None:
        config_dir = self.config_dir()
        say("SDR Oxide Brown — headless install")
        step(f"package:      {self.package.name}")
        step(f"config:       {self.config_src if self.config_src else '<none given - built-in defaults>'}")
        step(f"for user:     {self.user}  ({config_dir})")
        if self.dry_run:
            step("dry run:      nothing will be changed")

        self.install_package()
        self.install_rules()
        self.install_config()
        if self.server_unit:
            self.install_server_unit()

        say()
        if self.dry_run:
            say("Dry run: nothing was changed.")
        else:
            say(
                f"Done. Start it with '{PROGRAM}', or as '{PROGRAM} --server' "
                "on a station with no screen."
            )


def _extract_stripped(tf: tarfile.TarFile, target: Path) -> None:
    """Extract a tarball, dropping its one leading directory if it has one."""
    members = tf.getmembers()
    names = {m.name.split("/")[0] for m in members if "/" in m.name}
    strip = bool(names) and all("/" in m.name or m.isdir() for m in members) and len(names) == 1
    for member in members:
        if strip:
            parts = member.name.split("/", 1)
            if len(parts) < 2 or not parts[1]:
                continue
            member.name = parts[1]
        tf.extract(member, target, filter="data")  # noqa: S202 - a release we shipped


def _default_user() -> str:
    """The user the configuration is for: the one behind sudo, or the current one."""
    if os.name != "nt":
        sudo_user = os.environ.get("SUDO_USER")
        if sudo_user:
            return sudo_user
    for var in ("USER", "LOGNAME", "USERNAME"):
        if os.environ.get(var):
            return os.environ[var]
    try:
        import getpass

        return getpass.getuser()
    except Exception:  # pragma: no cover - a system with no user at all
        return ""


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(
        prog="install.py",
        description="Headless install of SDR Oxide Brown. Nothing is asked; "
        "everything comes from this directory.",
        epilog="The configuration is copied, and an existing one is moved aside "
        "first as <config>-<date>.bak, so a failed run never costs a station "
        "its settings.",
    )
    parser.add_argument("--package", help="the package to install (default: the first beside this script)")
    parser.add_argument("--config", help="the configuration to install (default: ./config)")
    parser.add_argument("--user", help="whose configuration to replace (default: the caller)")
    parser.add_argument(
        "--server-unit",
        action="store_true",
        help="also install and enable a systemd unit running 'sdroxide --server'",
    )
    parser.add_argument("--dry-run", action="store_true", help="print what would be done, change nothing")
    parser.add_argument("--verbose", action="store_true", help="print every command as it runs")
    args = parser.parse_args(argv)

    Installer(args).go()
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
