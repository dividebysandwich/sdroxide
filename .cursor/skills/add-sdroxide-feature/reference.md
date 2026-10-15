# Feature checklists

## New operating mode

1. Append a variant to `Mode` in `crates/sdroxide-types/src/mode.rs`. Add it to `Mode::ALL` at the picker position. Implement `label`, sideband, filter, `is_*`, and slot timing next to the existing modes.
2. If the mode needs new knobs, append fields on `DigiConfig` / `DigiStatus` (or a new status struct carried by an appended `RadioEvent`). Bump `PROTO_VERSION`.
3. Append any new `Command` variants in `crates/sdroxide-types/src/command.rs`.
4. Engine: handle `SetMode` / the new commands in `engine.rs`. Build the controller in the digi-factory chain beside the other `is_*` arms (`WsprController::new` is the pattern). Specific arms go before the FT8 fall-through.
5. Put the modem in `sdroxide-digi` (or `sdroxide-dsp` when it is a demodulator like DRM). Keep GPL/AGPL crates out of types and out of wasm.
6. UI: `crates/sdroxide-ui/src/app/panels/<mode>.rs` with `fn <mode>_panel(&mut self, ui, cmds, panel_h)`.
7. Call it from the mode chain in `app/frame.rs` (the `if mode.is_*` block). Add `panel_panes` in `app/panels/mod.rs` so phone layout has a tab per pane, including `WFALL`.
8. Mode button / top bar picks the mode up from `Mode::ALL`. Extend `--mode` in `src/main.rs` when the name should parse.

Copy the closest mode. WSPR is a receive list plus status. FT8 is decodes plus a QSO pane. A keyboard mode shares `text_modem`.

## New command on an existing mode

1. Append the `Command` variant.
2. Handle it in `engine.rs` `match cmd` and emit a `RadioEvent` or updated `RadioState` / `DigiStatus`.
3. Push the command from the panel. The frame loop sends the vec after drawing.
4. Bump `PROTO_VERSION` when the new variant or a changed payload struct crosses the socket. An appended enum variant still needs a bump when older servers must be rejected cleanly; follow the comment style at the top of `sdroxide-proto`.

## Settings tab

1. New file under `crates/sdroxide-ui/src/app/settings/`.
2. Register it in `settings/mod.rs` and in `settings_body`'s tab strip.
3. Device probes go through `RadioController::probe`, which the binary implements in `src/devices.rs`. The dialog must work against a remote engine: the answer describes the server machine.
4. Writes that belong to the station wait until `StationConfig` has seeded the form.

## New radio backend

1. Protocol crate `crates/sdroxide-<device>` (native only). Implement the stream type the engine already consumes (`IqSource` from `sdroxide-radio`).
2. Append `Backend` in `crates/sdroxide-types/src/radio.rs` and list it in the picker `ALL`. Add the config struct fields on `RadioConfig`.
3. `src/<device>_source.rs` opens the device and returns caps plus an `IqSource`. Wire it from the backend match the other `*_source.rs` files use.
4. `DeviceProbe` / `ProbeAnswer` variant, arm in `src/devices.rs`, and a settings function in `app/settings/radio.rs`.
5. Workspace dependency in the root `Cargo.toml`. A system library that is optional follows the `soapy` feature pattern (`--no-default-features` must still build).
6. USB device: add a udev rule under `packaging/linux/` matching the existing `60-sdroxide-*.rules` files.
7. Shared radios (one connection, several tabs) register in `src/device_registry.rs`.

## UI-only

Stay inside `sdroxide-ui`. Reuse `chrome::chip`, `angled_frame`, `theme::` colors, and `layout::Tier`. A control that keys the transmitter uses `tx_gated`. Persist view-only layout in `ViewState` (`app` view module); persist operator settings through the engine, not a wasm-local file.
