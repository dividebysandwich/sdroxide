# Headless install — for a club's software manager

Install SDR Oxide Brown, and a station's settings, on as many machines as you
like **without answering a single question**. The idea is the one a club already
uses for its other software: set one machine up properly by hand, take that
machine's configuration away with you, and put both the program and the
configuration on every other machine from a directory like this one.

```
headless-install/
├── install.sh                     Linux (bash)
├── install.py                     Linux, macOS and Windows (Python 3.8+)
├── README.md                      this file
├── sdroxide-2.0.13.1-linux-x86_64.deb      ← the package: any shape below
├── config/                        ← a *configured* station's settings
│   ├── config.toml
│   ├── radios.json
│   ├── radio-1/ …
│   └── …
└── rules/                         ← optional: the 60-sdroxide-*.rules files
```

Then, on every machine:

```sh
sudo ./install.sh                # Linux
sudo python3 install.py          # Linux;  python3 install.py on macOS
```

That is the whole procedure. Nothing is asked, and running it twice is safe.

## What it installs

| | |
|---|---|
| **The program** | `--package`, or the first `sdroxide-*` file beside the script. `.deb` (apt/dpkg), `.rpm` (dnf/zypper/rpm), `.pkg.tar.*` (pacman), `.AppImage`, `.tar.gz` (unpacked under `/opt/sdroxide`), `.msi` (Windows, silently), `.dmg` (macOS, copied into `/Applications`), `.zip`. |
| **Device rules** | `rules/*.rules` → `/usr/lib/udev/rules.d/`, then `udevadm` reload, and the station's user added to `plugdev`. The `.deb` and `.rpm` already carry these, so this step is usually a no-op. |
| **The configuration** | `config/` (or `sdroxide-config*.tar.gz`) → the directory the program itself reads. An existing one is **moved aside first**, as `<config>-<date>.bak`. |
| **A headless server** | only with `--server-unit`: a systemd unit running `sdroxide --server`, enabled and started. |

## Where the configuration goes

The program uses `ProjectDirs::from("org", "sdroxide", "sdroxide")` with
`-brown` appended to the last component, so:

| System | Directory |
|---|---|
| Linux | `~/.config/sdroxide-brown` (or `$XDG_CONFIG_HOME/sdroxide-brown`) |
| macOS | `~/Library/Application Support/org.sdroxide.sdroxide-brown` |
| Windows | `%APPDATA%\sdroxide\sdroxide\config-brown` |

`SDROXIDE_CONFIG_DIR` overrides all of it, which is how you run two independent
stations from one login.

## Taking the configuration from a machine you have set up

Everything the station knows lives in one directory:

```sh
mkdir -p headless-install/config
cp -a ~/.config/sdroxide-brown/. headless-install/config/
```

That carries the radios, the memories, the band plan, the logbook, the display
settings — the lot. Two things worth knowing before you copy it:

- **It is not a secret, but it is not nothing.** `net.json` holds the station's
  server settings and `profiles.json` the sign-in profiles. If you hand the
  directory out, check those first.
- **Paths inside it are absolute** where the program stores a directory (the
  recordings folder, the SSTV folder). A station whose home directory is not the
  one you copied from will point at a folder that does not exist — the program
  falls back to its own default and says so, but setting those two in
  Settings → General on the target machine is cleaner.

## Flags

| Flag | What it does |
|---|---|
| `--package PATH` | the package to install, if it is not the obvious one beside the script |
| `--config PATH` | the configuration to install (a directory or a `.tar.gz`) |
| `--user NAME` | whose configuration to replace. Default: the user behind `sudo`, or the current one. |
| `--server-unit` | also install and enable the `--server` systemd unit (Linux) |
| `--dry-run` | print every step and change nothing. **Do this first on a new machine.** |
| `--verbose` | print every command as it runs |

Exit codes: `0` everything done, `1` a step failed, `2` the arguments were wrong
— so a club's own automation can tell the difference.

## What it deliberately does not do

- **No questions, ever.** There is no prompt to answer and no interactive
  fallback; a missing package or configuration is an error, not a question.
- **No downloads.** The package is the file you put here. A station with no
  route to the internet installs exactly the same as one with.
- **No configuration of its own.** It does not edit what it installs; the
  settings are whatever the machine you copied from had.
- **Nothing outside the documented places**: the package's own files, the udev
  rules, and the one configuration directory (plus its backup).
