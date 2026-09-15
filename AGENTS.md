# Agent notes — SDR Oxide, the CB and SWL fork

## What this repository is

A fork of [sdroxide](https://github.com/dividebysandwich/sdroxide) tuned for the
**11 m citizens band** and for **shortwave listening**. Both live in one
program: the CB band, its WSJT-CB interoperability and channel plans, and the
listener's tools — the broadcast schedule, the SWL log, time-shift replay,
scheduled recordings, ECSS, the receive tone and the scan bands. Upstream is the
original; everything here is upstream's program plus those additions.

The listener work also has a listener-only fork at
[`madmedicnl/sdroxide-swl`](https://github.com/madmedicnl/sdroxide-swl).

## Repository layout and how to work on it

- The **working clone is shared** with the listener-only fork. Two branches, two
  repositories:
  - `main` → this fork, `origin` = `madmedicnl/sdroxide`.
  - `swl`  → the listener-only fork. Push with
    `git push https://github.com/madmedicnl/sdroxide-swl.git swl:main`.
- Since the SWL work was merged into `main`, the two branches carry the same
  program. Keep them in step: after work on `main`, merge `main` into `swl` and
  push it, and vice versa.
- The plan for the listener side lives in [`ROADMAP.md`](ROADMAP.md).

## Keeping up with upstream

- `dividebysandwich/sdroxide` is the original. Fetch and merge rather than
  cherry-pick where possible, so the history stays recognisable.
- Features useful to *anyone* (not just CB or SWL) are candidates to offer
  upstream as pull requests rather than keep here.
- Watch list:
  - `dividebysandwich/sdroxide` — upstream moves; merge regularly.
  - `jl1nie/mfsk-core#373` — the opt-in `cb-callsigns` feature; when it merges,
    the `mfsk-core` fork pin in `crates/sdroxide-digi/Cargo.toml` can go.

### When `jl1nie/mfsk-core#373` merges

1. In `crates/sdroxide-digi/Cargo.toml`, replace the
   `git = "https://github.com/madmedicnl/mfsk-core.git"` pin with upstream
   `mfsk-core` and add `"cb-callsigns"` to its `features` list.
2. Refresh `Cargo.lock`; the `madmedicnl/mfsk-core` source should disappear.
3. Confirm the 11 m CB decodes still pass (WSJT-CB callsigns, hashed pairs,
   country flags) — the feature only widens validation, it must not change
   anything else.
4. Note it in the README/commit as "mfsk fork retired".

If #373 is **rejected or closed unmerged**, decide with the user between a
runtime strict/loose policy upstream or keeping the fork pin — do not silently
drop CB validation.

## Regenerating the quick-start PDFs

`docs/cb-quickstart.{en,nl,fr,it}.md` is the source; the matching `.pdf` is
generated and can drift. The TeX engines on this machine are unusable
(`xelatex.fmt` and `latex.fmt` are missing), so render through HTML and headless
Edge instead. From the repo root, once per language (`en`, `nl`, `fr`, `it`) —
the stylesheet is `docs/cb-quickstart-pdf.css`:

```sh
pandoc docs/cb-quickstart.en.md -s -c docs/cb-quickstart-pdf.css -o /tmp/cb-en.html
/opt/microsoft/msedge/msedge --headless=new --disable-gpu --no-sandbox \
  --user-data-dir=/tmp/edge-pdf --print-to-pdf=docs/cb-quickstart.en.pdf \
  --no-pdf-header-footer file:///tmp/cb-en.html
```

Commit the `.md` and the regenerated `.pdf` together, and say so if the `.md`
changed but the PDF was not remade. (`docs/qo100-quickstart.*.pdf` predate this
note and were rendered from a separate HTML source; leave them alone.)

## Cutting a release

1. Bump the workspace version in `Cargo.toml` **first** and let `cargo` refresh
   `Cargo.lock`; commit it. The Windows `.msi` and the macOS bundle take their
   version from `Cargo.toml`, so a re-tag on the same version installs as the
   same version rather than an upgrade.
2. Tag `vX.Y.Z_CB` and push it, then dispatch the release by hand — a tag push
   does **not** run the workflow:
   `gh workflow run release.yml --ref vX.Y.Z_CB --repo madmedicnl/sdroxide`
3. From the release that first carries the stable-named Windows assets, point
   the README's top download links at them —
   `.../releases/latest/download/sdroxide-windows-x86_64.msi` and `.zip` — so
   they stop being edited every release. They do not exist before that release,
   so switch them in the same commit that announces it.
4. Install locally: `cargo build --release`, `pkill -x sdroxide`, then
   `cp target/release/sdroxide ~/.cargo/bin/sdroxide`.

## Build and test

- `cargo build --release` — the full binary (needs the vendored submodules; see
  the README's Building section).
- `cargo test --release --workspace` — everything.
- `cargo check --release --target wasm32-unknown-unknown -p sdroxide-ui` — the
  browser client, which shares the same UI code.

## House rules

- Keep changes listener-first: when a choice is between a ham workflow and a
  listening one, this fork takes the listening one.
- Do not touch the vendored subtrees (`vendor/`) except to update a submodule.
- Commit messages: a short imperative subject, then the why. Say what was *not*
  tested when it could not be tested here.
