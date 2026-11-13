# Data2G — provenance

`vendor/data2g` is a git submodule holding **Data2G**, an OFDM HF data modem.
It is used **unmodified**: no upstream line is edited, and nothing is copied
out of it into sdroxide's own sources.

| | |
|---|---|
| Upstream | <https://github.com/arodland/Data2G> |
| Pinned at | `f5388f792425d80ac4d175ff1838260e790a3433` (v0.4.3, 2026-11-10) |
| On-air protocol | 15 (`config.PROTOCOL_VERSION`) |
| Author | Andrew Rodland (KC2G) and contributors |
| Licence | Artistic-2.0; the features derived from N0JLF's Ion under Apache-2.0 (`LICENSE_ion`) |

Both licences are listed by the FSF as compatible with GPL-3, so the combined
binary can be conveyed under the workspace's GPL-3.0-or-later; upstream's code
stays under upstream's terms.

## What is built

Only upstream's `data2g_core` CMake target (`native/`), with every app,
binding and test switched off: the modem, the ARQ engine and the host command
language, no Qt, no sound card, no sockets. Its generated tables are committed
upstream, so no Python is needed. sdroxide's own C face is
`shim/data2g_shim.{h,cpp}`. zlib comes from the system, or from the pinned copy
upstream's CMake fetches where there is none (Windows).

## Upgrading

Data2G is beta and changes its on-air format between releases. Two stations
interoperate only on the same protocol version, so move the pin deliberately,
and update the version the tests and the manual name.
