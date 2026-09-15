# Agent notes — SDR Oxide, the SWL edition

## What this repository is

A **shortwave-listener** fork of [sdroxide](https://github.com/dividebysandwich/sdroxide),
seeded from the CB/SWL fork [`madmedicnl/sdroxide`](https://github.com/madmedicnl/sdroxide).
The plan lives in [`ROADMAP.md`](ROADMAP.md); the listener identity is Phase 1.

## Repository layout and how to work on it

- The **working clone is shared** with the CB/SWL fork. Two branches, two
  repositories:
  - `main` → the CB/SWL fork, `origin` = `madmedicnl/sdroxide`.
  - `swl`  → this repository. Push with
    `git push https://github.com/madmedicnl/sdroxide-swl.git swl:main`.
- Before working here, check out the **`swl` branch** (`git switch swl`). Work
  committed on `main` belongs to the CB fork and must not be pushed here.

## Keeping up with upstream

- `dividebysandwich/sdroxide` is the original; the CB/SWL fork syncs it, and we
  sync the CB/SWL fork. Fetch and merge rather than cherry-pick where possible,
  so the history stays recognisable.
- Watch list:
  - `dividebysandwich/sdroxide` — upstream moves; merge regularly.
  - `jl1nie/mfsk-core#373` — the opt-in `cb-callsigns` feature; when it merges,
    the `mfsk-core` fork pin in `crates/sdroxide-digi/Cargo.toml` can go.

## Build and test

- `cargo build --release` — the full binary (needs the vendored submodules; see
  the README's Building section).
- `cargo test --release --workspace` — everything.
- `cargo check --release --target wasm32-unknown-unknown -p sdroxide-ui` — the
  browser client, which shares the same UI code.
- A local install for trying it: `cargo build --release`, `pkill -x sdroxide`,
  then `cp target/release/sdroxide ~/.cargo/bin/sdroxide`.

## House rules

- Keep changes listener-first: when a choice is between a ham workflow and a
  listening one, this fork takes the listening one.
- Do not touch the vendored subtrees (`vendor/`) except to update a submodule.
- Commit messages: a short imperative subject, then the why. Say what was *not*
  tested when it could not be tested here.
