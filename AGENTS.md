# Agent notes

## Watch list — open upstream PRs from this fork

Check these at the start of a session (`gh pr view <n> --repo <repo>`); the
whole point is that they outlive any one conversation.

| PR | What | Action when its state changes |
| --- | --- | --- |
| `jl1nie/mfsk-core#373` | opt-in `cb-callsigns` feature so sdroxide can drop the mfsk fork | **see below — the one that needs work** |
| `dividebysandwich/sdroxide#423` | umbrella for the CB/SWL fork features | close once the individual PRs are in |
| `dividebysandwich/sdroxide#450` | decode-list CSV / received-report ADIF | — |
| `dividebysandwich/sdroxide#451` | opt-in first-press rounds to 000 | — |
| `dividebysandwich/sdroxide#452` | browser ADIF / CHIRP import | — |
| `dividebysandwich/sdroxide#453` | PC-keyboard CW straight key | — |
| `dividebysandwich/sdroxide#454` | station profiles | — |
| `dividebysandwich/sdroxide#455` | ten UI themes | — |
| `dividebysandwich/sdroxide#456` | USB sound-card backend | — |
| `dividebysandwich/sdroxide#457` | audible alerts | — |

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

## Regenerating the CB quick-start PDFs

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

## Cutting a CB release

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
