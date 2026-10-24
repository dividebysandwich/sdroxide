# ewebsock — provenance

`vendor/ewebsock` is a copy of the **ewebsock** crate, Rerun's WebSocket client
that works natively and on the web, reached through `[patch.crates-io]` in the
workspace manifest rather than as a workspace member.

| | |
|---|---|
| Upstream | <https://github.com/rerun-io/ewebsock> |
| Version | `0.8.0` (crates.io, the newest published release) |
| Author | Emil Ernerfeldt `<emil.ernerfeldt@gmail.com>` and Rerun Technologies AB |
| Licence | MIT OR Apache-2.0 |

`LICENSE-MIT` and `LICENSE-APACHE` are upstream's own files, copied unchanged.
Both are inbound-compatible with this workspace's GPL-3.0-or-later, so the built
binary is GPL as before and upstream's code stays under upstream's terms.

## What is patched

Two changes.

**One error classifier**, in `src/native_tungstenite.rs`. The arm of
`read_from_socket` that recognises a read timeout is factored out into
`is_read_timeout` and widened by two `raw_os_error` values; a counter
(`pending_in_a_row`) is threaded from the two caller loops so the widening
cannot swallow a socket that is genuinely stuck, and one test pins what the
classifier lets through. Beyond these two changes no upstream line is edited,
removed or reordered, and `src/web.rs` — the whole of the wasm client's path —
is untouched.

Two lines of the manifest change, neither of them about behaviour. The
`include` list is repointed from `../LICENSE-*` to `LICENSE-*`, because the
licences sit beside the crate here rather than a directory up; and
`unused_must_use = "allow"` joins upstream's own `[lints.rust]`, because a crate
reached through `[patch.crates-io]` is built as a path dependency and so does
not get the `--cap-lints allow` a registry dependency does — without it three of
upstream's warnings appear on every build of this workspace.

**One option**, `Options::tls_config` (native, `tls` feature only), with the
`TlsConfig` newtype it carries and the `dep:rustls` the `tls` feature now
enables. With it set, `src/native_tungstenite.rs` dials through
`connect_with_tls` — tungstenite's own `connect_with_config`, step for step,
with the rustls connector named instead of defaulted, which upstream tungstenite
has no blocking entry point for. Without it, the upstream call is made exactly
as before. The `tokio` client ignores the option. It is how sdroxide's remote
client accepts a station's self-signed certificate once the operator has
trusted it (issue #736); see `crates/sdroxide-ui/src/remote_tls.rs`.

## Why the patch exists

`ewebsock`'s native transport is one thread that alternates between draining the
outgoing queue and reading the socket, and what stops the read blocking for ever
is `SO_RCVTIMEO` — 10 ms by default (`Options::read_timeout`). Every timeout
comes back as an error and is classified: `WouldBlock`, or `TimedOut` on
Windows, means "nothing to read"; anything else ends the connection.

On Windows there is a third answer. A blocking `recv` with `SO_RCVTIMEO` set is
implemented over an overlapping receive, and when the timeout fires while the
AFD driver still has the request queued, the next call reports
`ERROR_IO_PENDING` (997) — the operation is already in progress — rather than a
timeout. Rust has no `ErrorKind` for it, so it arrives as an uncategorised I/O
error and upstream treats it as the connection failing.

Nothing has failed. The socket reads normally on the next attempt. But a client
polling at 100 Hz on four sockets meets it eventually, which is what sdroxide's
Windows remote clients saw after ten to thirty minutes:

```
read: IO error: An overlapping I/O operation is in progress.. (os error 997)
```

— the whole session gone, and a **Reconnect** button to press. (sdroxide now
redials by itself when a link drops, which covers this and everything else that
can drop one; this is what stops the drop happening at all.)

The counter is the other half of the patch and is there because ignoring an
error is only safe while it stays transient. sdroxide sends no keepalive, so a
socket that answered nothing but 997 for ever would present as a frozen window
rather than as a lost connection — worse than the bug. A thousand in a row, some
ten seconds at the default timeout, is far beyond anything a live connection
produces and is reported as upstream would have reported the first one.

## How to remove this

Both changes have to be upstream first. When upstream classifies
`ERROR_IO_PENDING` as a timeout — or moves off `SO_RCVTIMEO` altogether, which
its own `read_timeout` docs call a TODO — *and* takes a rustls configuration
(or a tungstenite `Connector`) of the caller's, delete `vendor/ewebsock`, drop
the `ewebsock` line from `[patch.crates-io]` and from the workspace `exclude`,
bump the version in `crates/sdroxide-ui/Cargo.toml` to whatever carries both,
and point `remote_tls::options_for` at upstream's option.
