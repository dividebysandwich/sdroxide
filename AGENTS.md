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
