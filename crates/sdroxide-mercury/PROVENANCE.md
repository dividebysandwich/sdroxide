# Mercury — provenance

`vendor/mercury` is a git submodule holding **Mercury**, Rhizomatica's HF data
modem. It is used **unmodified**: no upstream line is edited, and nothing is
copied out of it into sdroxide's own sources.

| | |
|---|---|
| Upstream | <https://github.com/Rhizomatica/mercury> |
| Pinned at | `09f258f84a0b69cb4d8556530a25822617ea17dd` (v1.9.23, 2026-11-09) |
| Author | Rhizomatica and contributors |
| Licence | GPL-3.0-or-later |

Bundled inside it, and compiled in:

| Part | Path | Licence |
|---|---|---|
| codec2 / FreeDV data-mode subset | `modem/freedv` | LGPL-2.1 (`LICENSE-freedv`) |
| iniparser | `common/iniparser` | MIT |
| nanorq (RaptorQ) and oblas | `datalink_broadcast/raptorq` | MIT (upstream sleepybishop/nanorq and sleepybishop/oblas; the vendored copy carries no licence file) |

All are inbound-compatible with the workspace's GPL-3.0-or-later.

## What is built

`build.rs` compiles the object lists of upstream's
`make libmercury_host.a MERCURY_HOST=daemon` with `cc` — the modem, the ARQ,
the TNC ports and the host API in `host/mercury_host.h` — plus the iniparser
and shared-memory rings that build expects its host to supply. Upstream's
Makefile is not run: it writes objects into the source tree and probes for
libraries (ALSA, PulseAudio, hamlib, hidapi, OpenSSL) the daemon build does not
use. When upstream changes those lists, `build.rs` has to follow.

## The one change, made outside the submodule

Mercury binds its TCP ports to every interface. `tcp_interfaces.c` is
compiled through a wrapper generated in `OUT_DIR` that redefines
`INADDR_ANY` as the loopback address before including the upstream file, so
sdroxide's copy is reachable from this machine only.

## Platforms

POSIX C with pthreads; upstream's Windows build is MinGW only. Under MSVC the
crate compiles empty and `available()` answers false.
