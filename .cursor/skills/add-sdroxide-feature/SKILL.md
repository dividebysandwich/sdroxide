---
name: add-sdroxide-feature
description: >-
  Add a feature to SDRoxide along the existing crate boundaries, egui panel
  wiring, and postcard wire format. Use when adding a mode, operating panel,
  radio backend, DSP block, settings tab, command, or any operator-facing
  behavior in this repository.
---

# Add an SDRoxide feature

Read [reference.md](reference.md) for the file checklist of the kind you are adding. Follow the architecture, UI, wire-format, and platforms rules already in `.cursor/rules/`.

## 1. Classify the change

| Kind | Touches | Stays out of |
| --- | --- | --- |
| Panel or chrome only | `sdroxide-ui` | engine, proto |
| Control the engine already understands | `sdroxide-ui` pushing an existing `Command` | new protocol |
| New mode or new engine behavior | `sdroxide-types`, `sdroxide-radio` engine, often `sdroxide-digi` or `sdroxide-dsp`, the panel, `PROTO_VERSION` when the wire layout changes | wasm-only decode |
| New radio backend | new `sdroxide-<device>` crate, `src/<name>_source.rs`, `Backend`, `src/devices.rs`, settings radio tab | `sdroxide-ui` hardware calls |

The UI is a view of `RadioState` plus events. Authoritative state lives on the engine thread. A browser client must keep working, which means the new behavior runs natively and arrives as `RadioEvent` / status the wasm UI already knows how to draw.

## 2. Implement in this order

1. **Types.** Append `Mode`, `Command`, `RadioEvent`, or struct fields. Put picker order in `ALL`, not in declaration order. If a serialized struct or a non-append enum change rides the WebSocket, bump `PROTO_VERSION` and note why in `crates/sdroxide-proto/src/lib.rs`.
2. **Engine.** Handle the command in `crates/sdroxide-radio/src/engine.rs`. A digital mode gets a controller constructed in the `make_digi` chain (specific `is_*` arms before the FT8 fall-through) and emits status the UI already drains.
3. **UI.** `impl SdroxideApp` in the matching `app/` module. Panels push `Command`s; `frame.rs` sends them after layout. Register the panel in the mode chain in `app/frame.rs` and in `panel_panes`. Draw with `chrome` and `theme::` accessors. Honor `layout::Tier` (phone panes, touch targets).
4. **Settings, persistence, CLI.** Station-level settings wait for `RadioEvent::StationConfig` before writing. Per-radio config is `radio.json` via `Backend` / `RadioConfig`. Mention a new mode in the `--mode` help when the CLI should accept it.

## 3. Verify

```sh
cargo test -p sdroxide-types -p sdroxide-proto
cargo test -p sdroxide-ui
cargo run -- --siggen
```

Add the engine or digi crate to that test list when you changed it. For a UI change also open the browser client (`trunk build` in `crates/sdroxide-web`, then `sdroxide --server --web-root crates/sdroxide-web/dist`) when the panel is mode UI a remote operator would see.

Wasm-safety check: `cargo check -p sdroxide-ui -p sdroxide-types -p sdroxide-proto --target wasm32-unknown-unknown`.

## 4. Leave these alone

- Do not insert enum variants ahead of existing ones.
- Do not give `sdroxide-types`, `sdroxide-proto`, or the wasm graph a dependency on `sdroxide-radio`, `sdroxide-digi`, `sdroxide-dsp`, or a device crate.
- Do not decode or encode digital modes in the UI. The server and the native engine already do that.
- Do not call `cargo fmt --all` (it formats `vendor/xng`). Format the packages you edited.
