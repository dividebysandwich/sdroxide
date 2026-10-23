# Agent notes — SDR Oxide, the CB and SWL fork

> **His name is F4NRM.** The operator says *"Kevin"* by accident — **correct it
> when he does, and use `F4NRM` everywhere**: notes, replies, commits, release
> text. His GitHub handle is `F4NRM` (older notes here say `kevin2008-01`; the
> same person). Prose and quoted speech below still say "Kevin" in places, and
> **quotes are left as they were said** — do not rewrite a quote to fix a name.
>
> **The mode work is the live queue, and it has its own brief.**
> [`MODE-PHASE-4-PROMPT.md`](MODE-PHASE-4-PROMPT.md) is what a session starts with
> now: **finish the missing modes and get TX working for everything fldigi can
> transmit**, and **vendor a mode when the port and the bench have both been
> exhausted** — which supersedes the "port it, do not vendor it" rule of 2026-10-21,
> at the operator's instruction. It ends with a table of every mode we ship, five
> columns, and the mode phase it follows is
> MODE-PHASE-3B-PROMPT.md (removed) (done; its day-1 record is
> MODE-PHASE-3-PROMPT.md (removed)). The paragraph below
> describes that earlier day and is kept for that reason:
> MODE-PHASE-3-PROMPT.md (removed) was what a session started
> with: the PSK coded families (BPSK63F, the R set, the C/RC multi-carrier
> modes), then the loose ends behind them. Its "current truth" section is the
> state **as it stood then** — four families wired receive-only, `PROTO_VERSION`
> 226, a hundred commits local and none pushed — and its "do not re-derive" list
> is the day of measurement traps that cost this phase its time. [`MODE-PROGRESS.md`](MODE-PROGRESS.md)
> is the journal, one line per mode. **The dated sections below are a history**,
> newest first: where they disagree with that brief, the brief is right.
>
> ⛔ **WEFAX, DAB, SSTV and DSC are confirmed working in the field — leave them
> alone.** F4NRM and his radio group report all four working perfectly
> (2026-10-21), so **do not refactor them, do not "improve" a decode, do not take
> them into the mode work and do not re-open their journal entries as work items.**
> A working mode that a whole radio group has confirmed is worth more than a
> tidier one, and a change nobody asked for is how that stops being true. If a
> report ever comes back against one of them, it is a new report about a specific
> fault: start from the report and reproduce it, not from a tidy-up.
>
> **Fresh session, 2026-10-09. Start here.** `main` is green, the whole open
> queue is merged (#22 SSTV KEEP DIAL, #23 the browser `remember` cookie, #24
> the ADS-B noise floor), and `2.0.4_brown` is cut and installed.
> `PROTO_VERSION` is **198** (the Calm theme moved `UiTheme::Default`'s
> discriminant, a `ClientScreen` wire change).
>
> **Contributing (read this first if you are not the maintainer's own agent).**
> A pull request to this fork must be **clean and mergeable as opened** — the
> six rules and the maintainer/contributor split are in **"Pull requests to this
> fork"** below. **Kevin's PRs (kevin2008-01, from Claude sessions) that are
> sensible and merge cleanly are approved and merged without waiting** — see
> that section.
>
> **The next project is undocked mode** (detachable windows — the waterfall on a
> second monitor): read [`UNDOCKED-HANDOVER.md`](UNDOCKED-HANDOVER.md) top to
> bottom — it is the fresh handover and it carries the todo list. Item 1 (the
> state) is done; **item 2, extracting the panadapter draw out of `frame.rs`, is
> the only risky step** and is deliberately on its own.
>
> **And the shape it is being built toward is SDRuno's** — a set of windows for
> the spectrum, the receiver control, the scanner and so on. The window map and
> the remaining todo are in [`SDRUNO-UI-HANDOVER.md`](SDRUNO-UI-HANDOVER.md);
> the operator expects to **switch models when the work reaches MAIN SP** (the
> panadapter window's toolbar), so that file says where the seam is.
>
> ⛔ **PARKED, AND DO NOT BRING IT UP.** The operator's standing instruction,
> 2026-10-17: *"don't mention the sdruno again till i call for it."* It is his own
> pet project, kept **out of the main project**: what exists is on branch
> **`fork/sdruno-clone`** (off `fork/sdruno-look`, which carries the theme), it is
> **not** to be merged into `main`, and it is **not** to appear in a proposal list,
> a reply, a release note or the TODO. Do not raise it as the next thing to do,
> and do not answer an unrelated question by pointing at it. The branch and
> `SDRUNO-UI-HANDOVER.md` stay where they are as the record. **He will call for it
> when he has session room, and not before.**
>
> **Where the SDRuno work stands (2026-10-09): items 1, 2 and 3 are built** —
> the band keypad in the console's left column (digits 0–9 naming the HF
> allocations, **Bands**/**MHz** above, the full band list beside it, not behind
> a dropdown: 28 bands will not fit in ten keys), **MAIN SP**'s toolbar (the
> readout in a bar across the top of the panadapter window, a **DISP** chip, a
> **DOCK** chip), and **AUX SP** (`DetachableModule::AuxPanadapter`, app-id
> `sdroxide-panadapter-aux`, the same draw as SP1 in a window of its own).
>
> **Adding a detachable module is safe now, and that is the thing to keep.**
> `UiSettings::detached` reads through `detached_slots`, which accepts a list of
> any length — because `Settings::load` **quarantines** a `config.toml` it
> cannot parse and answers `Settings::default()`, so a fixed-length array would
> have taken an operator's theme, fonts and layout down with their window
> geometry every time the registry grew. The tests are
> `sdroxide-config/tests/detached_slots_survive_a_module_count.rs` and
> `ui::tests::the_detached_list_loads_at_any_length`.
>
> **The band/mode selector is its own window now (2026-10-10)** —
> `DetachableModule::BandMenu`, app-id `sdroxide-bandmenu`, opening 560×760.
> This is the SDRuno RX-control band panel, and it exists because the **docked
> column cannot be wider than ~280 pt**: the operating panels below it are built
> for ~680 pt, so a docked selector showing its modes in five columns would
> squeeze them and overrun the dock (the #643 guard). In its own window the mode
> sections lay in **five columns** and the operating panel keeps its full width.
> `band_menu_body` is the one body, drawn by the window and the dock alike; while
> the window is out the docked column is not drawn (one selector on screen). The
> entry points are a **⇱ WINDOW** chip in the band popup and on the dock header;
> the **🪟 WINDOWS** menu and Settings → UI pick it up from
> `DetachableModule::ALL`. `the_band_window_lays_the_modes_in_five_columns` pins
> the five columns (and that the dock is fewer) through the pure `mode_grid_cols`.
>
> **Items 1–4 are built, so the window set is the SDRuno one**: the band keypad
> (console), SP1 with its toolbar, AUX SP as a second spectrum, and the MAIN
> window's own **🪟 WINDOWS** button on the radio-tab strip that opens each of
> them by its SDRuno name. **Item 6, workspaces, is built** — saved named
> window arrangements in `workspaces.json`, edited on Settings → UI, with a
> **Ctrl+W** shortcut that opens that page (`Action::Workspaces`, `PROTO_VERSION`
> 198 → 199, bindings schema stamp 2 → 3 so an existing `input.json` picks the
> key up). **Item 5 is built too**: every menu tool window goes through
> `self.tool_window`, so each has a ⇱ chip and a remembered geometry — the table
> in `SDRUNO-UI-HANDOVER.md` item 5 lists them and the locals each carries back.
> The `&mut` locals turned out **not** to be a design problem, and the refactor
> found a **dead EDIT button** in RECORDINGS on the way. **WEFAX is deliberately
> left docked**: its only window is a per-chart viewer, not a menu tool.
> **Settings is a tool window too** (`sdroxide-tool-settings`), and it is the one
> that needed `tool_window_sized`'s `min_size` and `centre_first` — a 900x760 box
> in a corner lands under the top bar, which only rendering it showed.
>
> **Live handovers.** **ALE** ([`ALE-HANDOVER.md`](ALE-HANDOVER.md), issue #262,
> mid-flight — read it before touching ALE). **FST4W**
> ([`FST4W-HANDOVER.md`](FST4W-HANDOVER.md) — planned, not built; the verdict is
> do not ship as wired). **Olivia** ([`OLIVIA-HANDOVER.md`](OLIVIA-HANDOVER.md)
> — receive proven on the air; the open item is whether a real fldigi can copy
> *our* transmission). **STANAG 4285** — receive built and locking real
> captures; the payload needs Annex E. The standard is transcribed in
> [`STANAG4285-EDITION1.md`](STANAG4285-EDITION1.md) and
> [`STANAG4285-HANDOVER.md`](STANAG4285-HANDOVER.md) carries the state.
> The **CW keyer is built and benched** (`353d136f`, "all works!"); its design is
> [`CW-HANDOVER.md`](CW-HANDOVER.md), its outcome is §10 below.
>
> **Phone UI is delegated — off our list** (2026-10-09). The 1.9.17 phone
> *crash* is real and this fork's own phone layouts are still documented in the
> sections below, but the work is somebody else's now: do not pick it up, and do
> not open a handover for it. Its old `PHONE-UI-HANDOVER.md` was removed.
>
> **DAB is Kevin's — delegated, off our list** (2026-10-12). **DAB/DAB+ have
> shipped** and are confirmed on the air: a PlutoSDR at 2.5 Msps decodes DAB+
> cleanly with no dropouts ("fort et clair"), and the MOT slideshow, DLS station
> text, programme type and VU meter all landed with it (his #65–#71, merged
> 2026-10-11). He has the bench, the capture and the hardware, so **DAB is his:
> do not pick up a DAB item, do not "help" with one, and do not open a handover
> for it.** `DAB_ENABLED` is `true` and `Mode::Dab` is back in the band menus —
> several places in this file still describe DAB as *withdrawn*, *disabled* or
> *not yet verifiable*, and those paragraphs are **history, not the state**.
> Same shape as the phone UI being delegated on 2026-10-09.
>
> **The SWL schedule-sources work is in [`SWL-HANDOVER.md`](SWL-HANDOVER.md)** —
> fork-only, from fork issue #69, **scoped but not written**. It is the second
> broadcast schedule source (bebepip runs **Aoki**; **HFCC** also, and it needs a
> *pair* of files), plus the listener's own stations as a real source, plus
> great-circle paths with a **dark-share** on the 3D globe. The three file formats
> are decoded in that file from bebepip's own pasted rows, so **read it before
> touching `broadcast::parse_schedule`** — and note the standing decision:
> **one active source at a time**, not "all of them", because the three overlap
> heavily and he wants the station name on the waterfall for signal ID.
>
> ⛔ **DO NOT MERGE `upstream/main` INTO THIS FORK — WITHOUT EXCEPTION.**
>
> **Upstream is BEHIND this fork, not ahead of it. Merging it in is a regression,
> not an update.** Measured 2026-10-12: the fork is at `2.0.9` /
> `PROTO_VERSION 205`; `upstream/main` is at `1.6.10` / `PROTO_VERSION 179`. The
> fork is **983 commits ahead** and 68 behind, so "68 behind" means *we have moved
> past them*, never *they have work we need*.
>
> A trial merge produced **47 conflicted files**, and the conflicts are not
> ordinary friction — upstream would **un-do fork work**:
>
> - **`Cargo.toml`**: upstream drops the vendored **`mfsk-core` patch** and the
>   `vendor/mfsk-core` exclude. That patch fixes the **FT8 subtract
>   out-of-bounds panic** which kills the decode worker mid-session (upstream has
>   it only as jl1nie/mfsk-core#574 on a later base). Taking upstream's file
>   **reinstates a crash-into-silence bug on the default mode.**
> - it rewrites **`version = "2.0.9"` down to `"1.6.10"`**;
> - **`PROTO_VERSION` 205 → 179** is a rewind of 26 released wire versions.
>
> **The maintainer and the operator handle every upstream merge, individually, by
> hand — cherry-picking specific commits they have chosen. Never run
> `git merge upstream/main`, never merge a tracking branch, and never resolve a
> conflicted merge "by taking upstream's side" to make it go away.**
>
> To *read* upstream (to compare a fix, or check what they changed) is fine and
> encouraged — `git log main..upstream/main`, `git show <sha>`, `git diff` are all
> read-only and safe. Merging is not.
>
> **Every session, in order:** `git fetch upstream` (fetch is safe, merge is
> **not**); read the maintainer's comments on our open upstream PRs (answer only
> where he asked something — silence is not a prompt to re-ping); check the fork's
> **Discussions** (see the support channel note below — the fork's issues are
> **open**, and a bug report arriving as an issue is the normal case); look
> only at upstream issues we have a PR for. The current PR list and the rest of
> the workflow are in "The standing queue and how to check it" below. A tag or a
> nightly runs the full gate before it is cut — see "Cutting a release".

## Session 2026-10-23: three UI fixes from the operator's own screenshots

**The mode parameter row drew its chips on top of each other.** Reported from
eleven screenshots of the installed build: in PSK, `BPSK63` sat on `BPSK31` and
`SAVE` on both. Two layouts, one mistake — the params row was a
`horizontal_wrapped` **nested inside** the header's `horizontal_wrapped`, and the
tail (`SAVE`/`CLEAR RX`/`ENIGMA`/`SQL`) was a `row_tail`, i.e. a right-to-left
layout, also inside it. Both are laid out against the line they start on and then
paint downward, so when the row wrapped, lines two and three landed on line one.

**The rule**: a wrapped row is one flow. Draw into the caller's row, never a
wrapped row of its own, and let a tail flow at the end of it instead of being
pushed to the right edge. Guarded by
`the_mode_parameter_row_does_not_draw_on_itself`, a **collision test on the text
shapes** swept 420–1200 pt. Two things about that test are worth keeping: it needs
**Classic UI on** in the harness (the column is what makes the panel narrow enough
to wrap — without it the first version passed on the broken code), and it must
measure **chips, not captions** (a `ui.label` in a wrapped row reports the rest of
its line as its rect, so "Family" claims 318 pt for a 40 pt word).

**Issue #156 — the readout moved the dial by the mode's tone offset.**
`readout_for_audio` returned one number doing two jobs: `shown - dial`, which an
edit subtracts, and the gap to where the signal is, which the annotation shows.
They coincide for a repeater shift and for CW QRG and **not** for a digital mode,
whose readout *is* the dial. Returning the tone offset as the map-back made every
wheel step and typed digit move the VFO by 1500–1700 Hz, which on a mode working
below the dial is a one-way ratchet — exactly the report. Now two numbers:
`map_back` (zero for a digital mode) and `note`.

**Issue #146 — the LISTEN tab hides the transmit controls, and never grants
them.** `UiSettings::listen_tab_hides_tx` (off by default, asked once) makes
picking LISTEN in the band/mode menu set a flag that `swl_mode()` already ORs in;
OPERATE clears it and the radio's own setting decides again. **The tab is
remembered** (`UiSettings::band_menu_listen`, the operator's call on 2026-10-23),
so a station left listening comes back listening with the transmit controls still
hidden. The rejected reading was "OPERATE = TRANSMIT", which would make a
transmitter one click away; this half cannot loosen anything — a receive-only
station stays receive-only whichever tab is chosen.

## The mode roadmap — what to build next, in order (2026-10-19)

**The queue, cheapest first.** Detail, measurements and the reasoning are in
[`MODE-ROADMAP.md`](MODE-ROADMAP.md); this is the order and nothing else, kept
here because it is the thing to look at when starting a session.

1. ~~**Verify the modes we already ship against a second corpus**~~ — **done
   2026-10-19.** Feld Hell, Hell 80, FM Hell, RTTY 45/50/75 and SYNOP all read
   their recordings. Hell 80 needed a real fix (it transmits 7 rows, not 14);
   the other three "failures" were the test rig tuning to the wrong centre.
2. **`fork/psk-variants`** — symbol rate and constellation as parameters.
   `crates/sdroxide-dsp/src/psk.rs` hardcodes `BAUD = 31.25` and its detector is
   differential BPSK only, so BPSK63/125/250 and all four QPSK speeds cannot be
   tuned to at all. It fixes seven corpus files, and it is the front end of both
   STANAG 4285 and MIL-STD-188-110A — which is why it comes first.
3. **MFSK8/16** — the cleanest *new* mode: fldigi is GPL-3, Shortwave Radiogram
   sends MFSK32 every week, and it is the first run through the "add a mode"
   path (modem, `Mode` variant, `DigiConfig` field).
4. ~~**STANAG 4285**~~ — **built 2026-10-22**: the receiver locks real off-air
   captures (the folded sync 0.85 against a 0.14 null, the reference blocks
   0.64/0.75/0.62), the mode is wired with a panel of measurements, the negative
   controls refuse, and **Annex E is transcribed and built** — the code, the
   interleaver, SOM/EOM and an ITA2 reading, proven on a loopback. What it needs
   now is a **real coded transmission**: the night's captures are uncoded or idle
   and produce no SOM. See [`STANAG4285-HANDOVER.md`](STANAG4285-HANDOVER.md).
5. **One item from the hours table per day** after that: Contestia (after the
   Olivia fix, not before), MT63, DominoEX, RS_ID.

**Not doing, and the reason is a patent rather than the work:** DMR, P25, NXDN,
dPMR — AMBE+2 is covered until 2028-05-20. See the DMR section below.

## Session 2026-10-11, later: **FLATTEN** (waterfall floor), and the RTTY anchor

Two pieces of work, both on `main`, both off a look at **MSHV** (MixW-lineage
Qt program, GPL-3, on the Extras disk) rather than off a report. MSHV turned
out to be **behind** us on both counts — its waterfall is a fixed
`int data_graph_all[880][204]` whose `x_pos` saturates and freezes, coloured
by 20×501 **PNG** palettes sampled with one `pixel()` per screen pixel — so
almost nothing was portable. Two ideas were, and both are now built.

### FLATTEN — per-column noise-floor flattening (shipped, on by default)

`05afaa69`. A receiver's noise is rarely the same everywhere on a span, and
FIT sets **one** floor for the whole picture (a percentile over every visible
bin), so a quiet stretch goes black under a level fitted to the loud half.

Each row's columns are averaged over ~16 (`FLATTEN_SEGMENTS`) — **one column
is one FFT bin, so a floor estimated from a bin is that bin** — the difference
from the row's own mean becomes a lift, and it ramps between segment centres.
Applied to the row **before** it reaches the texture, so no shader, no
texture, no wire change: **no `PROTO_VERSION` bump**, and the only state is
`ViewState::flatten`, local per radio like `auto_fit`.

**The integer offset does not converge, and that was the find.** Held as `u8`,
any step smaller than half a unit disappears into whatever the cast does —
truncating stalls outright, and `round()` is worse than it looks, because
`45 + (51−45)×0.08 = 45.48` rounds back to 45. Every such cast leaves fixed
points a few steps short of the target, so the lift stalls at **45 of the 51
ramp steps** its target asks for: a floor 17.6 dB up where it should be 20,
forever. The state is a `Vec<f32>` and is rounded only where it lands on a
row. The test says so in its failure message and fails against the integer
version — worth re-reading before anyone "simplifies" it back.

Two load-bearing decisions, both pinned: the lift is **never negative** (a
quiet column is raised to the floor, a louder one left byte-for-byte alone —
MSHV's ADLE refuses negative offsets at `display_ms.cpp:1773` for the same
reason), and it is **capped at 20 dB** (without it, spectrum below the
receiver's own noise is pumped up until it looks like signal). The offsets
live in texture-column space, so `move_offsets` carries them through the same
`Shift`/`Rescale` the history is moved by — left behind, the correction slides
off the spectrum on every pan and zoom, which reads as a rendering quirk
rather than as a bug and so survives indefinitely.

**Bench verdict: "it looks way better and smoother" — on both radios**, a
wideband SDR and a demod-audio rig. Hence **on by default**, against the
spec's own recommendation. Looking at it also showed the cost, which the spec
had only argued: the black end of the ramp is gone, so you gain "a weak
signal shows in the quiet part" and lose "this part of the band is dark".

`crates/sdroxide-ui/examples/flatten_probe.rs` draws the same band with and
without it through the real code and the real colour map — **a committed
probe, not scratch**, and it is how the first version was checked. It caught
that its own synthetic "noise" was a travelling sine: the PNG was showing my
generator, not the algorithm.

### RTTY/NAVTEX/DSC: the panadapter anchored 2210 Hz off the station (shipped, unproven)

`c26b7b82`. Thieu — **VERON member, Dutch contest team** — by mail on
2.0.7.1: *"Versie Brown 2071 brengt de afstemming in RTTY. Op hol, lijkt nog
moeilijker te remmen dan 201. Wordt vervolgd"*.

`focus_hz` anchored every mode holding a standard tone pair on the **pair**
(dial + 2210 Hz for RTTY) whatever the radio. Right for a demod-audio source,
where the dial is on a suppressed carrier; **backwards for an SDR**, where
`Mode`'s own doc says RTTY stations are worked *"by moving the dial onto
them"*. At 3 kHz of span the signal is still on screen two thirds off centre;
at the 1 kHz you zoom to in order to read the tones — the span a contest
operator runs — it is **outside the window altogether**. CTR is on by default,
so nobody asked for it.

**This is SSTV's split, applied to the mode next door.** `focus_hz` already
handled SSTV with the argument written out — *"Only a demod-audio source. On
an SDR the operator tunes the dial onto the picture"* — and RTTY never got
it. That is the shape to look for next time: a decision settled for one mode
and never applied to its twin.

**Four tests were pinning the bug**, one by name:
`rtty_anchors_on_the_tone_pair_not_the_dial` asserted the fault in prose
(*"the dial is not even on screen, and anchoring the picture there takes the
signal off the far edge"*). All four rewritten to hold both halves; fail-first
message is *"the window left the station it is tuned to"*. **A test can pin a
bug and read like a specification** — read what a test's *name* claims before
trusting it.

**Checked before editing, not after:** the log reads `Mode::on_air_hz`
directly (`app/mod.rs`) and the transmitter uses it too (`engine.rs`), *not*
`focus_hz`, so a NAVTEX or DSC decode is still recorded on the channel centre
and not 1.7 kHz low. `click_sets_offset` deliberately untouched — a click in
RTTY tunes the dial on every radio. CW's `cw_qrg` has the same shape and is
**left alone on purpose**: it sits behind a named setting rather than being
forced by the mode.

**Two things the diagnosis cost, worth remembering.** `git log` on the RTTY
files between the two tags finds **nothing** — only the zoom-out jitter fix
touched the panadapter, the RTTY click commit `7c8a14c3` predates 2.0.1, and
`on_air_hz`'s LSB sign is correct. So "what did you change" had no answer,
and the reading that resolved it was **grammatical**: *"2071 **brengt**"* is
present tense — he was describing 2.0.7.1's behaviour, not dating a
regression. A reporter's tense is evidence. And his standing — VERON, contest
team — is not colour: he tunes by dial in narrow views in a pile-up, which is
exactly where a 2210 Hz anchor error is worst and where a casual listener
would never notice it.

**Still unproven.** No RTTY was found on 20 m or 40 m in a bench capture, and
the detector had no positive control, so that proves nothing either way. The
open question, asked and unanswered: **dial or click?** If he means clicking,
this may not be his report at all.

### The off-screen offset hint, and a release cut twice

The RTTY entry above left one thing on the table, and it mattered more than
the fix. **Every panadapter marker was drawn inside `if in_view(hz)`** — the
cyan audio-offset cursor and RTTY's amber mark/space pair both — so an offset
that left the window stopped being drawn at all. At 1 kHz of span RTTY showed
a cleanly centred signal and **no trace of an offset**, indistinguishable from
a mode that has none.

That silence is the defect, and it is the one a reporter cannot name.
*"Lijkt nog moeilijker te remmen"* is a judgement about a working
configuration whose only fault is that the program stopped explaining itself.
`offset_edge_hint` pins `◀ tones 1710 Hz` to the edge the offset lies beyond,
with the distance to the **nearer** edge — the smallest honest number, and the
one that says where to zoom out to. Bench verdict on the hint: **"like a
charm"**.

**Drawn with `painter.text`, not `layout_no_wrap` + `galley`.** The first
version drew the rule and silently dropped the words. The panadapter's own
labels, 150 lines away in the same function, use `painter.text`. Reach for
the call that already works next door, not the one that looks better.

**One reporter, three modes — and that is why the visibility work went in
before his answer, not after.** The anchor change widened behaviour for every
SDR user, RTTY and NAVTEX and DSC, off a single report. That is defensible
only because the fault was *proven* and not inferred, and it is the reason
legibility was worth doing on its own: it removes the class rather than the
instance, and it stands whether or not his report turns out to be this.

**2.0.8 shipped; 2.0.9 was cancelled on a stale lock; 2.0.9.1 carries it.**
The `--locked` trap and the point-tag recovery are written up under
**"Cutting a release"** — read that before the next tag, because it is the
check that is seconds to run and was skipped.

## DAB status, 2026-10-11: confirmed on the air (Kevin, 2.0.7.3)

**DAB+ is working end to end on a PlutoSDR at 2.5 Msps: "fort et clair", no
dropouts.** Kevin confirmed it on `2.0.7.3`, on his own Pluto. Three PRs
(2026-10-10) got it there; Kevin has confirmed all three on the air:

| PR | what | how it was proven |
|---|---|---|
| **#38** | faad2 was built with `DRM_SUPPORT`, whose `fill_element` refuses a whole frame (error 30) when a fill element carries anything but SBR. DAB+ encoders pad almost every Access Unit that way, so most frames decoded to nothing: the "chopped audio" on every station and client. `sdroxide-faad2/build.rs` compiles a patched `syntax.c` from `OUT_DIR` that skips the payload, as the non-DRM build does; `vendor/` untouched, the anchor asserted to match once. | Kevin's 8B capture (BFM BUSINESS, 197.648 MHz, 2.5 Msps): 75 AUs refused before, 0 after. `a_frame_padded_with_a_fill_element_decodes` fails on unpatched faad2. On the air: confirmed. |
| **#39** | Block frequencies to the kHz (`197.648`, not `197.6`). | Unit test; on the air: confirmed. The separate "dial reads 197.148" report was never reproduced and is now confirmed fine. |
| **#40** | The S-meter froze in DAB (and ADS-B, AIS, VDL2): no demodulator, so no `Meters` event at all. A wideband lane now measures its own bandwidth off the spectrum (`spectrum_channel_dbfs`, shared with the scanner). | `wideband_lane_meter.rs`, both tests fail on the old code. On the air: confirmed. |

**This closes the long-standing "faad2 refuses most Access Units" item** recorded
under DAB in `ROADMAP.md` (now marked resolved there). It was a build flag,
not bit errors or a codec licence problem, so the fdk-aac route is not needed.

**Open, and the next DAB-adjacent item:** the Pluto does not report its receive
gain to the engine (`IqSource::rx_gain_db` is the default `None`), and its
default AGC is `slow_attack`, where the AD9361 moves the gain on its own. So on a
Pluto the S-meter follows the chip's gain as well as the antenna. Being worked:
read `hardwaregain` back from the control thread.

Also confirmed by Kevin the same day: **#45** (web client: sound seconds behind
the waterfall after a hidden spell). Still untested on the air: **#41** (SSTV
LSB on 60 m) and **#42** (CTR keeping a zero-IF LO half its offset away).

## The anti-touch lock (phone and tablet, 2026-10-10)

A **padlock chip** after SYS on the tablet strips (after ☰ on the phone) locks
the screen against stray fingers: a tap locks, a **3 s hold** unlocks, with a
ring filling round the padlock. It works on the **input**, not on the widgets:
`touch_lock::filter` runs from `raw_input_hook` (both `MultiApp` and
`SdroxideApp`) and drops every press, touch, wheel, pinch and key except a
press on the padlock — so the waterfall, which reads the pointer itself, is
covered too. Session-only, shared by every tab; it lets go by itself if the
padlock stops being drawn (window turned desktop-sized), so the screen can never
be locked with no way out. Desktop tier: no padlock. Guard:
`app::touch_lock_tests` (verified failing with the filter removed: `+ tuned
through the lock`).

**After an unlock the padlock rests for 10 s** (`RELOCK_PAUSE_S`, Kevin's
report 2026-10-12: on phone and tablet the 3 s hold unlocked and the same
finger locked it again before it lifted). The old guard, `spent`, cleared the
moment the press was no longer seen as down, which a phone's browser can do
mid-hold; the pause does not depend on how the press is reported. Guard:
`an_unlock_is_not_undone_by_the_finger_that_made_it`, verified failing on the
old code with `the padlock locked again under the finger`. A tap during the
pause says `UNLOCKED · lock in N s` rather than doing nothing silently.

Its partner is **Wake lock** (Settings → UI, under Simple UI,
`UiSettings::keep_screen_on`, off by default; **in `ClientScreen` since v205**,
at Kevin's request, so it is remembered with the profile on the server rather
than per device): the browser's Screen Wake Lock, re-asked every frame it is wanted and
missing because the browser drops it whenever the page is hidden
(`wake_lock::keep_awake`). Needs https; the row says when it is unavailable,
refused, or (installed program) browser-only. Not driven in a real browser here.

## Session 2026-10-08, later: PRs merged, DAB to the radio tab, eQSL QTH, release resilience

**The open PRs are merged and the worktrees are gone.** #22 (SSTV KEEP DIAL),
#23 (browser `remember` cookie — the wasm build), #24 (ADS-B noise floor);
pr22/pr24 worktrees and their target dirs removed. Versioning quirks fixed in
`348a2f7f` (the register was **v196** after #22, not the v189 the notes had
said; it is **v197** now that eQSL's field landed).

**Kevin's DAB advice moved to the radio tab** (`5bef39e7`): the width warning
left the decoder panel and now sits beside the sample-rate control in yellow,
dismissible; the arithmetic (`dab_window_rate` / `dab_widening_helps`) and the
DAB rate constants live in `sdroxide-types` so the browser client can reach
them — **the first cut called `sdroxide-dsp` from the UI and broke the wasm
build**, which the wasm check caught. `PROTO_VERSION` 195 → 196.

**eQSL QTH nickname (upstream #647) — built, live-verified, offered upstream as
#648.** `NetworkConfig::eqsl_qth_nickname` appended last (`PROTO_VERSION` 196 →
197), a Settings → Uploads → eQSL box, and the nickname sent on the **upload,
the login Test *and* the confirmation download** — eQSL identifies a multi-QTH
account by callsign *and* nickname, so a *correct* password with the wrong or
absent nickname is answered "No such Username/Password found" on the download
too (the fork first shipped only the upload; the operator's Test-button hint
found the other half). Verified live on the operator's own multi-profile
account: **"works flawless"**. Fork `aa939ba4` + `f4b9d84b`; upstream #648 is
the same change, so one drops out when the other lands.

**The release no longer stops on one platform** (`ec010201`): `create release`
runs with `!cancelled()` and the web-dist download is tolerant, so a failed
build job — the macOS-ARM runner has done it three or four times — still
publishes the platforms that built. The run ends red and the asset is visibly
missing. `SDR Oxide Brown nightly 2026-10-08` is out; `2.0.2_brown` is built
and installed locally.

**Upstream is alive again** (a burst on 2026-10-08, 14:1x–14:4x): #600, #601,
#615, #622, #635, #646, and **#640 fixed by the maintainer** — which supersedes
our own **#645** (the same issue). #645 has no comments and is now redundant;
close it rather than re-review it. **#648** (eQSL) is open.

## Session 2026-10-08, night: **#643 FIXED**, and the fork's browser build is broken

**#643 is done, at the cause — commits `d7ebc0ec` (fix + probe + regression
test) and `d3863556` (Olivia note removed), pushed.** The operator reviewed the
running build: **"works perfectly"**.

**What it actually was, because the plan above was still wrong.** Not `max_rect`
vs `available_width`, not `image_panel` reading a stale width: **every Frame in
the SSTV panel budgeted its content box to the whole allocation and forgot its
own stroke.** egui's Frame outer rect = content + inner margins + stroke, so a
frame asking for `size` inside an allocation of `size` grows 2 pt (1 pt stroke
each side) every time — the row landed `0…692` past a dock at 681 in a 960 pt
window. The budgets are now `size − 18` wide (16 pt margins + 2 pt stroke) and
`size − 14` tall (12 + 2), the TX preview frame pays its own chrome separately,
and Olivia's unwrapped header row (≈1000 pt of caveat text) is
`horizontal_wrapped` — which is the *other* half: the mode that overran the
**window** never touched a budget at all.

**Fail-proofed exactly as the house rule asks.** The three budget groups were
flipped back to their old numbers with a targeted script (asserted one match
each, so no silent no-op) →
`Sstv at 960 pt: ink x 0..692 crosses the dock edge at 681` → restored → pass.
The regression test `the_operating_panel_stays_inside_its_column`
(`crates/sdroxide-ui/src/app/mod.rs`) drives Sstv / SstvFm / Rifp / Olivia /
Ft8 at 960 / 1280 / 1920 and checks **both** edges — the window's right and the
dock's left — which is the probe-gap note this section's plan called out (the
probe only reported past the window, so it found Olivia and missed the three
image modes).

**The `SDROXIDE_PANEL_TRACE` instrumentation is stripped** (grep clean across
`sdroxide-ui`, the `trace_panel` helper in `frame.rs` deleted, `chrome.rs` back
to byte-identical). Gate: `cargo check --workspace --all-targets` silent,
`cargo test --release -p sdroxide-ui` — 751 passed, 0 failed.

**The yellow Olivia caveat label is gone** from the mode panel (`d3863556`) —
the operator's word: it isn't needed anymore. Nothing else changed there; the
header wrap stays (it is the #643 fix).

**The fork's own PRs, checked — and #23 covers a real break.** Both are
`claude/*` branches, opened today against `madmedicnl/sdroxide-brown`:

- **#23 (OPEN) "Browser sign-in: send `remember`, not hours" — MERGEABLE, and
  its claim is verified true here.** `main` **does not compile for the wasm
  target**: `login.rs:261 expected u32, found bool` — `96817d2` made
  `remember_cookie` a `bool` and left the browser-only call passing it as
  hours. Native builds never compile that line. Run
  `cargo check --release --target wasm32-unknown-unknown -p sdroxide-ui`
  whenever `login.rs` or any wasm-only path is touched — the full workspace
  check cannot see it, and it is otherwise only caught by the tag/nightly CI's
  web job. **Recommend merging #23** (decision is the operator's).
- **#22 "SSTV: KEEP DIAL" — MERGED** (`153463f8`, 2026-10-08) at the
  operator's word after a review, not a test run: `DigiConfig::sstv_keep_dial`
  appended last, `PROTO_VERSION` 195 → 196. Its own body had said the **engine
  tests were not run** (that session could not build `rade_c`); the review
  cleared the wire, the engine placement, the chip and the test's shape, and
  the operator accepted it on that basis. If a report ever comes back from it,
  the untested half is the runtime of `crates/sdroxide-radio/tests/sstv_keep_dial.rs`.

**Carried forward:** #23 and #24 are merged; Kevin's DAB-advice → radio-tab
move is **done** (`5bef39e7`, 2026-10-08). The open items from #16/#18 stand —
the three-tab flicker, the Pluto CPU thread, and the still-untested runtime of
the merged PRs (`sstv_keep_dial`, the ADS-B floor on a real Pluto); the phone's
right border is **delegated, off our list** (2026-10-09);
`cargo check --release --target wasm32-unknown-unknown -p sdroxide-ui` is now
worth a slot in the pre-push routine whenever wasm-reachable code changes.

## Session 2026-10-08, evening: the issue list, Kevin's #18, and #643 carried forward

Worked the fork's issue list, then Kevin's **#18** ("the program nags").
**Ended with #643 diagnosed but not fixed — that is the one thing carried
forward.**

**Shipped, on `main`, installed as 2.0.2_brown:**

| commit | what |
|---|---|
| `76817a2f`…`19085f2f` | sign-in cookie + REMEMBER ME box (no 12 h / 1-day choice) |
| `e4cc57c9` | bindings "stop asking" now stops asking; DAB width warning no longer advises the impossible |
| `1eae73ab` | DAB audio queue priming |
| `19085f2f` | DAB CLEAR chip; private windows documented |
| `fb3a460f`, `78876a01` | dismissible advisories (✕, remembered) on DAB, AIS, ADS-B, VDL2 |

**Upstream PR #645 is open and MERGEABLE** — the #640 audio/panadapter fix, two
files, +85/−22, tested failing against the unfixed code.

**`main` is protected: PR required, 1 approval, admins not enforced** —
deliberate, because the operator pushes to `main` directly and enforcement
would block him. Do not "fix" this.

**Kevin's #18, honestly.** Five of his six items are fixed. Two of them were
**wrong warnings, not excess warnings** — one contradicted a setting he had
already changed, one had a "stop asking" button that did not stop it. Fixed at
source, not muted. **Done (`5bef39e7`, 2026-10-08): his own proposal — moving
the DAB advice into the radio tab.** It now sits beside the sample-rate control
in yellow and dismisses for good once read; the panel copy is gone. The blocker
was never real: `sdroxide-ui` already depends on `sdroxide-dsp`. The arithmetic
(`dab_window_rate` / `dab_widening_helps`) is shared in `sdroxide-dsp` and the
constants in `sdroxide-types`, so the engine's warning and the radio tab cannot
drift. #23 and #24 are also merged.
Replies posted to #18 (twice) and #17, each saying what is in tonight's nightly.

**#643 — the carried-over item, and next session's first job.** Reproduced and
SSTV-specific; Olivia, SSTV, SSTV-FM, RIFP, in both LISTEN and OPERATE
(operator-confirmed). The full measurements are in the #643 section below —
Olivia is a *different and larger* fault (at 960 it paints 0…1770, 810 pt past
the window, so its width comes from a floor that binds only when the column is
small). Read that section for the numbers; what follows is the plan:

1. **Instrument `operating_panel`'s calls to `image_panel` (`frame.rs:2368`)
   and `text_modem_panel`**, printing each panel's `max_rect` beside the
   `allocate_ui` that made it. Two lines, one run — it names the bug or
   eliminates both panels.
2. **Fix it.** `digi_panel` does not overrun, so the difference is in how
   those two are called.
3. **Prove it:**
   `MODE=Olivia DOCK_W=960 cargo test -p sdroxide-ui --release --lib probe_narrow -- --nocapture`.

   **Fix the probe's gap first**: it reports rects past the *window* edge, so
   it finds Olivia and misses the three image modes, which stay inside the
   window and cover the *dock*. The regression test must check **both** edges
   or it guards half of this.

**Do not fix this with a clamp at the panel boundary.** Argued and rejected
twice: it hides the cause and it would only catch the window case.

**Corrections made this session — three times a measurement disagreed with a
report and the measurement was wrong:**

- Claimed #643 was upstream's build from the window title — the fork hardcodes
  `sdroxide — {label}`.
- Claimed it again from the SSTV save path — that is a persisted setting, not
  the live `config_dir()`.
- Called the overlap "constant 39 pt" — it was the 1920 row of a table from a
  sweep that only ran at one width.

Also: **`git checkout --` on a file being edited destroyed a whole fix** and it
had to be redone — undo a scratch patch with the patch, not with checkout. And
**two python insertions into `chrome.rs` silently did nothing** because they
anchored on a string from a different file: no error, no change. Verify the
edit landed, always.

**Operator preference, standing: shorter replies.** Polite, personal, direct.
No essays, no over-explaining, no restating the reasoning.

**Do not** ship a fix to a tester's report without it being **verified failing
against the unfixed code**. Never tell Kevin something is in a build when it
isn't — say what is and what isn't.

**Housekeeping:** tree clean, `main` pushed, 2.0.2_brown installed at
`~/.cargo/bin/sdroxide`, workspace check silent, ~130 commits pushed.

## Session 2026-10-07, end of day: **2.0.2_brown**, and a report that landed

**`v2.0.2_brown` is tagged, pushed and installing** — `Cargo.toml` at `2.0.2`,
the binary built and installed locally, release run `37664359497`. **It is not
numbered 2.0.1 because `v2.0.1_brown` already exists** as a pre-release, and the
house rule is that a tag is never re-cut: the cookie and the "NOT THIS TIME"
answer are both *after* that tag, so they need a version of their own. Marked a
pre-release, because it is the one built to be tested rather than announced.

**Kevin's second comment is the most important field report this fork has had,
and it is against us.** The whole of it, in order:

1. **A sign-in per radio, confirmed as ours and as universal.** *"Each new
   session requires entering a username and password, and re-entering them when
   switching from radio 1 to radio 2… This happens to everyone, so it means you
   don't test the versions you produce yourself. That's a shame; you'd save time
   and effort."* That last sentence is **correct** and is recorded here as the
   lesson rather than as an argument to answer: the client half of sign-in has
   **never been driven in a browser against a passworded multi-radio station**,
   and the tests that exist exercise [`LoginForm`]'s logic — a unit test on a
   form is not a test of a sign-in. The cookie removes the dependency on the
   broken path, but it does not close the gap, and the gap is now named.

   **Scale it, though.** It was a bug, we got it wrong, we said so in his own
   thread and fixed it in the next build. Nobody was hurt and nobody lost
   anything but a few extra sighs, and the note above is here because the
   *shape* of it is worth remembering — not as a case against us. Do not let a
   lesson harden into a grievance: he volunteered the testing, and the reply he
   got was a fix, not an excuse.
2. **He answered the question the operator's bet would have got wrong.** His own
   `/radios` output lists three paths on one host, so it is one station, not
   three — and the operator's recollection of "three machines" was not what his
   own report said. Check the artefact before acting on a memory.
3. **He declined the flicker question**, twice over: *"I wouldn't go to someone's
   house to see what's displayed on their phone."* Fair, and a standing rule
   from here: **do not ask a phone user for phone-only evidence.** A desktop
   browser reproduction is the only route, or no route.
4. **The Pluto/ADS-B thread at 99.9 % predates every version.** His words: *"All
   versions of Oxide have been affected by this CPU bug since the beginning."*
   Ours to measure before it is the firmware's — and he should not be sent off
   to chase it. He offered the Tezuka 0.3.23 firmware; take the before/after
   numbers if they arrive, nothing more.
5. **FT8 stopped decoding on his machine and he blamed his own overlapped
   installs** — probably right, and a clean install is the test.

**What he was told**, in discussion #16 (comment `18799241`): the table of what
one cookie buys and does not buy, that 2.0.2 is the pre-release to test, that
**fewer updates are coming** while the operator tests the browser by hand, and
that he is not being sent to investigate the Pluto thread. The credit is in the
**README's acknowledgements** as well as the thread — three stations of his own,
a full checklist in one pass, and a report that separated fixed from broken
instead of rounding it up.

**The standing shape for the next few days: the operator is testing, not
building.** He is setting a station up and has the coming stretch free, so new
work is manual verification of what is already in the tree rather than new
features — and Kevin has been told to expect less news, which is the honest way
to keep a tester who has already done us a great deal.

**And the conditions were constrained on both sides, which the record should not
hide.** Two radios on the bench and no Pluto, no browser to drive, one pair of
hands on this side, and the operator's week shared with treatment — under which
he has not been able to listen properly or get a single FT8 contact through for
over a week, and still a great deal got built and shipped. What came out of that
is a shared result, not a favour granted to the project: his calls on the licence
question, on keeping the radio simple, and on saying no to things that only
looked urgent are the reason any of it is shippable. **Gratitude runs both ways
here, and the ledger above is not a list of what we owe.**

---

## The open queue, as of 2026-10-07 (v2.0.2_brown shipped)

The whole of it, in the order a session should pick it up. **Housekeeping, done
this evening: working tree clean, `main` in step with `origin`, no untracked
files** (so no scratch probe parked in an `examples/` folder, which `--workspace`
would build), installed binary 2.0.2_brown matching `Cargo.toml`, and
`v2.0.2_brown` a pre-release with `v2.0.0_brown` correctly still "latest".

### Ours, and do these next

1. **Browser-verify the 2.0.x browser fixes.** The highest-value item on the page
   and the one that closes the gap Kevin's report proved: the **Copy diagnostic**
   download, the **share picture** gate, the **sign-in cookie** (12 h / 1 day /
   not this time) and the **scrolling radio strip**. All four are unit-tested and
   none has been seen in a browser. Needs a passworded multi-radio server and the
   operator's own browser — nothing else is worth doing first.
2. **The ATS Mini's documented serial control surface.** Research is finished: the
   command reference is read, the units and the command set are known, and the
   licence boundary is settled. Next is a sketch of the client (an ATS-style
   source over serial or TCP, no new `Backend`, no `PROTO_VERSION` bump) plus the
   RadioScript meter script for the bench. **Not started, and nothing should be
   until the operator picks up the board.**
3. **DAB at 3.1 Msps on the RSP1.** Our own warning says the front end is at its
   floor at 2.5 Msps. We have a wideband receiver on the bench: either it confirms
   the warning or it finds something, and it is an hour.

### Open, and waiting on a reply

4. **#4 bindings** — offered two fixes (wording on the CONTROLS page, or the
   default for his own profile); awaiting his choice.
5. **#5 auto-clear the decodes on a frequency change** — offered as an
   off-by-default setting, because clearing them by default loses decodes; awaiting
   whether he wants it automatic at all.
6. **#6 "none" versus "All"** — asked which of the three `ALL` chips he meant
   rather than renaming the wrong one; awaiting a screenshot or a panel name.
7. **#10 recorder replay** — asked which mode he wants first. The missing piece is
   an audio-file source that reads the UTC/frequency/mode stamp out of the
   filename; the naming is already there.
8. **#13 armv7** — built and attached to `v2.0.1_brown`; awaiting ipaddr42's
   report from his own hardware, and only then fold it into the release matrix.
9. **The three-radio tab-close flicker** — still needs his answer to one question
   (does the tab *count* flap, or only the screen flash). **It is also testable
   without hardware**: the `MultiApp` harness with three tabs and a stepping clock
   exists, and the three-tab test was never written. That test is the guard for
   whichever cause it turns out to be.

### Parked deliberately, with the reason

10. ~~**The 2 pt page overflow**~~ **FIXED (2026-10-07), and it was never the
    chip row** — see the item 1b section below for the whole thing. The ink that
    went off-screen was the top bar's *background*, painted by egui's `Panel`
    frame fill 2 pt past the window's edge at every size; the band is now painted
    from inside the panel with a painter clipped to the window. The sweep is
    strict at half a point and the ignored strict test is **un-ignored and
    passing**.
11. **The empty station-key guard** — real, masked by the sign-in cookie, harmless
    while masked. Pick it up when something actually depends on it.
12. **Olivia transmit interop** — needs somebody with an fldigi on the other end.
    It is the single most useful observation anyone could send us, and no amount
    of reading our own code substitutes for it.
13. **The Pluto / ADS-B 100 % CPU** — real and predates every version, and outside
    what our bench can reproduce. Open to anybody who has a Pluto.

### On the month list, unscheduled

14. **DAB's MOT slideshow and DLS** — wants its own message pair rather than a
    field, and a capture of an ensemble that actually broadcasts one.
15. **DMR** — **decided against (2026-10-11)**, and the reason is a patent rather
    than the work. Read **"DMR: scoped to the codec, and decided against"** in
    this file before answering anyone who asks again — including the two who
    have already asked twice. Nothing to build, nothing to promise.
16. **FST4W** — the verdict is recorded and it is **do not ship as wired**; see
    `FST4W-HANDOVER.md` for the diagnosis and `fork/fst4w-wiring`.

### Branches worth knowing about, so nobody re-derives them

**Pruned 2026-10-09** (local and `origin`): the merged `ci/*`, `fork/*` and
`fix/*` branches, the abandoned experiments `fork/esp-sdr-protocol` and
`fork/fst4w-wiring`, `local/agents-notes`, the `claude/*` branches whose PRs are
merged, and every `upstream-pr/*` branch whose PR is closed. The work is in
`main` or in the PRs on GitHub; nothing was lost. Two are worth a pointer anyway:
- **ESP-SDR IQ-over-Wi-Fi** (fork discussion **#11**) — an unmerged experiment,
  and a *different device class from the ATS Mini* (it has I/Q; the Mini has
  none). The branch is gone; the idea is not re-derivable from the tree.
- **FST4W** — the do-not-ship verdict and its diagnosis are in
  `FST4W-HANDOVER.md`; the branch is gone.
- `upstream-pr/*` — **only the branches backing an OPEN upstream PR are kept**
  (#648, #626, #598, #597, #559, #557, #554, #545, #537). Delete a branch when
  its PR closes; do not keep dead ones "on purpose". Do not rebase one unless
  the maintainer asks.


**Upstream #640 is fixed here, and our fork had all four of its faults.** ct7cht
reported and diagnosed it on a TS-440 with a DigiRig (CAT + RX audio) and an
RTL-SDR on the 45 MHz IF as a panadapter, and sent a patch that works on their
hardware. Every part was present in our engine:

1. **`finish_audio` sat behind `self.main.is_some()`** in both paths, with its own
   unconditional `return` when there was no main to take an `out_rate` from. But
   **no main *is* the panadapter configuration** — the attached receiver paints
   the picture and the transceiver supplies the audio — so the one function that
   had to run was the one skipped, and FT8 and CW decoded silence. The main chain
   is now the only conditional part, and `out_rate` falls back to the engine's
   configured `audio_out_rate`.
2. **The channel analyzer was created for a digital mode with no main to feed it
   from** (`main.channel_iq()`), and frame selection prefers it — so FT8 showed a
   seeded, frozen 3.7 kHz window with the live wide panadapter sitting behind it.
   `want_channel` now requires `self.main.is_some()`.
3. `run_audio`'s early return, and the pooled path's gate — the same fault as (1)
   seen from the two callers.

**The lesson worth more than the fix: the test was asserting the broken
behaviour.** `crates/sdroxide-radio/tests/cw_panadapter.rs` built its engine with
`EngineConfig::default()`, which has **no audio — and no audio means no `main`
chain at all** (`Engine::new`'s `None => (None, None, …)`). So the entire file
had been running #640's configuration while its doc claimed to test an ordinary
station, and `the_digital_modes_keep_their_channel_view` was passing on a frame
that can only be *frozen*. The assertion was right and the harness was wrong. It
now builds a real chain (an unread ring, the `skim_window` idiom) and the no-main
case got its own test, **verified to fail against the unfixed code** — reporting
`3700 Hz`, which is the number to remember.

**So: a harness that takes every default is not testing the program, it is
testing the defaults.** An `EngineConfig::default()` reads like "nothing
special", and it is in fact one of the two configurations this engine has
(`audio: Some` / `audio: None`), with materially different behaviour — including
whether the panadapter works at all. When a test's subject is a *station*, build
the station.

## The phone panadapter: the waterfall-only default was about the width

Kevin's #9 point 3 (*"the scope display is still missing in phone mode"*) and
point 2 (landscape) are the same fault. `Tier::Phone` decided the panadapter's
layers, and **`tier_for` is right that a phone in landscape is a phone**
(`852x393`, with a test saying so) — so the tier cannot tell the two apart, while
the reason for the waterfall-only default only holds for one of them: *"a
spectrum trace in a 360 pt-wide window…"*. A phone held sideways is **852 pt
wide** and lost its spectrum for a reason written about a window half its size.

`layout::panadapter_waterfall_only(ctx)` is now `tier == Phone && width < 600`,
read from the window so a rotation changes it. Its pure half,
`panadapter_waterfall_only_for(tier, width)`, is tested beside `tier_for` and was
**verified to fail against the tier-only version**.

**And the phone's layer switches did not do nothing — they were unreachable.**
`SHOW WATERFALL` was drawn inside `picks_layers`, which is false on exactly the
tier that needs it, and the panadapter forced `frac = 0.0` on top. The default is
now a *default*: hiding the waterfall is the operator asking for the spectrum —
the trade the SPEC popup offers everywhere else — and that is how a phone reaches
the scope. Showing **both** at once on a narrow window is deliberately not done:
it needs the phone's default to be persisted rather than derived, which is the
landscape rework and wants a screen to look at.

## Two smaller ones from the same report

- **The browser evicts the site's storage and the client keeps its screen there**
  (Kevin's #9 point 7: sleep the phone, get the defaults back, with no password
  prompt — because the login is in the same store read on a different path).
  `navigator.storage.persist()` is now asked for at startup (`crates/sdroxide-web`).
  **This is the half that could have been prevented**: the same eviction had
  already cost this operator his control bindings, and the answer then was "save
  the screen to the profile" — a workaround for a problem we were causing. Both
  stay: the request where the browser grants it, the profile where it refuses.
- **A phone menu chip that opens a window left the menu stacked over it**, because
  the popup closes on a click *outside* and a chip is not outside. `MEM`, `GRID`,
  `⚙ SETTINGS` and `? HELP` all do it. Closed from the group rather than by
  switching the popup to `CloseOnClick`, because the same menu carries the band
  and mode chips, which are picked several in a row.

## Session 2026-10-06: the notes were a release behind, and Olivia is settled

**Two corrections to this file, both found by running the standing routine
rather than by reading it.** The queue section claimed "nineteen open, all
`CLEAN`"; the discussions section named three threads where there are thirteen;
and the newest release documented was 1.9.18. All three are now fixed above.

**`v1.9.19_brown` is tagged, pushed and released — CI green, one run, no
re-tag.** `Cargo.toml` is at `1.9.19`, the tag is on `origin`, and the release
workflow completed `success`. It carries the **SSTV Re-upload chip** (discussion
**#7**: load a received picture into the transmit slot and send it back out for
the stations who could not copy it — "a card is what a broadcaster sends back",
in the same spirit as dropping the QSL card below). That is the functional
half; the operational half matters more and is the trap the queue section now
names: **two users are reporting bugs as "persistant on 1.9.18" that 1.9.19
fixes**, and neither can be told "fixed" until we know which build they run.

**The Olivia polarity question is closed**, and not by us. See §3 — Kevin sent
a capture whose text MultiPSK had already shown, and our decoder reads it on his
own radio. The open item that remains is the *opposite* direction: whether a real
fldigi can copy *our* transmission, which is the sync-tones work and is
untouched.

**One genuine bug fixed upstream this week, and the fork's note about it was
wrong.** `decode_snapshot` on the 0.13 base was recorded here as "a pre-existing
fixture mismatch on this box, not ours". It was a real portability bug —
fixtures pinning `f32::to_bits()` make the last bit of a float a property of
the compiler and the platform libm. Reported as **#579**, fixed by **#581**
(`f6323dca`) after jl1nie measured several architectures himself. The full
entry is in the subtract section; the lesson is the one worth keeping.

## Session 2026-10-04, later: 1.9.18_brown, and what the release gate is for

`v1.9.18_brown` is tagged and pushed (run `37238357892`). Three functional
changes since `v1.9.17_brown`, `PROTO_VERSION` unchanged at **192**:

1. **The phone layout** (`e8e7812a`) — three chips on the row (RX, DISP, ☰) and
   one nested menu behind the ☰, grouped BAND / MODE / SYSTEM / DECODE WINDOWS /
   EXTRAS. Phone tier only; desktop and tablet are byte-for-byte unchanged.
   Fork discussion **#9**, and the base layout fault is shared with upstream
   (#516), which is why the fork answers it with a responsive menu rather than
   upstream's fixed-width chips.
2. **The Enigma machine and its crib solver** (`9099375f`, `021f6cce`) — the new
   fork-only crate `sdroxide-enigma`, and an **ENIGMA** chip in the free-text
   keyboard panels (`panels/text_modem.rs`, twice: the mode's own panel).
3. **The SSTV SAVE chip** (`d9425662`) — asks for the picture's *bytes*, not
   its texture, so it no longer waits for a session restart (discussion #7).

**The 1.9.17 phone *crash* is NOT fixed, and must never be described as such.**
The two new regression tests (`phone_crash_regression_360x800`,
`…_1440x3200`, `app/mod.rs`) drive a whole frame off-screen at both reported
geometries and **pass on current code**; the crash has never been seen on the
bench. So the release notes claim the *layout* fault is addressed and say
plainly that the crash is still open, asking for the browser console output if
it recurs. **That is the general shape for a half-fixed report: separate what is
fixed from what is not, in the notes themselves, or the next reader inherits a
claim nobody verified.**

### The gate found three warnings, and one of them was a test that never ran

The house rule is a silent `cargo check --workspace --all-targets`, and the
unreleased work broke it three ways:

- **`step_trace_is_exact`** in `sdroxide-enigma/src/machine.rs` was an **empty
  function with no `#[test]`**, left over from an earlier draft of the
  double-step test. It never ran, nothing called it, and it is exactly what a
  `dead_code` warning is for. The real trace is `the_double_step_is_modelled`,
  so the Enigma commit's claim ("the double-step trace") was true and the stub
  was still junk — **the two can both be true, and only the compiler sees it.**
- a needless `mut` in `a_letter_never_enciphers_to_itself` (`clone()` is called
  on the binding, `encipher` on the copy).
- **`P_MENU`** in `top_bar.rs`'s phone-strip tests: dead the moment the phone
  tier went from six chips to three. Its measurements were folded into
  `P_MENU_PHONE` rather than dropped with it, since the numbers are what a
  future chip has to beat.

**Run the gate before the tag, not after.** All three were invisible to
`cargo build`, and two of them lived in a commit whose message claimed its tests
were complete.

### Two new rules the gate taught

- **`cargo test --workspace` builds `examples/`, so scratch WIP cannot live
  there.** The untracked `crates/sdroxide-digi/examples/fst4w_probe.rs` was
  written against an FST4W API the vendored `mfsk-core` does not have
  (`fec::ldpc240_74`, `message_to_tones_fst4w`, `Fst4W120` — see
  `FST4W-HANDOVER.md`), so it failed the whole workspace build with five
  unresolved-import errors and the release gate could not run at all. It is at
  `/tmp/opencode/fst4w-scratch/fst4w_probe.rs`. **An untracked file in
  `examples/` is not free** — `cargo test`, `cargo build --examples` and
  `--all-targets` all read it, and it is invisible to `git status` as a change.
  The shape to prefer is a committed probe that is **meant** to be there — one
  that writes a sample of something the program draws, so the drawing can be
  looked at. (`crates/sdroxide-ui/examples/qsl_card_probe.rs` was exactly that,
  until the QSL card it drew was deleted on 2026-10-05 — see the note below for
  the rule it was kept for.)
- **A test that touches the machine it runs on is not a test.** `open_external`
  (the desktop's link/URL opener, added 2026-10-05 after eframe turned out to
  implement `open_url` **only in its web target**) was unit-tested by calling it
  with a `mailto:` URL — which shelled out to the real `xdg-open`, so
  **`cargo test` opened a mail window on the operator's desktop** in the middle of
  a run. The guard is now split out as `external_url_ok`, which decides the same
  thing and launches nothing, and that is what the test pins; the launching half
  is what pressing the button is for. **When a function's whole job is a
  side effect on the world, split the decision from the effect and test the
  decision** — the effect cannot be asserted without doing it.
- **Rendering an image and *looking at it* finds what tests do not.** A QSL card
  built on 2026-10-05 passed all nine of its unit tests while **three typed fields
  were missing from the output**, and the callsign down the strip read `44DC`
  instead of `19DCG044`. Both were layout, neither was arithmetic, and no
  assertion about buffer sizes, colours or encoding could see them: the tests
  asked *whether a card came out*, never *where the ink landed*. Only rendering
  it and looking found either. The card itself was **deleted the same evening**
  (a QSL card is what a broadcaster sends *back*, not something we compose — see
  the board), so there is no code guarding this now; the rule is what is kept.
  The general shape: **for anything drawn rather than computed, render it and
  look at it**, and when tests do guard it, count dark-pixel **bands** in the
  output rather than asserting on a buffer.
- **A new fork-only crate must be formatted when it is added.**
  `sdroxide-enigma` was committed unformatted (both `lib.rs` and a 100-line
  `ROTORS` table). The 2026-09-29 sweep made "every fork-only file is clean"
  true, and that only stays true if a new crate is swept on arrival — which is
  free, because a fork-only file can never conflict with an upstream merge.

**Numbers for the day:** full suite `cargo test --release --workspace` — 247
targets, **5363 tests, 0 failures**, no code warnings. The wasm check
(`cargo check --release --target wasm32-unknown-unknown -p sdroxide-ui`)
passes and carries 273 pre-existing warnings (274 before this release's
cleanup — measure before blaming a change). CI: the `web client (wasm)` job is
first and green, which is the one that would have caught browser-only breakage
in the phone work.

## Session 2026-10-02: three releases, one rustc regression, and Olivia

**Read this first on any release day.** Three releases in a row failed for
reasons that had nothing to do with each other, and each took a full diagnosis.

### 1. Rust 1.99.0 cannot compile `mfsk-core` for aarch64 (the big one)

`v1.9.11`, `v1.9.12` and `v1.9.13` all died the same way: every aarch64 job
compiled the tree in ~9 minutes and then wrote **nothing at all** until it was
killed — so, since `create release` has `needs: [web, build]`, nothing shipped
and `/releases/latest` stayed on **v1.9.10**. 1.9.11 and 1.9.12 therefore have
tags but **no GitHub Release**, ever.

The cause was not where anyone thought. The earlier note here said the aarch64
jobs "stalled silently at the final binary link"; they did not. A probe
(step-level deadline on the build, a memory sampler beside it, and a diagnostics
step that survives the deadline — GitHub will not serve a running job's log, so
a stall is otherwise only visible as *silence*) found:

```
PID 21787  ELAPSED 2073s  RSS 416MB  STAT Sl  %CPU 100  rustc
cmdline: rustc --crate-name mfsk_core --edition=2024 …
Threads: 6   wchan: futex_do_wait   VmSwap: 0
mem_used=1606MB avail=14340MB swap_used=0
```

One rustc invocation, one thread pegged, **flat** memory, no OOM, clean `dmesg`,
14 GB free. An LLVM codegen pathology on the **FT8/FT4 decoder**, not the
linker, not our binary, not memory. And the toolchain was unpinned: rustup moved
to 1.99.0 on **2026-10-01**, the day 1.9.11 was tagged, and `v1.9.10` (the last
release with ARM artifacts) built the same crate in seconds on 1.98.1.

**Fixed by pinning** `dtolnay/rust-toolchain@1.98.1` in all four CI steps
(`release.yml` twice, `windows-msi.yml` twice) — verified: both aarch64 jobs then
built in **9 min 42 s**. x86_64 was never affected, which is why eight jobs
succeeded while six hung and nobody could see a pattern.

**Two consequences to remember.**
- Pinning **exposed a second latent bug**: the Windows job sets
  `CARGO_BUILD_TARGET=x86_64-pc-windows-gnu` and never installed that target —
  it inherited it from `@stable`. With the pin it failed at the first crate
  (`can't find crate for core`). Now an explicit `rustup target add` step, in
  both workflows.
- Pinning also **duplicated #572's helpers**: `cat/src/lib.rs` auto-merged and
  took `link_configured` and `effective_cw_keying` twice. Auto-merging cleanly is
  not evidence of correctness — check the committed tree compiles.

### 2. The release job never checked the repository out

`create release` downloaded artifacts and nothing else, so the `awk` that reads
the curated `CHANGELOG.md` entry died with `cannot open file CHANGELOG.md` and
the job exited 2 — **after** all fourteen builds succeeded and their assets were
uploaded. Latent since the notes were switched to the changelog (after v1.9.11);
v1.9.13 was the first release to run that line at all. Fixed with a checkout of
`${{ github.ref }}`.

**`gh run rerun --failed` cannot recover a workflow bug**: a re-run executes the
workflow *as it stood at the tag*, so a fix has to ship under a new tag.

### 3. Olivia: receive FIXED and proven on the air; transmit unproven

**Read this before touching Olivia.** It was "not the Olivia protocol" and the
**receive** path is now the real thing, proven on a listener's own radio.
`sdroxide-dsp/src/olivia.rs` is still upstream's file (`4ab061fe`), but the
fork has since rewritten the transform, the scrambler, the interleave **and the
transmitter**. `5464e7a9` on `main`
(merged from the now-deleted `fork/olivia`) is the transmit half; `cd668ab5` is
the receive half. Background, because the diagnosis is the useful part:

Discussion #5: a user cannot decode Olivia; MultiPSK copies the same signal on
the same dial. It was not sensitivity and not tuning — upstream's own module
doc had said so since it was written: *"the scrambler constants are not yet
bit-matched to fldigi"*.

Against the ARRL description of the mode, the gaps are:

| | Olivia | `olivia.rs` |
|---|---|---|
| Scrambler | Walsh vectors scrambled with `0xE257E6D0291574EC`, each character rotated right 13×n bits | a per-position **tone** rotation from an xorshift seeded `0x1D5` |
| Bit interleaving | 1st bit of 1st symbol, 2nd bit of 2nd symbol, … | none |
| Sync tones | every transmission is bracketed by them | no concept of them |

The 64-symbol block, the (64,7) Walsh mapping, the Gray tone assignment and the
spacing = symbol-rate rule were always correct, so the work was bounded to the
scrambler, the interleaver, the transform and the sync tones. Its loopback tests
passed because they tested it against itself — which is how it shipped looking
ready, and they stayed ignored for exactly that reason until the transmit half
was rewritten. **Raised upstream as
[#621](https://github.com/dividebysandwich/sdroxide/issues/621)** — an issue,
not a PR, per the standing rule that only a genuine upstream bug goes up and then
as an issue. **Do not re-open that reasoning**: the last line of this entry is
wrong in a way worth keeping.

**Partly done (`7135e19b`), and the reference is found.** The authoritative
implementation is **`src/include/jalocha/pj_mfsk.h` in `w1hkj/fldigi`** (the
author's own mirror), reachable raw — GitLab is Cloudflare-blocked and
SourceForge serves HTML, but `raw.githubusercontent.com/w1hkj/fldigi/master/...`
works, and Debian ships the whole tarball at
`deb.debian.org/debian/pool/main/f/fldigi/`. It settles every convention:

- `SymbolsPerBlock = 2^(BitsPerCharacter-1)` = **64**, carrying `log2(tones)`
  characters. Ours was already right.
- `EncodeCharacter`: a **delta** at `Char mod 64`, sign from bit 6, then the
  **inverse** Walsh transform. Ours (`hadamard_bit(byte & 63, i)`) already matched.
- `ScrambleFHT(c * 13)`: flip the **sign** where bit `(13·c + i) & 63` of
  `0xE257E6D0291574EC` is set — a Walsh-domain sign flip, **not** a tone rotation.
- Interleave: character `c` occupies bit `(c + i) mod log2(tones)` of symbol `i`.
- Receiver: forward `FHT`, peak position, `+64` when the peak is negative.

All of those conventions were adopted, and then a fourth bug turned up that
nobody had looked for: **the transform's butterfly.** Upstream's `fwht` used the
textbook `(b1+b2, b1-b2)`, fldigi uses `(b2+b1, b2-b1)`. They differ by a
**per-row sign**, and in Olivia **a sign is bit 6 of the character** — so every
lowercase letter and the idle character decoded as its bit-6-cleared twin
(`u`(117)→`5`(53), `h`(104)→`(`(40)), while uppercase and space were fine.
That is why the old output showed fragments like `CQ`, `ET`, `LEE` and never the
whole message. `olivia::fht`/`ifht` now sit in `olivia.rs` and `mfsk::hadamard_bit`
is gone.

**The one thing to carry forward: the polarity is deliberately the negative of
fldigi's source, and it is settled by a recording, not by the source.**

- fldigi's `EncodeBlock` sets the symbol bit where its codeword is **negative**,
  and fldigi's `SoftDecode` votes **negative** for a set bit. Those two are an
  exact pair, and a standalone copy of its loops round-trips under them.
- **Our receiver votes positive, and it is the one that reads a real Avalon SW
  Net recording to its known text**: `CQ SouthWest NET … de G7LEE G7LEE G7LEE`.

The conventions are exact negatives, so they cannot both be the air. A C++ dumper
of fldigi's own code confirmed `f < 0` is **byte-for-byte** its `OutputBlock` and
`f > 0` is the exact complement. `loopback_32_1000`, `loopback_8_250` and
`every_character_reads_back_unchanged` all fail on the first and pass on the
second, and the off-air decode is identical either way — the recording decides,
and the receiver already agrees with it. **A previous entry here blamed the
scramble order or the bit placement. That was wrong, and reading the reference
harder would never have found it: the answer was in the one artifact only an
air recording could supply.**

**Still not done, and the next thing to do: nothing has ever been transmitted to
another station.** The open items, in the order they will bite:

1. ~~**No sync tones.**~~ **START TONES NOW SENT (2026-10-06); the tail is the
   part still missing.** fldigi brackets every transmission with a pair of tones
   at the band edges and gates them on `olivia_start_tones`, **default true**, so
   a default fldigi transmission carries them and a decoder may be looking for
   exactly that. `fldigi::send_tones()` is reproduced: 8192 samples in four
   quarters of 2048, alternating low/high at `centre ∓ bandwidth/2` — half a
   tone spacing **outside** the outermost data tone, so the pair brackets the
   bank — each quarter ramped over 256 samples at both ends, phase zeroed at the
   burst start and carried across quarters. One idle character follows, pushed
   with **no** source index so it is never counted as sent text. Length comes
   from fldigi's own `SCBLOCKSIZE` (512) and is **not** a function of the sample
   rate. `Geom::edge_hz` is the only place the placement is decided.
   **The tail is a separate job**: the caller stops asking for audio the moment
   `sent_chars` reaches `total_chars`, so tones written into the block buffer
   would never be drained — it needs a change to the transmit-active contract in
   `text_modem.rs`, deliberately not smuggled in with the start tones. Full
   geometry, the exact source lines and the five tests (each verified to fail
   against the broken version) are in `OLIVIA-HANDOVER.md`.

   **And the reason this was on the list was wrong, which matters more than the
   feature.** The note claimed the tones are "how fldigi finds a frame at all".
   **They are not.** `olivia_start_tones` is read only inside `send_tones()`,
   which the transmit path calls; fldigi's *receiver* never consults it and
   free-runs a correlator bank over `FreqOffsets = 2·SyncMargin + 1` = **17**
   frequency offsets by block phase, integrating `SyncIntegLen = 4` FEC blocks —
   the same free-running strategy ours has always used. So "we sent no sync
   tones" was never a reason a fldigi decoder might copy nothing, and the real
   suspects are the tail and the things nobody has tested. **The tones are still
   right to send** — default on, and the mode's bracket — but do not let this
   entry imply they were the blocker. This is the second time an inherited claim
   in this file was the actual bug; the Olivia polarity one is the first.
2. ~~**No on-air proof of the polarity.**~~ **GONE — supplied by a third party,
   2026-10-05, and it settles item 2 completely.** See the entry below; read it
   before touching polarity again.
3. No frequency search, where fldigi searches ±8 tone spacings.

**Read this before working on Olivia again: our receiver is now proven on the
air, and our *transmitter* is not.** Those are opposite directions, and only the
first has been shown. Fork discussion **#5** ("Olivia decoding", Kevin): he
captured a real Olivia signal that MultiPSK copied, **with the text MultiPSK
showed**, and our decoder reads it on his machine —

> `32/1000 → "Wikipedia, the free encyclopedia that anyone can edit"`

at 32 dB in 1000, i.e. a genuinely weak signal, not a strong one. That is the
capture item 2 above asked for, it came from somebody with no stake in our
reasoning about fldigi's source, and it is a **user-reported success on his own
radio**, which no test in this tree can be. Polarity, the scrambler, the
interleaver and the Walsh butterfly are settled; the earlier `cq_swnet.wav`
recording settled them from our side and this confirms it from the air.

**What that does *not* prove, and the distinction is the whole point:** our
receiver locking on a real signal is the *receive* direction. It says nothing
about whether a real fldigi decoder can copy *our* transmission, which is item 1
— sync tones and a tail — and item 1 is untouched. Our block-grid lock free-runs,
so we can read a signal that has no sync tones while emitting a signal that has
none; both can be true at once, and only the second one is what "interop" means
to a station on the air. **Do not report this as "Olivia works"**: what is proven
is that we *receive* Olivia correctly. Nobody has yet decoded our transmission
with fldigi, and that is the remaining open item.

**On-air material, keep it.** `/tmp/opencode/cq_swnet.wav` is the one that
decodes: 8 kHz mono, 50.9 s, Olivia 16/500, comb 1243.75 + k·31.25 Hz, 256
samples/symbol, from the Avalon SW Net article's own `<source>` tags and tested
there in fldigi. Run it with
`SDROXIDE_OLIVIA_SAMPLE=/tmp/opencode/cq_swnet.wav cargo test -p sdroxide-dsp
--release --lib -- --ignored --nocapture an_off_air_capture_decodes` (313 s, and
it asserts the *content*). `/tmp/opencode/mx0ioa.wav` is too short to be a gate.
Do **not** use `~/Downloads/kiwi-farnham_…wav`: weak, ~2.4 kHz off, and it does
not decode in fldigi either. The operator's own capture
`~/Downloads/capture500-16.wav` is 8 kHz mono, 61.7 s, 500/16, comb centred
~978 Hz — ~22 Hz off a nominal 1000 Hz, more than half a tone step, which is why
a fixed-frequency bank fails to lock on it. Harnesses and the fldigi tree are in
`/tmp/opencode` (`ref_dump.cpp`, `ref_probe.cpp`, `olivia_polarity.py`,
`fl/fldigi-master/`). `OLIVIA-HANDOVER.md` has the same detail as a status note.

### 4. An 11 m CQ run had no bound at all

`23cd78be` skipped the transmit watchdog for a CQ run on 11 m, stating that
"the run is bounded instead by `max_tx_repeats`". But `max_tx_repeats` never
counted CQ calls — that check has always read `TxGrid | TxReport | TxRReport`,
with a comment saying a CQ repeat is exempt because "the watchdog above bounds it
instead". Both halves of one bound were gone at once, so an unattended 11 m
station keyed CQ every slot, indefinitely. Found from the operator's own log
("CQ ran more than 10 times"), fixed in `82eb8fcc`: on 11 m the count includes
`CallingCq`, and the notice says `no answer after 10 CQ calls` plus the
watchdog's "press REPLY, or call CQ to start a new run". Off 11 m nothing
changed — `repeating_a_cq_is_not_an_unanswered_call` still pins that.

**The general lesson**: a comment claiming a bound exists is not evidence that
it does. When one commit removes an instrument, check what the *other* half of
the pair actually tests.

### 5. The QSO is logged when our 73/RR73 is on the air — verified, do not "fix" it

The operator's etiquette point: the log entry must fall with the step-5 message
transmitted, sooner is fine, later is not, "otherwise people may stop being
polite and just rush QSOs". It already does, from two different hooks:
`record_tx` pushes the `»` line as the burst is **keyed** (`controller.rs:1035`)
and `on_burst_done → note_tx_sent → log_qso` pushes the `✓` **after the over
finishes**, only for `Tx73 | TxRr73` (`qso.rs:1603`). So the `✓` always follows
the `»` in the transcript. The one deliberate exception is a **Hound**
(`qso.rs:844`, `qso.rs:1136`): the Fox's RR73 closes the contact and the Hound
does not answer, because a 73 would take a slot from the pile-up.

### 6. Upstream merge (22 commits, five PRs) and what each one cost the fork

Merged as `8c398189`. **The wire did not move**: upstream is at
`PROTO_VERSION` 172, the fork at 189, and upstream's two new entries (v171
FSK441 TX's `tx_refused`, v172 `text_macros`) are already the fork's v178 and
v182. `Mode`'s discriminants are untouched — the one hunk that would have
reordered `Acars`/`HdRadio` to match upstream was resolved in the fork's favour,
so `Cquam` stays at 39 and everything from `Acars` up keeps its number.

Five files took **upstream's** whole form (strict improvements):
`morse_trainer.rs` (`serde(default)`), `geo.rs` (a `get`-based `grid4` that
cannot panic on a mistyped multi-byte locator), `fsk441.rs` (transmit wording).
Four took the **fork's** (upstream's side is a subset): `mode.rs`, `digi.rs`,
`modulator.rs`, `proto/lib.rs`. Eleven UI/doc files took the **fork's**, and
five of those are **not droppable**:

- `grid_tracker.rs` — the CB **country mode**, which upstream has no use for.
- `morse.rs` — the **SEND** pane (USB-paddle sending drill; raw evdev, Linux-only).
- `panels/macros.rs` — **F1–F9** hotkeys on the message buttons.
- `panels/cw.rs` — the straight-key and USB-paddle transmit path, beside #572.
- `persist.rs` — the **SWL reception log**.

**Owed and not taken:** upstream's post-merge refinements to those same features —
the grid tracker's panic-safe `grid4` and per-band tally (partly folded in, see
below), the Morse trainer's wasm dead-code fix, and the manual sections for all
five. They are the maintainer's own answers to review points on our own PRs, so
they should come across: a focused pass over five files, not a rebase.

The grid tracker's band-filter fix **is** in: the fork read `awards_cache`,
which follows the AWARDS window's band filter, so squares outside that band
vanished with nothing on screen to say why (upstream's `3348b389`). It now uses
its own unfiltered cache, `grid_squares`, for both the squares and the
countries' confirmation column.

The popup's mode chips now read **GRID/HAM** and **COUNTRY/CB** (the main
window's `GRID` chip is unchanged). The operator's reason, worth keeping: not
every SWL listener knows callsign etiquette, and the label should not require
them to.

### 7. #569: both questions answered, and both change the fork's plan

The maintainer: put the keyer in **`CwController` on the engine side** — "Don't
have the client generate edges and send them as `CwKey(down)`" — and keep
**`CwKeyer` iambic-only**, because straight keying already goes through
`CwKey(down/up)` and what went out is already decoded by `CwSelfRx` into
`sent_text` (`cw_controller.rs:234`, `:592`, `:504`; `engine.rs:9292`). Straight
mode in `CwKeyer` is a second implementation of that, so it goes. Reply posted.
For the trainer's contact-to-text decode we take his first option: reuse
`CwSelfRx` on the sidetone rather than write a second decoder.

**Done — see §10.** Both answers were followed as he asked: the keyer is
engine-side in `CwController`, and `CwKeyer` is iambic-only. The open question
(how the contacts travel) is **#626**: a new `Command::CwContacts { dot, dah }`
on a per-sample-timed keyer, with the `PROTO_VERSION` 172 → 173 that the
appended `DigiConfig` field and the new command need. The three-way split that
replaces the #573 plan: **#569** the keyer, **#626** the engine side, and the
**user-facing package stays on the fork** rather than going up as a third PR.

### 8. The bench has no Olivia on it right now (checked 2026-10-02)

`tools/rsp1-capture/` is the throwaway RSP1 capture tool the bench section kept
promising and did not have — CF32 interleaved to a file, built against
`vendor/soapysdr`. Two things it cost: `stream.activate(None)` is **required**
after `rx_stream` (without it `read` fails with a `Timeout` that looks like a
radio fault), and `rx_stream` takes its sample type as a parameter.

Used to look for a live Olivia to decode, on the three primary calling
frequencies from WB8ROL's own band plan — 20 m **14073.5**, 40 m **7036.5**,
80 m **3583.25** kHz. **Nothing.** The strong narrowband signal at 14073.625 kHz
(29x the noise in a 500 Hz window) is **not** Olivia: one spectral line, no
16-tone comb, and per-frame energy flat at 0.031 across every 64-symbol frame.
No comb on 40 m or 80 m either.

So the capture buys nothing today, and Olivia still needs either a live
transmission when the band has activity (20 m "high, daytime", 40 m late
evening, 80 m evenings) or a reference recording whose text is known. The frame
geometry work below is unchanged by any of this.

### 9. The screen settings a remote client stores: ungated, then explicit

Discussion #4: kevin2008-01, "This doesn't work, and the settings aren't saved
when starting a new session. Tested on versions 1.9.9, 19.10, and 1.9.3.3".

The path was written end to end — the client pushed on every settings change,
the server stored it per profile, the server pushed it back on sign-in. So it was
not a missing feature; **the gate was the problem.** Both halves only ran when
"Screen settings on" was set to **On the server**, and that picker is easy to
miss, so the feature was invisible either way — which is how it reached a tester
on three releases.

Two decisions, both the operator's, and they pull in opposite directions on
purpose:

- **The screen is not gated.** It is presentation-only and never touches anything
  the machine owns, so sharing it is harmless; a *shared keyboard* is not. So it
  always travels, and `UiSettings::client_save_scope` is **gone** rather than
  left as a key nothing reads. The settings row states where the look lives
  instead of offering a choice.
- **It is saved on request, not on every change.** Ungating alone was wrong: on a
  passwordless server every client is the same profile, so one operator moving
  the theme would move it for whoever signed in next. **Save to profile** and
  **Back to profile** buttons, and the row says where the look in force came
  from and what the last save did. Still no wire change — `ClientScreen` is
  untouched.

**The bindings opt-in stays, and the reason is written next to the control**
(`ui_tab.rs`, "Carry control bindings"): it is asked for, by someone running
their own server and reaching it from several of their own devices. The answer
to a shared station is the acknowledgement and the default-off, **not** removing
the feature — do not delete it as cruft.

**Not verified**: no server-and-browser round trip has been run. The cause is
identified and the code is right; "it works" is still a claim to be tested.

### 10. #569: BUILT — the engine-side keyer, and the one claim that was wrong

**Built in `353d136f`, all six pieces at once, `PROTO_VERSION` 190.** It is
committed, pushed, compiles silent across the workspace and passes its tests.
**And it is benched** — see the bench paragraph at the end of this section.

**The bug it fixes, in one sentence.** A paddle could not key a rig that keys
itself, and on every other route the timing was quantised to whatever the UI
thread sampled — both because the keyer was in the wrong place.

**The one claim in the handover that was wrong, and do not repeat it.** The
handover said an engine-side keyer *"fixes that properly rather than by
refusal"*, and that the keyer would key *"through every route — CAT, MCW/audio,
VOX"*. **It does not, and it cannot.** A rig that keys itself from text sends
the text over the control port and times the elements itself, so on that route
the sidetone is **never transmitted**. The keyer generates a tone; the tone is
not what that radio sends. `set_straight` therefore **still refuses** when
`self.cat.is_some()`, and that refusal is load-bearing — deleting it would
re-open #495 and give the operator a KEY chip that lights and sends nothing.

So the fork's answer is the house rule rather than a refactor: **the refusal
stays and the way out is named.** CW keying = **Sound card (MCW)** holds the
rig on a sideband and transmits the program's own tone, and then the keyer keys
through it — the confirmed bench route. The KEY chip's hover says which mode it
arms, and on a self-keying rig it says that *neither a paddle nor a straight
key* can reach it and which setting does. **Where a control cannot do the thing,
it must say so; it must not silently do nothing.** That was the actual defect,
not the refusal.

What "every route" *is* true for: any route where the program's own audio
reaches the transmitter — MCW, VOX, and a rig whose keyer can be put in
semi-break-in or whose sidetone path we can feed. That is most rigs, and it is
the whole 11 m bench. It was never true for a CAT keyer.

**The six pieces, as landed.**

1. `CwKeyer` is **iambic-only** — `KeyerMode`, the straight state (`contact`,
   `mark_start`, `dit_est`), `poll_straight` and the two straight tests are
   gone; `set_iambic(IambicMode)` replaces `set_mode`. Straight keying keeps
   `CwKey` and is read back by `CwSelfRx` as before.
2. `CwTx::next_manual_block_timed(out, rate, keys)` takes a **per-sample key
   timeline**; `next_manual_block` delegates with a constant state and restores
   `held` afterwards, because the caller owns that flag. A `keys` slice shorter
   than `out` **holds its last value**.
3. `CwController` owns the keyer and polls it **per output sample** in the
   hand-key branch of `fill_tx_block`, on a monotonic `keyer_t`, feeding
   `feed_sent_decode` as before. The lost-key-up cap applies to the **straight**
   path only — a keyer's elements are bounded, and holding a paddle *should*
   send indefinitely, exactly as a real keyer does.
4. The refusal is now a **named** one, as above.
5. `cw_key.rs` keeps only the device: contacts, the reverse switch, the
   exclusive grab, a monitor tone. No keyer, no client-side decode.
6. The panel splits: **straight** sends `CwKey(down)` as it always did;
   **iambic** sends `Command::CwContacts { dot, dah }` on a change and nothing
   per frame.

**The wire.** `CwContacts { dot, dah }` appended last, `PROTO_VERSION` 189 →
**190**, with the register entry and a postcard round-trip over all four
contact pairs. `cw_contacts_sent` on the app is what makes it once-per-change.

**Two things the split forced into the open, both worth keeping.**

- **A straight key's contact is middle-*or*-dit.** A paddle box with a straight
  key in it reports the key on whichever single contact it wired it to: the
  middle jack where there is one, the dit contact where there is not. Reading
  only the middle would leave every straight key in a plain two-contact box dead
  **and silent**. `straight_contact()` is middle-or-dit for that reason, and the
  doc says why rather than leaving it to look arbitrary.
- **The trainer's read-back moved to the tone.** The Morse SEND pane read back
  through the keyer's own text decode, which went with the keyer, so it now
  decodes the **monitor tone** through `CwSelfRx` — the maintainer's first
  option, and the decoder the transmit path already uses. Its "n elements"
  readout went with the counter and is now the characters actually decoded.

**Tests.** Three new in `cw.rs`'s `manual_timeline_tests`, written against the
**envelope**, not the samples: the tone is a shaped sine with a 5 ms
raised-cosine ramp and a zero crossing every half period, so a per-sample
threshold asserts nothing (the first draft of these failed on the *old* correct
code for exactly that reason). `the_key_timeline_is_followed_per_sample` puts a
key down and up **inside one 50 ms block** and pins *where* the silence starts —
the property a per-block read cannot have. Its pair
`the_plain_manual_block_still_keys_for_its_whole_length` pins the straight
path, so the two distinguish each other rather than both passing on anything.
22 CW tests + 3. `skim_window` is load-sensitive, not red — see the build
and test section.

**The bench: PASSED (2026-10-02), and the setting is not the one to assume.**
CH55x `1209:c550` "-Yuan-3key" (raw contacts, no iambic of its own, which is why
the keyer is in software) → **radio 1, the CRT SS9900v** on **27.265**, CW keying =
**Sound card (MCW)**, VOX, and **the rig's mode set to USB**. Arm **KEY**, then
iambic A and iambic B: single dits, a dah, a squeeze, and a held paddle. Listen
for clean dit/dah and watch the **sent_text** read-back follow. The operator's
verdict was **`all works!`**, iambic both ways.

**The setting that decides it, and why it is counterintuitive.** The rig was in
**CW**, and the run failed there — which is the §10 refusal talking, and it is
correct behaviour, not the keyer misbehaving. A rig set to CW ignores the
sound card entirely: it makes its own tone from whatever text it is sent, so
**there is no audio for a keyer to key**. Change the rig to **USB** (or any
sideband) and MCW holds it on that sideband and transmits the program's own tone,
which is the only route on which a generated tone means anything. **Set the rig
to USB first, then arm KEY.** Three CW sections of the manual now say so
(`02989ae5`).

**Not benched in the upstream shape.** #626 carries the engine half *without*
the panel, because that is what the maintainer asked for, and nobody has run the
keyer against a rig in that split form.

### 11. An `--oob-tx` switch: fork-only, and never upstream

Asked whether `--oob-tx` could become a Settings → General switch with a warning
popup and a restart. It can, and the flag is the right candidate: it already
loosens the lockout for a launch (`src/main.rs:264`,
`settings.tx_ham_only && !self.oob_tx`), and a restart carries it into the new
process with **no wire change**, because the flag is read at startup exactly as a
typed launch reads it.

The shape, if it is built: toggle opens a **modal** rather than flipping the flag
(the bindings-acknowledgement pattern); on confirm, persist and re-exec
`current_exe` with the same argv plus or minus `--oob-tx`. And the existing
startup window stays — `frame.rs:2016` shows "TRANSMIT LOCKOUT DISABLED"
whenever the engine reports `oob_tx`, driven off the *engine's* state so a remote
client is warned too. That window is what preserves the deliberate-act-at-launch
property the flag's own doc insists on (*"a deliberate act at launch, not a
setting to be toggled by accident"*): every start is still acknowledged by hand,
which is the friction the CLI imposes today, minus remembering the flag.

**Fork-only, and not to be offered upstream.** It is 11 m / CB operator territory
(`cb_tx_allowed()` is the fork's own), and a switch that loosens a transmit
lockout is the last thing a maintainer should have to review from a stranger.

### 12. The standing direction: CW keying is the goal, this fork first

The operator's order, 2026-10-02, and it outranks everything above about
upstream: **this fork first and foremost. Anything offered upstream is a bonus
for the maintainer and pure goodwill — it is never the reason for a decision.**

The goal is plain: **people must be able to key CW, period.** Where that needs
the operator to make a choice, make it and *inform them what the choice means*:
on the path where the program generates the tone and keys the rig by audio, a key
wired straight to the radio's key jack is not part of that path and will not key
through us — so say so rather than letting them discover it on the air.

**What that means for #569.** The engine-side keyer is not a refactor for its own
sake; it is what removes the one real reason CW cannot be keyed today. A paddle
cannot key a rig that keys itself, because `cw_controller.rs:923` refuses
(`set_straight` returns early when `self.cat.is_some()`) — and that refusal is a
missing capability wearing a safety's clothes. With the keyer in `CwController`
the program owns the timing, so the paddle keys through **every** route: CAT,
MCW/audio, VOX. The operator's only remaining choice is where the tone goes.

So the six pieces above are the work, and they landed together in `353d136f` —
see §10, which is the record and which also says which claim here was wrong. The maintainer's answers agree with the direction — engine-side,
`CwKeyer` iambic-only — and are followed where they do not conflict with it, but
they are not the constraint. If a PR upstream ever follows from it, it is
offered afterwards, as a courtesy.

**One thing that must not be lost in the doing:** the fork's own house rule that
a control must never silently do nothing. A paddle that cannot key a CAT rig, an
oob-tx switch that changes the lockout, a screen setting that does not travel —
each of those is the same bug wearing different clothes, and each is now either
fixed or labelled.
### House note: do not monitor CI continuously


The operator's instruction, after a session lost a lot of time to 90-second poll
loops: **check a run once per finished task, not in a loop.** A tag push is
followed by other work; the run is checked when the next thing finishes.

## Session 2026-10-06, end of day: 2.0.0, DAB, and where upstream now sits

**Two standing directions changed today, both by the operator, and both supersede
anything above that says otherwise.**

1. **Upstream is now secondary. The fork is the focus.** Upstream PRs are not the
   queue any more and new work does not go there — *"we'll be leaving upstream for
   what it is and fork is our main focus. just occasionally check on the issues
   there if there's something relevant for us."* So: **check the upstream tracker
   occasionally for anything that touches this fork, and nothing else** — no
   triage, no replies, no new PRs. The existing open PRs are left as they are
   (they are listed in the queue section, which is still true about their state,
   just no longer the day's work).
2. **Settings are stored on the server, not in the browser.** Kevin's argument on
   fork discussion #4, and the operator agreed with it. It is a good argument and
   it is his: *"network managers store everything on the servers… reliability.
   Whatever the user does to their session, the server returns the same thing"* —
   and the half browser storage can never have, **you can reset a session from
   somewhere else** (he overwrites `radio.json`/`session.json`/`config.toml` over
   SSH). He also measured that browser storage cannot be depended on: `persist()`
   refuses silently in Chrome and prompts in Firefox, and even at `persisted() ===
   true` his theme still did not survive a restart while his bindings did — which
   is a client *read* fault, not eviction.

So `2.0.0_brown` carries the **DAB / DAB+ receiver** (merged from the week-old
`fork/dab`, enabled, experimental) and **the profile's screen saving itself**
while signed in. Both are described in the release entry; the DAB merge's three
conflict resolutions are worth keeping in mind for the next stale-branch merge,
because two of them were *silently* wrong rather than conflicting:

- **`PROTO_VERSION` 194 → 195.** The branch had numbered its DAB change **187**
  and its ALC/band-gain change 188 — but `main` had already spent 187 on the ALC
  work and 188 on the client screen. **Check the register against `main` before
  trusting a branch's number.**
- **`Command::SetDabConfig` and `ServerMsg::DabStatus` moved to the tail.** The
  branch had the command mid-enum, before `SetGainByBand`.
- **`RadioState::dab` moved to the tail.** The branch had inserted it **after
  `ais`, mid-struct**, which shifts every field below it on a struct that rides
  the wire whole — and its own doc comment claimed it was on the tail. **It
  auto-merged cleanly. `git merge-tree` showed no conflict. Append-only is not
  something a merge protects; it has to be checked by reading the field order.**
- And the merge **duplicated a match arm** (`SetGainByBand`, present on both
  sides) — the one the gate caught, as an unreachable pattern.

**First job tomorrow, and it is half-done: the 32-bit ARM build.** Fork
discussion #13 — ipaddr42 wants a decode node on older ARM hardware (Debian 13
armhf, `ARMv7-A + VFPv3`), and every ARM artifact this project publishes is
64-bit `aarch64`. A probe lives at `.github/workflows/armv7-probe.yml`
(`workflow_dispatch`, its own workflow **on purpose**: `create release` needs
every build job, so a target expected to fail must not be able to fail a
release). It has run twice and failed both times, and **the two failures are the
whole finding**:

- **The Rust cross-compiles fine.** The second run downloaded and compiled
  hundreds of crates for `armv7-unknown-linux-gnueabihf` before stopping, so the
  decoder crates — the thing to fear — were not reached and are not the wall.
- **It stops at `alsa-sys`**, the sound-card binding:
  *"pkg-config has not been configured to support cross-compilation."* An amd64
  runner has no armhf `libasound`. Hand-rolling it inside a `run:` step was tried
  and **did not take** — the log shows the cross toolchain installing and **no
  `libasound2-dev` at all**, no fetch from `ports.ubuntu.com`, and
  `PKG_CONFIG_ALLOW_CROSS` never reaching the build script. Two iterations
  established that and no more; do not spend a third on the same approach.
- **The fix is a sysroot, not more apt.** `cross-rs/cross` (a container with a
  complete target sysroot) or the build inside `debian:armhf`. **Whether that
  then reaches the vendored C and fails there is still unknown, and is still the
  answer to his question.**

The reply to him is **drafted and unsent** — it owns two real errors of ours (the
Pi 1 is **ARMv6** and was listed as ARMv7; and we answered a performance question
he had not asked, when he had already conceded the unknown), accepts his spec,
and says the one thing he cannot know: **SoapySDR is droppable with
`--no-default-features`, and the rest of the vendored C — faad2, dream, nrsc5,
xng, rade — is not behind a feature**, so that is where a 32-bit build will
break. It also tells him plainly that these replies are written with an AI
assistant, because the operator is in heavy treatment for PTSD and is not sharp
at the moment, and he had noticed. **He decides when that goes.**

## Session 2026-10-07: DAB is confirmed in the field, and DMR is asked for

**Standing note on the phone, from the operator (2026-10-07).** Kevin uses his
phone daily and keeps it — the operator would prefer he worked on a tablet for
screen size, but *"he does love his phone ;)"*. So the phone is not a tier to
tolerate: it is the screen most of our field feedback comes from, and its layout
deserves the same care as the desktop. **Do not treat "works on a tablet" as an
answer to a phone report.**

**DAB works on somebody else's hardware — the first field confirmation.** Fork
**issue #15** ("DAB -RAdio", **dig647**, 2026-10-06): he asked whether DAB radio
could be implemented at all, the reply was that it would be in 2.0.0_brown with
a bandwidth caveat, and on 2026-10-07 at 04:12 he came back *"ja sehr schön …
sehr gute Arbeit"* and closed it. So the milestone release is the one that
settled it, and the standing open item — "DAB syncs but decodes no FIBs" — is
about **our live bench front end**, not about the decoder or the release.

**Kevin is positive on the settings-on-server direction.** He left a **❤️** on
the server-settings reply in discussion **#4** (the one that says the screen is
ungated, saved on request, and the bindings behind an opt-in), plus 👍 on the
SSTV reply (#7) and the recording-replay scope (#10). No new comments — these are
reactions, which do not show in a comment listing; check them separately.

**New request: DMR** (dig647, in the same #15: *"und DMR währe auch gut"*).
Recorded as a request, **not scoped and not started**. Two things to tell him
honestly when it is answered, and neither is a "no": DMR is a **VHF/UHF** mode
(so only for a listener whose front end reaches there — the RSP1 does), and its
hard part is the **AMBE+2 voice codec**, the same class of dependency as DAB's
audio. It fits the wideband-lane machinery the fork already has (ADS-B, AIS,
DAB); the codec is the work.

> **Both halves of that were wrong, 2026-10-11.** The codec is the *easy* half
> (mbelib-neo is GPL-2.0-or-later, we are GPL-3.0-or-later, there is no licence
> wall) and the work is the C4FM air interface. It was decided against anyway —
> **a patent, not the work** — and the full scoping is in **"DMR: scoped to the
> codec, and decided against"** near the end of this file. Do not re-derive it.

**03:37-ish: the armv7 first job was picked up and re-dispatched with `cross`.**

**`cross` + `no-simd` + `patch` → the build now reaches the final link.**
Three walls down in order, each one a whole run:
1. `alsa-sys` — a real armhf sysroot via `cross` (run 37577495536).
2. `opus` NEON intrinsics — opusic-sys's `no-simd` feature, reached through a
   gated `sdroxide-server/arm-no-simd` feature. **Do not "fix" it with
   `-mfpu=neon-vfpv4`**: Debian armhf is VFPv3 and NEON is not in that baseline.
3. `rade_c`'s cmake ExternalProject — `patch` was not in the image
   (`sh: 1: patch: not found`), added to `Dockerfile.cross-armv7`.

**IT BUILDS (2026-10-07 10:33, run 37606994024).** Four walls, each one a
whole run, and every one of them a *packaging* wall rather than the code:

    sdroxide: ELF 32-bit LSB pie executable, ARM, EABI5, for GNU/Linux 3.2.0

1. `alsa-sys` — a real armhf sysroot via `cross` (runs 37577495536).
2. `opus` NEON intrinsics — opusic-sys's `no-simd`, reached through a gated
   `sdroxide-server/arm-no-simd` feature. **Never "fix" it with
   `-mfpu=neon-vfpv4`**: Debian armhf is VFPv3 and NEON is not in that baseline,
   so a NEON build can SIGILL on supported hardware.
3. `rade_c`'s cmake ExternalProject needed `patch`, which was not in the image.
4. **The one that took three tries: `rade_c` built its nested opus with the
   *host* compiler.** `BuildOpus.cmake` passes
   `--host=${CMAKE_C_COMPILER_TARGET}` to autotools only when that variable is
   set, and CMake does not infer it because the wrapper CMake runs natively —
   so opus configured as a native build. Three separate faults came out of that
   one gap, in order: a mixed armhf/x86-64 rlib (*"file format not recognized"*
   at the final link), then `configure: error: cannot run C compiled programs`
   once the compiler was right but `--host` still empty. Two things had to be
   true: the un-suffixed `CC`/`CXX`/`AR`/`RANLIB`/`LD` **whitelisted through
   `Cross.toml`'s `[build.env] passthrough`** (cross passes only a whitelist into
   the container, so naming them on the step did nothing), *and* the autotools
   triple — `arm-linux-gnueabihf`, **not** the Rust target triple — passed in as
   `CMAKE_C_COMPILER_TARGET`. `crates/sdroxide-rade/build.rs` forwards it; unset
   on every native build, where nothing changes.

**The probe stays a probe.** Its own header says folding it into the release
matrix waits until it "has run on someone's hardware", and that has not
happened yet. The binary is attached to the `v2.0.1_brown` **pre-release** by
hand instead (`sdroxide-v2.0.1_brown-linux-armv7-cat.tar.gz`), which carries no
CI risk — a job that can fail in front of `create release` is a job that can
fail a release. When ipaddr42 reports it runs, fold it in.

**2.0.1_brown is cut as a pre-release** (tag pushed, `gh release create
--prerelease`, workflow 37600454169). It carries the strip scroll fix, the SSTV
dial hold and the two browser fixes — and **not** the armv7, deliberately: a
failing build job fails the release, which is the "would break something" case.

**Do not re-try the hand-rolled multiarch apt.** Two attempts are recorded above;
`cross` is what worked, and the two files it needs are in the tree now.

**Branch `fork/responsive-layout`: the layout bug class, and the oracle for it.**
The operator's call, and the evidence is three reports from three unrelated
panels — Kevin's *"3 radio = bug de décalage"*, *"the boundary line is missing,
the outline extends beyond the edge"*, and the tab strip cut off. One mechanism:
a row laid out wider than the window. Nothing caught them because the only
render test we had (`phone_crash_regression_*`) asserts a frame does not
*panic*, and the strip was overflowing the whole time while it passed.

- **The oracle works and is the deliverable.**
  `multi.rs`'s `page_overflow` drives a real `MultiApp` (via
  `eframe::CreationContext::_new_kittest`, which is what made a multi-radio
  render harness possible at all) at 320/360/411/768/1250/1920 with 1–3 radios
  and asks **how wide the page container got**. It measures `Shape::Rect`
  backgrounds anchored at the left edge — *not* the widest shape, because a
  scrolled strip paints tabs past the window on purpose and clips them.
- **FIXED — the strip.** Three radios on a phone overflowed by 47–220 pt; the
  page went to 407–583. It is a horizontal scroll area now
  (`MultiApp::radio_strip_scrolled`), and the sweep reads 362 everywhere.
  **A wrapped row does not fix it and the reason is worth keeping: a tab is a
  `scope`, and egui *squeezes* a scope rather than wrapping it** — the third tab
  folded its own name one letter per line, kept its own min width and stayed on
  the row. The strip got *wider* (602), not narrower. Do not retry the wrap.
- **NOT FIXED — the other half.** A top-bar chip row is a couple of points
  wider than its panel, egui grows the panel, and the page is **2 pt over at
  360 and 4 at 1920 with one radio** — so it is not the strip. The panel's right
  border is painted off-screen and there is no line to close it. The strict,
  **ignored** `nothing_is_drawn_wider_than_the_window` is the record; the active
  `the_strip_does_not_push_the_page_past_the_window` is the same sweep with a
  6 pt tolerance and is the guard for the class that is fixed.
- **Items 4 and 5 attempted, on the branch, not verified in a browser.** *Copy
  diagnostic* wrote to the clipboard via `ctx.copy_text`, which a page cannot
  always do (secure context + live gesture, and a refusal is silent) — on the
  web it writes a file now, through the SAVE chips' download. *Share picture*
  returned `is_ok()` on `navigator.share`, which reports only that the promise
  was made, so a rejected share looked like success and the fallback never ran;
  it is gated on `can_share_file()` now. **No browser was driven here** — this
  is Kevin's to confirm.
- **Item 3 (the 3-radio tab-close flicker) is NOT solved.** The obvious lead —
  that closing a remote tab re-opens it via `open_peer_radios` — is wrong:
  `peers_opened` is deliberately never cleared on close (see its doc). The
  report stands: with three tabs, closing station 3 while viewing station 1
  flickers, client-side only, and it is the next thing to look at.

### The SSTV view on a demod-audio front end — fixed, and CTR was the bypass

**Confirmed on the operator's bench (2026-10-07): "the crt is all fine now".**
The report was that in SSTV on the **CRT SS9900v** the picture was squeezed into
part of the waterfall and half off the right edge. It was, and it took three
attempts because the first two each fixed half and were then **undone by
something else**:

1. The **span** — narrowed to the SSTV band (`zoom_out_window`). Screenshots
   then showed the window centred on the *carrier* with the picture either side.
2. The **focus** — `focus_hz` answered the picture instead of the dial. Still no
   change on the radio that mattered.
3. **CTR.** `center_on_vfo` re-centres the view on **the dial** every frame
   (`spectrum_view.rs:2050`), and the only modes that escape it are the ones
   whose signal sits off the dial — `AudioCursor::center_on_cursor`, built for
   RTTY/NAVTEX/DSC. **SSTV is one of those and was not in the set**, so CTR put
   the window back on the carrier and undid both fits. The cursor now anchors
   SSTV at the picture centre (1750 Hz, signed by `is_lower_sideband_at`) and is
   marked centre-on-cursor, reusing `SSTV_TONE_HZ` so the anchor and the window
   cannot drift.

**The lesson, and it is the same one as the Olivia polarity and the QSL card:**
the thing doing the damage was *not* in the code the bug was reported against.
Two fixes were written and both were correct and both did nothing, because a
flag set on this very bench that morning (`center_on_vfo` default on, 2026-10-07)
was fighting them. **What found it was the operator's one-click experiment** —
*"clicking ctr and switching modes and back changed the view"*. Ask for the
experiment that isolates the other combatant before writing the third fix.

**Scope, and it is `DeviceCaps::audio_mode`:** the SS9900v is a **10-12 m**
sound-card rig, so its dial is stuck on a suppressed carrier the operator cannot
see and the picture is always 1.75 kHz to one side. An **SDR is not that** — the
RSP1's own SSTV view has the operator placing the dial *on the picture* — so
shifting their focus would move a view they had already chosen. Both are pinned:
`sstv_audio_window`/`focus_hz` answer for an audio front end and keep the dial
otherwise — and the operator confirmed the SDR case as it is (2026-10-07): the
RSP1's SSTV view is *meant* to stay where they put it, so do not extend the fit
to SDRs.

**Misreading to avoid:** the SS9900v is 10-12 m only, so a screenshot at 80 m is
the RSP1 whatever the tab looks like. An 80 m "SS9900v" screenshot with a 3.6 kHz
span was read as the rig for an hour; it could not have been.

### Upstream #643 (filed upstream by accident, reproducible here): the dock is tucked under the operating panel

siliconburner reported it against upstream; the operator reproduces it on our build
and is the one with the screenshot. **The SSTV or FT8 bottom panel goes over the
edge of the docked Bands & Modes panel, and the dock is partly tucked away.**

**Two of my own wrong turns here, both worth keeping**, because they are the
2026-10-07 house rule failing in new costumes:

1. I claimed the screenshot proved it was upstream's build, from the window title
   reading `sdroxide — Xegu (CAT)`. **The fork hardcodes that string** —
   `"sdroxide — {}"` at `app/mod.rs:1997` and `app/frame.rs:1502` — and
   `FLAVOR` is only the startup title, overwritten once a rig is attached. I had
   not traced the string to its source.
2. I then claimed it from the SSTV path `…\sdroxide\sdroxide\config\sstv_rx`
   against our `…\sdroxide-brown\…`. **That path is a stored setting**
   (`store_where(&self.sstv.dir)` — the saved directory, not the live
   `config_dir()`), so it carries whatever an earlier install wrote and says
   nothing about which build is running.

Both times I inferred a conclusion from a number I had not traced, and **the
decisive evidence was in the screenshot the whole time**: the dock is present at
all, and the mode list contains **OLIVIA**. Upstream's build has neither.

**What the code already rules out**, read on 2026-10-07:

- **Draw order is correct.** `band_dock_panel` is a `Panel::right` shown at
  `frame.rs:520`, well before the operating panel, so egui reserves the column
  for the centre.
- **The panel's width is not stale.** `let width = ui.available_width()` is read
  inside the digital branch (`frame.rs:1008`), *after* the dock.

**And what the operator's observation rules out**, which is the useful part: the
overlap is a **constant** — it does not change with a resize of the main window
nor with a resize of the dock itself. So it is **not** proportional to the dock's
width, the window's width or the panel's width, and it is **not** egui's
remembered `PanelState` for the resizable dock. **A constant offset is not a width
that was miscalculated**; it is something painted at a fixed position or with a
hardcoded inset. That rules out sizing as the mechanism and is where the next
session should start.

**MEASURED (2026-10-07), and the dock is innocent.** A probe test drove one
desktop frame at 1920×1080 with `band_docked = true`, `band_dock_visible = true`
and `mode = SSTV`, printing every shape rect:

| window | dock rect | dock width | operating panel right edge | **overlap** |
|---|---|---|---|---|
| 1920 | 1641…1920 | 279 | 1680 | **39.0** |
| 1600 | 1321…1600 | 279 | 1360 | **39.0** |
| 1400 | 1121…1400 | 279 | 1160 | **39.0** |

**egui reserves the column correctly.** `ui.available_width()` immediately after
`band_dock_panel` is **1640**, and 1640 + 280 = 1920 — so the dock is placed and
the centre knows about it. The dock is **not** the fault and neither is
`PanelState`.

**The operating panel overdraws by a constant 40 pt** (the 39 measured plus the
stroke): it paints `0…1680` where the centre is `0…1640`, and the panadapter
above it measures `0…1624`, so the two are not even the same width. Constant at
every window width, which is the operator's observation reproduced exactly.

**And the path is not the one to expect**: SSTV does **not** reach the digital
branch's `let width = ui.available_width()` at `frame.rs:1014` — an instrumented
trace at that line **never fires** for SSTV. It goes through the earlier branch at
`frame.rs:724`, whose `allocate_ui(vec2(width, panel_h))` at `:930` is the rect
that overlaps. So a fix aimed at the digital branch would not touch this.

**CORRECTION, 2026-10-08, and this one retires the whole section above.** The
operator's own screenshots of the real station **contradict every measurement in
it**, so none of it should be pursued. Three pictures, at two window sizes:

- **FT8, OPERATE, 1920 wide, dock present** — the digi panel's right edge meets
  the dock's left border cleanly. **No overlap at all.**
- **SSTV, OPERATE, same window** — the chip row runs *just* past the dock by a
  few points. Nothing like the 39 pt below.
- **The real fault** — a **narrow window (~960 pt) with no dock at all**, where
  the FT8 digi panel is laid out **wider than the window**: the map, the `Idle`
  box and the `no messages` box run to the right edge with **no right border**,
  and the left-hand filter chips are cut mid-word.

**So the 39 pt was an artefact of the harness**, which forced a state the
operator's station never reaches. That is the house rule firing on me a third
time in a day: *a measurement that disagrees with a report has the measurement
as the first suspect*. The probe set `state.rx[0].mode` directly and so only ever
drove the OPERATE path, and the state it built was not the state on screen.

**What is actually wrong, and it is a different mechanism:** a column laid out
**wider than its own window when the dock is absent**. Not the dock being tucked,
not a panel overdrawing it, and not arithmetic that scales with any width — which
is why **no resize changed it** and why it is cut on *both* sides: a fixed
minimum somewhere in the panel, not a miscalculated width.

**REPRODUCED, 2026-10-08, and it is SSTV-specific.** The operator's third
screenshot is **SSTV in a narrow window**, and a probe driving SSTV at four
widths finds the same shape:

| window | dock left | SSTV panel right | **overlap** |
|---|---|---|---|
| 1920 | 1641 | 1680 | **39** |
| 1280 | 1001 | 1048 | **47** |
| 1100 | 821 | 980 | **159** |
| 960 | 681 | 931 | **250** |

At 960 the panel covers nearly the whole dock, which is the picture. At 1920 it
is a few points, which is why "OPERATE is fine" and "LISTEN shows it" were both
true at once — **they were the same fault at two window widths.**

**And the "constant 39 pt" was the 1920 row of this table**, not a property. The
earlier per-mode sweep only ever ran at 1920, so every mode looked constant and
the arithmetic was read as a fixed offset. Nothing here is constant; it scales.

**CONFIRMED BY THE OPERATOR, and the sweep was right about the modes.** He tested
it and the overlap is in exactly four — **Olivia, SSTV, SSTV-FM and RIFP** — and
in **both** LISTEN and OPERATE. That is the per-mode sweep's list verbatim, so the
correction above overreached: the sweep was wrong about *constancy*, not about
*which modes*. His is the authority on the modes; ours on the widths.

**And Olivia is far worse than the other three.** Driving the probe at 960 pt:

| mode | painted width | vs a 960 pt window |
|---|---|---|
| **Olivia** | **0…1770** | **810 pt past the window** |
| SSTV / SSTV-FM / RIFP | 0…931 | inside the window, but **over the dock** |

So there are two things, and they are not the same size:

- the **three image modes** (all of `Mode::is_image()`) overrun their *column* and
  cover the dock, while staying inside the window;
- **Olivia** overruns the *window*, which is a different order of fault and the
  one worth finding first — 1770 pt of panel in a 960 pt window is not a
  rounding, it is a width taken from somewhere that is not this ui.

At 1920 Olivia does not overrun the window, so its width is not simply the window
either. Whatever it is, it is only wrong at narrower widths — which points at a
minimum or a clamp whose floor binds only when the column is small.

**The two reads to do, in this order:**

1. **Olivia's panel split** — the same question as SSTV's, on the panel behind
   `Mode::is_text_modem()`. It is the largest overdraw in the tree and it leaves
   the window, so a wrong floor there is the worst of the four.
2. **Then the image modes**, which are one function (`image_panel`) and already
   narrowed to "the `ui` it receives is ~21 pt narrower than its allocation".

**A note on the probe**, so the next run is not misled: it reports rects **past
the window edge**, which finds Olivia and misses the image modes — those stay
inside the window and cover the *dock*. Both edges matter, and the fix's oracle
should check the **dock's left edge** as well as the window's, or the guard it
leaves behind will only catch one of the two.

**So `image_panel` is in scope after all** — `panels/sstv.rs:595`, the one panel
`operating_panel` calls **without** `panel_h` and the only one that computes its
own column widths from `ui.available_size()` (`:676`). It is being given the
width it allocates itself from something wider than the allocation it was handed
— at 960 that is 931 against a real column of 681.

The reproduction is `probe_narrow_window_overflow` in the `#[cfg(test)]` block at
the bottom of `crates/sdroxide-ui/src/app/mod.rs`, reading `DOCK_W` and `MODE`
and printing every rect past the window edge. Uncommitted.

**MEASURED, and the column arithmetic is NOT the culprit.** An instrumented trace
at `sstv.rs:677` gives, for SSTV with the dock open:

| window | `avail.x` | `left_w + tx_w + handle_w` | real column | painted panel |
|---|---|---|---|---|
| 1920 | **1620.0** | 1620.0 | 1641 | 0…1680 |
| 1280 | **980.0** | 980.0 | 1001 | 0…1048 |
| 960 | **660.0** | 660.0 | 681 | 0…931 |

Two things fall out, and both close off the obvious suspects:

- **The four-column split sums to `avail.x` exactly at every width**, so the
  `.max(300.0)` / `.max(160.0)` floors on `left_w`, `tx_w` and `gallery_w` are
  not overrunning anything. That theory is dead; do not spend time on it.
- **But `avail.x` is ~21 pt *narrower* than the column the panel was actually
  allocated** (1620 against 1641, 980 against 1001, 660 against 681) — so the
  `ui` it receives is already short by something constant, **and the frame that
  paints is wider than `avail.x` as well** (1680 against 1620 at 1920).

So there are **two** widths in play that disagree with each other and with the
allocation, and neither is the dock: what `ui.available_size()` reports here, and
what the panel's own frame ends up painting. The `ui` reaching `image_panel` is
therefore **not** the `allocate_ui` child from `frame.rs` — or something between
them has taken its own measurement and re-laid-out the row.

That is where to look next, and it is above `image_panel`, not inside it. The
`operating_panel` call at `frame.rs:2368` passes `panel_h` to every other mode
and **not** to `image_panel`; whatever the branch does when `panel_h` is absent is
the obvious first thing to read.

**Superseded 2026-10-08:** the three-step plan in that evening's session
section above (instrument the two panel calls, fix, prove with `probe_narrow`)
replaces this one — it does not need a second harness and it names the panel.

**Resolved 2026-10-08, later session: #643 is FIXED** — see `## Session
2026-10-08, night:` at the top. It was never `max_rect` vs `available_width`
either: every Frame in the SSTV panel budgeted its content box to the whole
allocation and forgot its own **stroke**, so the row grew 2 pt a frame until it
landed past the dock. The regression test
`the_operating_panel_stays_inside_its_column` checks **both** edges (window and
dock-left, as this section's probe-gap note asked) and fails against the old
budgets with `Sstv at 960 pt: ink x 0..692 crosses the dock edge at 681`.

**CORRECTION to the note below this line, and the earlier claim was wrong.**
It said "a child overruns its own allocation by 40 pt". That is **not** what is
happening, and the shape dump says so: the panel's **frame and its content agree**
on 1680 — the frame is `0…1680` and the content ends at **1670**, exactly a 10 pt
inner margin. An overrunning child would sit inside the 1640 allocation and stick
out; here the whole panel was laid out at 1680, so the `allocate_ui` that produced
it was **handed 1680**.

What is established:

- `width` is traced at `ui.available_width()` = **1640** at `frame.rs:724`, and
  **never reassigned** anywhere in `:724…:924` (swept).
- The allocation is `ui.allocate_ui(egui::vec2(width, panel_h))` at `frame.rs:924`
  → `operating_panel` (`:2368`) → for SSTV, `image_panel`
  (`panels/sstv.rs:595`), which computes its own columns from
  `ui.available_size()` (sstv.rs:676).
- **A structural oddity worth noting:** `image_panel` is the only panel
  `operating_panel` calls **without** `panel_h`, and it is the one that lays out
  its own width.

**The one question left:** the `ui` handed to `image_panel` reports **1680**
available when the parent allocation was traced at 1640. Note egui's own
distinction, which is probably the key: at `:724` `ui.available_width()` was
**1640** while `ui.max_rect()` was the **whole window**
(`max_rect=[[0 0] - [1920 1080]]`). So the dock's reserve lives in
`available_width`/`available_size`, **not** in `max_rect` — and anything that
sizes itself from `max_rect` (or from an `available_size` taken before the dock
took its share) gets the un-reserved width. That is the line to pull next.

**Not done, and it is the next thing:** the overrunning row is not yet
identified. The probe is still uncommitted in the `#[cfg(test)]` block at the
bottom of `crates/sdroxide-ui/src/app/mod.rs` (`probe_dock_and_operating_panel_rects`,
reads `DOCK_W`); `frame.rs` is reverted clean. The way to finish it is the same
oracle shape as the border fix: have the probe walk the panel's *shapes* and name
the one crossing 1640, which points at the child instead of the panel.

### NEW, and it outranks the two above: **a sign-in prompt per radio** (Kevin, 2026-10-07)

Comment `18796013`, in the same #16 thread, opening with *"New bug => it's
asking for a session password for each radio station. Radio 1 = session password
+ popup warning / Radio 2 = … / Radio 3 = … This is way too much! It ruins the
user experience."* He has **three** radios on one passworded server (WOLF = id 0,
HACK RF = id 2, PLUTO = id 3), so this is the first report from somebody for
whom three radios is the normal case.

**The design already says this cannot happen, and that is why it needs naming
rather than a fix.** `login.rs`'s module docs are explicit: *"A station serves
each of its radios on a connection of its own, and asks each one for a password —
so a client holding a whole station would face one sign-in per radio. It does
not: an answer a station has accepted is kept for as long as the program runs
(`SESSION_LOGINS`) and offered to every other connection to that same station."*
The machinery behind that claim, all read on 2026-10-07 and **all apparently
sound**, which is why the report is a question and not a patch:

- `login::station_key` strips the radio off, so `/ws`, `/ws/2` and `/ws/3` all
  key the station the same (`rest[..rest.rfind("/ws")]`) — and `wss://` contains
  no `/ws`, checked, so a secure URL does not shift it.
- `SESSION_LOGINS` is written from `LoginForm::settle` on the accepted
  transition (`was_checking`), guarded only by `!self.station.is_empty()`.
- `session_answer` offers it **once per connection and ahead of** the stored
  sign-in; `answerable` also re-offers after a *"come back"*, which is not a
  refusal (`is_auth_busy`).
- `multi.rs:1359` polls `poll_auth` for **hidden** tabs too, precisely so the
  tabs behind the one on screen can let themselves in.

**So the two live suspects are both "the key was empty or wrong when the answer
was accepted",** which the `!self.station.is_empty()` guard then *silently
skips* — a tab whose `ctrl.peer_url()` is `None` yields
`station_key() == String::new()` (`app/mod.rs:2233`), nothing is recorded, and
every following tab finds an empty map and draws the card. That is one line of
guard doing the damage quietly, which is the shape of bug this file keeps
finding. The other is that the prompt is **not ours at all**: "popup warning"
is what Chrome's password manager says, and a card we fill silently could be
provoking it once per connection.

**Ask, do not guess** — one message, and he answers well and attaches
screenshots: is the prompt **our** sign-in card (Username / Password / REMEMBER /
SIGN IN) or a **Chrome popup** ("Save password?"); does ticking **remember**
make it stop for good; and does it appear at start-up as the three tabs open, or
later. That answers both suspects at once, because a Chrome popup that survives
*remember* is not ours and ours that survives it is the empty-key path.

**Also in that comment, and unanswered:** *"Pluto is still using 100% of the CPU.
This bug has never been fixed in any version."* He has the thread to it — the
Tezuka firmware **0.3.23** release adds an `sdroyxde` package — and he offered
to test it. Our side has not measured it, and `sdroxide-adsb` at a steady 99.9 %
is a single-thread bottleneck in *our* decoder, so it is ours to check before it
is the firmware's.

### Built (2026-10-07, later): **the sign-in cookie** — one password, one station

The operator's question, and the right shape for it: *"can't we make a cookie
that keeps the login open? … the user has to accept a 12 or 24h cookie."*
**Built**, and it needs **no `PROTO_VERSION` bump** — which was not obvious in
advance and is the reason the design below is the one it is.

**Why HTTP and not a socket message.** A cookie is set with a response header,
and axum's WebSocket upgrade spends its response headers *before*
`auth::challenge` has read a password — so a card on the socket cannot give one
out, however it is asked for. Hence `POST /signin` over plain HTTP
(`crates/sdroxide-server/src/signin.rs`): the client posts the credentials once
when the operator presses SIGN IN, the station answers `Set-Cookie`, and every
connection after it — every radio, `/solar-ws`, every reload — carries the cookie
on its own upgrade and skips `challenge()` entirely (`auth::challenge` grew an
`already` argument; the three routes read the cookie from `HeaderMap`).

**No new secret file, and that is the load-bearing decision.** The signing key is
derived from the credentials the server was *already* configured with
(`session_cookie::key`). So: nothing new to store, back up or lose, and
**changing the password invalidates every cookie at once** — which is what an
operator who changes their password means, and what a separate stored secret
would have to be taught to do anyway. A station with no password configured has
no key, so this can never be a way in the password was not.

**HMAC-SHA256 written out** over `sha2` (already in the tree via rustls) with a
`subtle` comparison. `hmac` is not in `Cargo.lock`, so adding it would have meant
a fetch; eight lines against a crate the build already has is the better trade,
and the construction is fixed by RFC 2104.

**Two bugs the tests caught, both worth keeping:**

- **SIGN OUT would have signed you in for twelve hours.** `clear_cookie` routed
  through `set_cookie`, which *clamps* `hours` — and `0` clamps to the twelve-hour
  dwell. So the button meant to end a session handed back one lasting half a day.
  Caught by a test asserting what the string actually says, not what it was meant
  to. `clear_cookie` is now written out on its own with `Max-Age=0`.
- **The dwell was the default, not an override.** `settle` runs every frame, so
  setting `self.dwell` there put the twelve-hour default back over an operator
  who had just tapped **1 DAY** — silently halving what they chose. Now guarded
  on the station actually changing.

**One test failed with the report's own words**, which is the best possible
confirmation: `one_sign_in_covers_every_radio_on_the_station` — three radios, one
`/signin`, all three connections — panics on the unfixed code with
`/ws asked for a password it already had: AuthRequired`. That is Kevin's bug, not
a paraphrase of it. Beside it, the four things that must keep asking:
`without_a_cookie_it_still_asks`, `a_wrong_password_buys_no_cookie`,
`signing_out_expires_the_cookie`, `the_cookie_carries_the_dwell_that_was_asked_for`
(five ports, 39481–39485, because the turnstile is server-wide state).

**Two things the client half had to get right, both now unit-tested on native:**

- **The HTTP address.** `signin_url` turns `wss://host` into `https://host/signin`,
  and **keeps a reverse-proxy path prefix** — `wss://host/shack` must ask
  `https://host/shack/signin`, or the password goes to the proxy root, whose 404
  reads to the operator as a station refusing a correct password. My first test
  expected the prefix to be stripped; the code was right and the test was wrong,
  which is why the prefix case is pinned now.
- **`HttpOnly` means the page cannot sign itself out**, so **SIGN OUT** is an
  endpoint. It lives on the new **Settings → General → Signed-in station** row
  (drawn whenever `peer_url()` is `Some`, so the browser — where the cookie is
  the whole mechanism — is the case it was designed for) and says what it does:
  the station forgets this device; **the connection you are on stays up**.

**The fallback, which is the part that keeps an old station working.** A station
too old to answer `/signin` would break the card's promise, so the page falls back
to the old password store and **names that station** on the settings row rather
than holding a password quietly (`cookie_fallback_stations`). Native is untouched:
no cookie store exists, so **Remember on this device** and `remote_login.json`
remain, and SIGN OUT clears it.

**It masks a bug we have still not found.** The report's two live suspects were
the empty station key (which `!self.station.is_empty()` then skips in silence)
and Chrome's save-password popup. The cookie removes the password field that
would cause the popup, and covers the other — so **the empty-key guard is still
unfixed and still worth fixing.** A feature that quietly covers an unexplained
report is how the next one gets misread.

**Honest limits.** Same-origin only in practice (fine behind caddy, which is one
host; a client on another origin needs `SameSite=None; Secure` and runs into
Chrome's third-party-cookie rules). Revocation is expiry or a password change —
no device list, no sign-out-everywhere, because that is a server-side session
store. **Not driven in a browser here**: the server half is end-to-end tested,
the client's pure half is unit-tested, the `fetch` and a real cookie on a phone
are not. Told to Kevin as "next release, I will post when it is on a
pre-release" — fork discussion #16, comment `18797670`.

### Narrowing the two open items — the steps, so this is not re-derived

**Item 3, the 3-radio tab-close flicker** (fork discussion #16, Kevin; browser
only, server unaffected, two tabs fine and three flickering). Cheapest and most
decisive first, and **step 1 needs a human** because it splits the three
possible causes in one question:

1. ~~**Ask him three things**~~ **ANSWERED (2026-10-07, comment `18796013`).**
   - *"The flashing continues Radio 3 / Radio 1"* — so it is the strip he
     described and **2.0.0's scrollable strip did not change it**. He has not
     tried 2.0.1 yet ("I'll download version 2.0.x tomorrow morning").
   - *"I haven't tried it"* (the 30 s question) — **so "a countdown settles, a
     redraw loop never does" is still open.**
   - *"There is no console in Chrome on Android"* — and he is right, there is not;
     a desktop-client reproduction is the only route to that evidence, so do not
     ask a phone user for it again.
   - **"some phones it crashes, others it doesn't"** — new, and it splits the
     three causes again: a *crash* is not a flicker. Treat "it crashes on some
     phones" as its own item and ask which.
   - Always radio 3 closed while viewing radio 1; he re-confirmed the pairing.

   **Still worth asking once, cheaply:** whether the tab *reappears* (count
   3 → 2 → 3) or only the screen flashes. He answered "the flashing continues"
   without separating them, and that one question is what splits step 3 from
   step 4. Ask it with the 2.0.1 result in hand.
2. **Reproduce without a browser.** The multi-radio harness now exists
   (`eframe::CreationContext::_new_kittest`). Build a `MultiApp` with
   `remote: Some(factory)` and three tabs, close one, then run ~120 frames on a
   stepping clock and assert `tabs.len()` stays 2. If it flaps, the bug is
   caught here and step 1 was only a nicety.
3. **If the count flaps**, it is the peer-reopen path. Instrument
   `open_peer_radios` (`multi.rs:~1064`) to log `wanted` and `peers_opened` per
   frame. **The suspect is that tabs created at startup from the server roster
   never enter `peers_opened`** — only peers opened *by that function* do — so a
   closed one is offered again. Note `peers_opened` is *deliberately* never
   cleared on close (see its doc), so the fix is to seed it for the initial tabs
   or to keep an explicit "the operator closed this" set. **That is a design
   decision, so measure before choosing.**
4. **If the count does NOT flap**, it is a repaint storm, and there are only two
   per-frame repaint requests in the shell: `multi.rs:1372` (`poll_auth` →
   `after_ms(120)`) and `multi.rs:1397` (`poll_reconnect` → `after_ms(250)`).
   Log which one fires and **on which tab**. `poll_reconnect` redials on a
   countdown, so a banner flapping every 250 ms is literally a
   "disconnect/reconnect loop" — and the question is then why closing tab 3
   dropped *another* tab's link (look at `shutdown_ctrl` and anything shared:
   the spot feed, the station's one connection).
5. **Check the `while` loop at `multi.rs:1051`** — it closes every tab whose
   `peer_removed()` is true and posts a notice each time. Log `peer_removed()`
   per tab per frame around the close: if a *remaining* tab reports it, the loop
   closes and re-notices, which is churn with no reopen involved.
6. Whichever it is, **the step-2 test is the guard** — land it with the fix.

**Item 1b, the 2 pt page overflow — FIXED, and the bisect below was looking in the
wrong place** (`#16`, present with *one* radio at 360 and 1920, so not the strip:
the page's background `Rect` is 0..362 in a 360 window and its right border is
therefore painted off-screen). **Do not use `sed` on `engine.rs`/`top_bar.rs` for
any of this** — a bare indentation-anchored pattern cost a day's work today by
rewriting an unrelated `1.0` in the TUNE path.

**What actually happened, because the steps below are wrong and that is worth
keeping.** Steps 1 and 2 were both run and both came back clean: the strip
planners sum correctly at every width, and instrumenting `angled_frame` showed
every panel's *content* comfortably inside the window — 8..352 pt in a 360 pt one.
Step 3, bisecting a row, would have found nothing, because no row was too wide.

The recorder was still the right idea, only the question was slightly wrong. It
asked what a panel was *pinned* to; the thing that needed asking was what the
panel *painted*, and the answer was egui's own: **`egui::Panel`'s frame fill
reaches ~2 pt past the window's right edge at every size**, regardless of what is
inside it. That is the top bar's background, which is why the border was missing
on two different panels and in both a phone and a desktop browser — one paint,
everywhere.

**The fix, and it is one place.** The band is now painted from inside the panel,
through a painter clipped to the window, rather than by the panel's frame; the
frame keeps its inner margin and carries no fill. Its height is only known after
layout, so it uses the paint-slot trick [`angled_frame`] already uses for the
gradient fill — reserve a `Shape::Noop`, set it once the panel's rect is known.

**Two corrections to the oracle itself, both of which were reporting things that
were not faults:**

- It counted **fully transparent** rects. egui keeps painting transparent ones — a
  frame with no fill, a reserved slot — and one of them overhung at every size, so
  the oracle called it ink that no operator could ever see. It now asks for
  **ink, not geometry**: a rect with a transparent fill and no stroke is skipped.
- The sweep's **6 pt tolerance** existed only to clear this one 2–4 pt remainder,
  so it was letting through a fault that was never the strip's. It is now **0.5**,
  which is what a hairline stroke spends, and `nothing_is_drawn_wider_than_the_window`
  is **un-ignored and passing** — it was written for exactly this moment.

**The lesson, and it is the same one as the CTR/SSTV note:** instrument the thing
that knows the arithmetic, not the thing that merely looks wrong, and check
whether a reported fault is *visible* before chasing its geometry. Kevin's report
was precise — the outline extends beyond the edge — and taken literally it is not a
row at all.

1. **Turn it into a pure-function assertion before touching any drawing.**
   `plan_short_strip` (`top_bar.rs:309`) is pure and already has a test helper
   (`a_short_strip`, `top_bar.rs:8752`). Add a sweep: for `avail` from 320 to
   1920, assert `box_w + ptt_w + gaps + grid_w <= avail`. **If that fails, the
   arithmetic is the bug** and no egui knowledge is needed. Do the same for the
   desktop planner, since 1920 is a different code path (+4 there, +2 at 360 —
   do not assume one fix covers both).
2. **If the plans sum correctly**, the overflow is inside a chip. Instrument
   `angled_frame` (`chrome.rs:29`, test-only) to compare `ui.min_rect().width()`
   *after* `add(ui)` against the `w` it pinned. That says row or child.
3. **Then bisect the row** by drawing it with pieces removed and watching the
   overshoot — `what_paints_past_the_edge` (`multi.rs`) already prints the widest
   shapes, so each bisect is one run.
4. **The acceptance test already exists and is ignored**:
   `nothing_is_drawn_wider_than_the_window` (`multi.rs`). Un-ignore it with the
   fix; the active `the_strip_does_not_push_the_page_past_the_window` is the
   6 pt-tolerance guard for the class that is done.

**Deferred to the next session, deliberately:** the **DAB MOT slideshow** (the
station images the ensemble broadcasts). Everything needed is already in place —
`dabradio` decodes it (`PadExtractor::extract_all_from_au` → `PadData::Mot`), the
Access Units are the same ones `take_pcm` hands to faad2, `image` is already a UI
dependency with png+jpeg, and the SSTV pane has the bytes→texture pattern. It was
left because it is **not tiny**: it wants its **own** wire pair rather than a
field on `DabStatus` (that snapshot is sent several times a second and an image
is tens of kilobytes), and it **cannot be verified without a DAB+ capture whose
service actually broadcasts a slideshow** — the Nancy capture is the candidate
and `dash-lab` is not on this box. The DLS (station text) comes from the same
call and belongs in the same right-hand column.

## The standing queue and how to check it (re-measured 2026-10-06)

**Our open upstream PRs are the whole queue.** Track only these. Everything
else on the upstream tracker belongs to the maintainer — do not triage or reply
to new upstream issues we have no PR for.

**Eight open, re-measured 2026-10-06 — and five of the eight are `CONFLICTING`.**
The earlier "nineteen open, all `CLEAN` and mergeable" is stale in both halves.

| PR | | state | draft |
|---|---|---|---|
| **#626** | CW: iambic keyer on the engine, sending contacts | **CONFLICTING** | draft |
| **#597** | Digi: JTTY | **CONFLICTING** | ready |
| **#554** | UVPacket (draft) | **CONFLICTING** | draft |
| **#545** | (tr)uSDX nG (draft) | **CONFLICTING** | draft |
| **#537** | Band openings | **CONFLICTING** | ready |
| **#598** | 2G ALE (draft) | MERGEABLE | draft |
| **#559** | band-menu captions | MERGEABLE | ready |
| **#557** | recording silence split | MERGEABLE | ready |

**Eight of the queue have landed upstream and are already in our `main`** —
verified by merge-commit ancestry (`git merge-base --is-ancestor <merge_sha>
main`), *not* by the divergence count, which reads 0 and would have hidden it
entirely: **#613** grid tracker (2026-10-01) · **#572** CW key as audio (10-01) ·
**#568** Morse trainer (10-01) · **#561** FSK441 TX (10-01) · **#611** NAVTEX
AFC (10-03) · **#612** LimeSDR Mini rates (10-03) · **#604** FT8 decode depth
(10-03) · **#586** FT8 signal subtraction (10-03). **So every "the fork's copy
drops out when these land" note below is now spent, not pending.** #603 and #569
closed unmerged, which is what the notes already said of both (withdrawn;
superseded by #626).

**No maintainer ask is waiting on us.** All four open PRs carrying comments were
answered and his last word on each is a confirmation: **#537** (2026-09-28)
*"Both logic flaws fixed in `d9612883` — and thanks, they were real"*; **#557**
(09-28) the hidden-tab gate tick, addressed in `c9d6589c`; **#626** *"The shape
is right: the keyer belongs in the engine, contacts are the right thing to send
rather than edges"*; **#559** carries only third parties (phsdv, kevin2008-01).
Our upstream **#621** (Olivia) has **0 comments** four days on — per the standing
rule, silence is not a prompt to re-ping.

**#626 is rebased and `MERGEABLE` (2026-10-06), and its review is NOT answered.**
Branch `upstream-pr/cw-key-engine` is now `d74bae94`, force-pushed with
`--force-with-lease`. Three conflicts, all append-at-tail, and one of them was
a **version collision**: upstream's own #604 had already taken
`PROTO_VERSION` 173, so the contribution moves to **174** with the `v173` history
note kept above it. `cw_key_mode` sits after `ft8_depth`, not before it.
`cargo fmt` clean; the four affected crates green.

**All six of the maintainer's blockers are FIXED on `main` (2026-10-06)** — and
that ordering is the point. They were not upstream chores: **every one of the six
was a live bug on our own `main` too**, which is only visible if you check our
tree rather than the PR. Six commits, `53a184f2` … `ec0a4db8`, every test
verified to fail against the broken version.

| his wording | what it actually was, here |
|---|---|
| buffers grow unbounded on the audio thread | worse: `CwKeyer::out` was **never drained by anything** — `take_text` is called only from tests, so every keyed character pushed onto a queue no code path could pop, for as long as the keyer stayed armed |
| a press from idle never starts an over | `set_cw_contacts` had **no test at all** — it appeared only in its definition and the trait's default. Every keyer test drove `fill_tx_block`, which is exactly the path that cannot show this |
| nothing releases a held paddle on disconnect | `CwKey` is a down *edge* and the keyer is driven by *closed contacts*; releasing one never released the other, and the lost-key-up cap deliberately does not apply to a keyer — so a departed client keyed a carrier by itself |
| the keyer is never disarmed | the keyer is not a mode of its own, so a keyer left standing with `straight` off meant **no** hand-keying at all; the cure was changing mode and back |
| mode B loses the trailing element on a squeeze release | mode B's memory is a claim about the **falling** edge, and `contact` latched rising edges only |
| a tap is latched only in `poll` | a press is an **event** and `poll` is a **sampler**; a tap inside one engine loop was sampled only in its released state |

**Two of his six were misread as "disarm the keyer", and disarming is wrong.**
An abort must **release the contacts**, not disarm: a keyer is a setting, not a
transmission, and `abort_tx`'s own comment records that disengaging the mode on
abort is what left the chip lit, the transmit box locked and Space dead.
Disarming is reserved for `set_straight(false)` and a config change, which *are*
the operator saying they are done hand-keying.

**There is no reference program, and that is worth knowing before the next
search.** Checked for how the others do it: **fldigi has no software iambic
keyer at all** (external Winkeyer / ICOM / FT991 / NanoIO only — very likely why
he wanted ours engine-side, there was nothing on his side to copy), **JS8Call has
none**, and **flrig's "CW keyer" is a DTR/RTS keyline**, not a paddle keyer.
Every "iambic keyer" repo on GitHub is an Arduino/Pico toy with no transmit
lifecycle. So these six are his engineering judgement, not conformance to a
convention, and the design was ours to set from the start.

**A lesson from the squeeze test, because the first version of it was green for
the wrong reason.** Squeezing *from idle* cannot tell mode A from mode B: the
press latches the dah either way, so the trailing dah goes out even with no
memory at all. It only bites once the latch is **consumed first** — a dah on its
own, a character gap to flush it, then the squeeze and its mid-element release.
The same trap as the Olivia polarity: a test that passes before and after a fix
is testing nothing, and the only way to know is to remove the fix and watch it
fail.

**The other four conflicting PRs (#597, #554, #545, #537) are deliberately left
alone.** With the maintainer silent, a rebase is a force-push that changes
`updatedAt` and shows up as activity on a queue nobody is reading. #626 earned it
because it carries functionality already shipped on our `main`; a JTTY or UVPacket
draft does not, and re-pushing those would be adding to the backlog faster than it
drains. **Revisit only when he answers something.**

**A trap hit on the way, worth keeping.** The rebase resolved three conflicts by
hand and the resolutions were **not committed before the force-push** — the branch
went up with two doc-comment lines at column 0 and a `pub use` list wrapped the way
I typed it. `cargo fmt --check` is what CI runs, so that was a broken push, caught
only by re-checking the *remote* content afterwards rather than the working tree.
Fixed in a follow-up commit and re-pushed. **The house rule "compile the committed
tree, not just the working one" is not only about merges** — it is about anything
you are about to publish.

**Upstream issues to read** are only those we have a PR for: #608→#611,
#609→#612, #585→#613 — **all three now merged**, so that list is empty. We are
also engaged with, but have no PR yet: **#577** K3 I.F. panadapter (diagnosis
posted, awaiting the reporter), **#576** IC-7851 RTTY (awaiting rig details),
**#595** Perseus SDR (needs a scope decision), **#592** Windows (blocked on a
failing DLL name or a Windows/Radeon repro).

**Each session, in order:**

1. `git fetch upstream` — **fetch only; never merge it (see the guard at the top
   of this file).**
2. Read the maintainer's comments on the open PRs above. Reply only where he
   asked something — silence is not a prompt to re-ping.
3. Check the **fork's issues AND Discussions** — both are open, and a report
   arrives as an issue more often than as a discussion. Downloads are growing,
   so expect new threads and answer them rather than opening an upstream issue.
4. New upstream issues: look only at ones we have not seen, or ones we already
   have a PR connected to. Skip the rest without comment.

**Fork Discussions — re-counted 2026-10-15: twenty-four threads, and the forum
already exists.**

**The fork's ISSUES are open too, and an earlier note here said they were not.**
It said *"Issues are disabled on the fork"*, which cost an afternoon: an agent
read it, told the operator there was no support queue, and answered nobody.
Verified with `gh repo view … --json hasIssuesEnabled` → **`true`**. Re-check it
rather than trusting any number written here.

**Check both, every session.** Issues are where reports arrive — a user opens
one without thinking about which tab. Discussions are where the longer threads
live. Re-count rather than trusting any number written here.

**Open issues as of 2026-10-12** (re-measured; treat the list as perishable):

| # | who | what | state |
|---|---|---|---|
| **#72** | drdatapi | RTL-SDR start frequency on QO-100 — an I.F. offset saved as the start frequency, so the dongle was asked for 6 GHz and did not open | **answered and fixed** (`ba565a56`): `set_center` refuses a centre above the tuner's ceiling and names it. The *saving* of an impossible start frequency is **not** fixed — asked of the reporter |
| **#69** | bebepip | Import a broadcast schedule other than EiBi (Aoki / HFCC / own CSV) | **answered, not built.** The parser is hard-wired to EiBi's column layout and code tables; the display side is already source-neutral. Three questions put to the reporter (which source, replace or sit alongside, where entries appear). Called a real job, not promised as quick |
| **#68** | bebepip | S-meter uncalibrated — S9+10 on pure noise | **answered** (`f45b00d9`): only the SDRplay and Pluto implemented `rx_gain_db`, so an RTL-SDR's whole tuner gain was added to the reading. **Withdrawn 2026-10-15, then RESTORED 2026-10-16** — and the round trip is the lesson. Kevin asked for it to be removed, having established the reading behind it came from the *wrong build*. But **the correction was never wrong, the range was**: ±5 had been widened to ±15 on that bad reading. He asked for his first revision back — PR **#74**'s, `27b79ed3` — so the calibration is **in and at ±5** (`6f95a06f`), which is what he says worked perfectly. **No `PROTO_VERSION` bump**: the field was never taken off the wire, so the layout is byte-identical and the revert only put it back to work. Upstream has its own at #642. |
| **#51** | drdatapi | DELETE button for recorded MP3s | answered — the **CLEAR** chip in the playback window |
| **#28** | dig647 | Channel spacing, and DMR | spacing answered (6.25 / 12.5 kHz added). **DMR re-answered 2026-10-12**: Kevin's "the codec is a big project" was right in its conclusion and wrong in its reason. The codec is `mbelib-neo` (GPL-2.0-or-later, compatible, no licence wall), and the blocker is **US Patent 8,359,197**, which covers the AMBE+2 variant DMR uses until **2028-05-20**. A clean Rust crate exists for D-STAR and declines DMR for exactly that reason |

**#72 is the shape worth keeping.** Reported by somebody who had never seen this
code, and it was one line that had been assumed correct for months: his *number*
named the mechanism — 6 GHz is a fixed multiple of a 1.7 GHz ceiling — and it
matched a specific line once read.

**#68 is the shape worth keeping in the other direction.** The reading was
real, the arithmetic was right, and the *premise* was wrong — the reporter's
figures came from a build that was not the one being fixed. **Ask which build a
number came from before building anything on it**, which is the same rule as
"ask which build he is on before answering" and the reason the issue form now
requires a version.

- **#4** "Settings saved to the saved profile" (kevin2008-01, 22 comments) —
  the client-screen feature, which **is fixed** (see the store section below).
  Kevin reports it *"Persistant on 1.9.18"* and asks for key/mouse/page
  assignments to be saveable per profile — which already exists behind
  Settings → UI → "Screen settings on" = **On the server** plus the bindings
  opt-in. **Both his reports are on a release that predates the fix; 1.9.19
  carries it.** Needs a reply that says which build to run, not "fixed".
- **#5** "Olivia decoding" (10 comments) — **resolved, and it is the best
  evidence in the fork**: Kevin sent a known-text capture our decoder reads
  correctly (§3). Worth keeping the thread for that reason alone.
- **#8** "FT8/FT4/FT2 problem 1.9.18" (6) — Kevin closed it himself:
  *"Thx fixed. Tried it on another machine, it works fine."* No action.
- **#9** "UI phone problem" (3) — Kevin: *"Persistant on 1.9.18"* with a fresh
  screenshot. Same trap as #4: the responsive phone menu is in 1.9.19. **The
  phone *crash* is still open and must never be described as fixed** — ask for
  the browser console output if it recurs on 1.9.19.
- **#7** SSTV relay (5) — answered: the **Re-upload** chip shipped in 1.9.19.
- **#16 "V 2.0.0 Full Report"** (kevin2008-01, the longest thread in the fork)
  — **the support channel's most important thread, and the one to read first.**
  He works a full checklist in one pass against three of his own radios (PlutoSDR,
  HackRF, RTL-SDR on one Debian 13 box behind Caddy, from Chrome on Android as an
  installed PWA and from a desktop browser) and separates fixed from broken rather
  than rounding it up. Fixed and confirmed by him: the profile screen saving
  itself, Ctrl+000 defaults, the bindings-adoption prompt, the settings-window
  overlap, image save on Android, the PWA install, the two-radio desktop layout.
  Open from it: the **sign-in per radio** (fixed by the cookie in 2.0.2, not yet
  verified by him), the phone's missing right border, the three-radio tab-close
  flicker, and the Pluto/ADS-B thread at 99.9 %. He answers well and attaches
  screenshots — ask precisely, and believe him over a recollection.
- **#17 "Liste des fichiers installés sur un serveur"** (kevin2008-01, Ideas, 0
  comments) — asked for **a complete list of the files an installation puts on
  the machine**, so he could uninstall completely, then reported **FT8 decoding
  nothing at all on the fresh install**. **Both answered** in comment `18800248`.
  The list came out of `Cargo.toml`'s `[package.metadata.deb] assets`, the nine
  `packaging/linux/60-sdroxide-*.rules`, the `PKGBUILD` and
  `install-desktop-entry.sh` — and the three things worth keeping from it are
  ones nobody had written down: **the Debian package installs no systemd service**
  (so `apt purge` cannot leave one running — his `/etc/systemd/system/sdroxide.service`
  came from his own **sdrversion** script, not from us), **nothing is written to
  `/etc` or `/var`**, and the **installer creates no configuration** — the program
  makes `~/.config/sdroxide-brown` on first run, which is the other half of a
  complete removal. Now in the README's Installing section and manual §9.5, so
  the thread cannot be the only copy. **His FT8 was not a decoder fault**: a fresh
  install has no `radios.json`/`radio.json`/`radio-N/`, so nothing is listening —
  and he installed **2.0.1**, not the 2.0.2 he now wants. **Ask a fresh-install
  reporter "is the waterfall alive?" before anything else.**
- **#14 ATS Mini / RadioScript** (kevin2008-01, Ideas, 0 comments) — **on
  research and NOT OURS TO BUILD**: hjberndt's RadioScript firmware is closed
  source with **no licence**, so nothing may be copied from it or reimplemented,
  and the operator rules that way. It is also the most popular firmware for those
  boards, which is why expect this question often. He reverse-engineered a third
  ESP32/Si4732 firmware (hjberndt's RadioScript) and attached it; four findings
  out of the image are in that section, the important one being **a TCP server on
  8081**, which turns his missing bridge script into ~15 lines of BASIC on the
  radio and answers his *"does a script need an open browser tab?"* (`/run`).
  **We already support this class** — `Backend::AtsMini` (MIT) speaks the *stock*
  firmware, so the prior question is which firmware his board runs. Answered in
  comment `18800333`, and told plainly that the board is not an SDR and that the
  only licence-clean path is a **documented** interface from the developer's own
  side. The firmware binary must never be committed.
- **#13** non-upstream build targets (armhf &c) — **the armv7 build now exists**
  and is attached to `v2.0.1_brown` as
  `sdroxide-v2.0.1_brown-linux-armv7-cat.tar.gz`; it has not yet run on his
  hardware, and the probe stays out of the release matrix until it has.
  **#12** AI-assisted upstream dependency PRs · **#11** ESP32 as an RX ·
  **#10** replay recorded audio.
- **#3** "Over heating" — resolved (another program's SoapySDR, not us).
  **#6** "My help for testing" · **#2** welcome thread.

**The trap worth naming, because it is the same one as the QSL card and #573.**
Kevin says "persistant" against a version, and the honest answer is which build
carries the fix. Telling a user his report is fixed when he is running the
release *before* the fix teaches him that our "fixed" is worth nothing — and he
has already had to say "persistant" twice. **Ask which build he is on before
answering, and name the tag that carries the change.**


## Session 2026-10-15: the site, Bluetooth audio, and a BETA weather decoder

Five things, and the two that change how work is done come first. **Everything
below is on `main` and in no release** — `2.0.12` predates all of it.

### Two programs, one executable name — and the site is the fix

The fork's repository homepage was set to **upstream's releases**. Anyone
arriving at `sdroxide-brown` was handed a link to somebody else's downloads,
which is a large part of why people mix the builds up. It now points at
**`sdrbrown.info`**.

- The site is **GitHub Pages from `/docs`** (`docs/index.html`, `docs/style.css`,
  `docs/CNAME`), because the Pages API accepts only `/` or `/docs`. A bonus: the
  manual is now browsable on the site.
- **The page's one job is identity**, which is why it is short where upstream's
  is enormous: what Brown is, `sdroxide --version` and what `_brown` means, the
  differences, the download, and the Discussions link.
- **The forum already existed** — Discussions, 24 threads — and was simply
  invisible. No third-party service was needed; nothing pointed a newcomer at it.
- DNS is four A records at GitHub plus a `www` CNAME, and the wildcard `*.`
  record that shipped with the domain **had to go**: it catches every name under
  the domain and silently keeps serving the registrar's parking page.
- `hasIssuesEnabled` is **true** and so are Discussions. An earlier note here
  said issues were disabled and it cost a session.

### Bluetooth audio: cpal cannot open a PipeWire node, and that is not the bug

The Midland wireless mic, asked for and delivered.

- **Pairing attaches no audio profile.** `bluetoothctl devices Connected` showed
  the mic and there was still no node until `bluetoothctl connect <mac>`. This
  cost a run and would cost an operator the same.
- **ALSA cannot see a PipeWire graph.** It exposes a `pipewire` and a `pulse`
  PCM and nothing behind it, so `bluez_input.…` was never listed however long
  anyone looked. `crates/sdroxide-audio/src/graph.rs` lists the sound server's
  own nodes by name and **only those ALSA cannot already show**, so a sound card
  is never offered twice under two names.
- **Opening a node by name does not work, and that inverts the obvious design.**
  `pulse:bluez_input.…` is accepted by ALSA — verified, it captured the Midland
  — but **cpal can only open what the host enumerated or the host's default**,
  and **PipeWire has no per-application capture source** (playback has a
  per-stream target, capture does not). So picking a node runs `wpctl
  set-default`, which takes over the machine's capture, and **the warning says so
  in the settings row** rather than only in the log.
- **The warning's predicate was wrong the first time.** It asked
  `is_graph_node_name`, which matches `alsa_input.` — and the ordinary analog
  card is called `alsa_input.pci-0000_06_00.6.analog-stereo`, so every sound
  card warned that it takes over the machine's recording. A confident false
  warning is worse than none. The condition is **membership of the extra list**,
  which is what the engine actually does, so the two cannot drift.
- `wpctl get-default` **does not exist** in the version here; it prints its help,
  so the undo line came out empty every time. `pactl get-default-source` answers.

### Reading a published table, and the three wrong "fixes"

The SYNOP/SHIP work, and this is the part worth carrying.

- Real example messages exist **online** — Hong Kong Observatory's ship code
  sheet, CTOM Oman's worked group-by-group decoding, tutiempo.net, AWI. I told
  the operator to go outside and make a recording, which was wrong and cost him
  a trip he could not make.
- **Testing against a fixture I invented is how this decoder lost two correct
  readings.** It passed on a message I made up and was wrong in six ways.
- **When a source disagrees with the code, check a second source before changing
  the code.** I confidently "fixed" `iw` to two figures and `QcLoLoLoLo` to nine,
  and **both were already right** — one source had merely spaced the
  specification differently. Two independent examples said so.
- **Five-digit groups are told apart only by position.** `iRixhVV` is *five*
  characters, and a length-based dispatcher read a visibility group as a
  barometer of 169.2 hPa. The decoder is a **walk with a cursor**, not a table.
- **A slash is a value** — `8597/`, `43///`, `8/000` are "not reported", which is
  neither zero nor absent.
- **Three of my own test assertions were wrong and the code was right.** Fixed
  the tests. A test that fails because the test is wrong is a test to fix, not
  the code to bend.

### Upstream: take commits, never the branch

Upstream had 69 commits we lacked and is at `PROTO_VERSION 179` against our 210.
**Taken by hand:** the window title (#664, both commits — and ours keeps
`FLAVOR`, so a title that said "sdroxide" would have defeated the point of it),
and PSTRotator (#638), integrated as a fourth arm of **our** `RotatorTransport`
rather than upstream's parallel enum, which would have left `RotatorConfig` with
two fields both answering "how is the antenna reached".

**Skipped: FT8 streaming decodes (#667) — we already have it**, byte-identical
apart from a path qualification, and our `apply_armed_qsy` is a superset. Taking
it would have been an empty commit wearing their message.

**#648, #660, #661, #663 all merged upstream.** Our queue there is empty.

**#537 was reported, answered, and then fixed the same day.** The maintainer:
*"the warm-up fix is only called from tests … coming back to a band, or mixing
DX-cluster spots with PSK Reporter still shows false OPEN alerts."* **He was
right, and the fault was worse than a large factor — it was infinite.**

`first_watched` is keyed by **path**, not by timestamp, so it outlives pruning.
A feed is only polled for the band the dial is on, so a path on a band you have
just come back to has had **no observation at all** while its clock still said
"warm for hours". Warm over nothing is the silent-baseline branch, which divides
by zero and returns `f64::INFINITY` — and the factor serialises as `None`,
because infinity is not a number a reader can be shown. The DX-cluster half is
the same clock from the other side: `ingest` was doing `or_insert(now)`, so a
path only the cluster ever mentions had its **first ever spot** set its watch
time, and "watched for hours" and "silent for hours" were one sentence.

Two changes: a spot no longer starts the clock (only `watch()` does), and the
manager tells the tracker when the dial moves
(`BandOpeningTracker::restart`). It costs a few minutes of quiet after each band
change, which is the honest trade.

**And the tests for it are the cautionary tale.** The first version of both
**passed on the broken code as well as the fixed one**, for three reasons in a
row: the module's own `spot()` helper stamps timestamps relative to a `NOW`
constant, so every fixture fell outside the retention window and *nothing was
ingested at all*; then one spot, where `min_distinct_calls` is five. **A test
that passes on the broken code is not a slow test — it is not a test**, and the
only way to know is to remove the fix and watch it fail.

### Two structural bugs, and why no test could see either

The audio stream (#594) had been **built and "socket-tested"** for days, with
three tests pinning its format. It sent **nothing at all**, and it took the
operator trying it to find out — *"absolutely nothing"*, an empty file.

**Two bugs, stacked, and both structural rather than arithmetic:**

1. **The row was unreachable on every SDR.** It lived inside the sound-card
   section, which is gated on `sound_card_roles` — and that answers `None` for
   every SDR in the list. So the setting existed, the engine honoured it, and
   nothing on screen could turn it on.
2. **The tap behind it was never filled unless REC was running.**
   `want_rec_main = self.recorder.is_some() && …` decides whether the receive
   chain fills its own recording copy — and the stream reads from exactly that
   copy. It was written as though the recorder were the only consumer. Zero bytes
   until you pressed record, with hover text that mentioned only volume and MUTE.

**Why the whole suite missed both, and this is the reusable part:**

- The **settings probe draws a sound-card radio on purpose** — its own comment
  says *"so the pickers and the typed field are in the picture"*. **A probe that
  only ever looks at one kind of radio cannot see what the other kind is
  missing.**
- A **format test over a socket** says the bytes are right when they are sent.
  It cannot say they are never sent. The three existing tests each called
  `UdpAudio::send` directly, so the one question that mattered — *does anything
  call it?* — was outside every one of them.

It is the same shape as `EngineConfig::default()` reading as "nothing special"
while actually being one of the two configurations the engine has. **A harness
that takes every default is testing the defaults, not the program** — and a
probe that only draws the easy case is testing the easy case.

Both now have guards verified failing on the old code:
`the_audio_stream_row_is_reachable_on_every_backend` walks SdrPlay, Cat, RtlSdr
and AtsMini, and `the_recording_copy_is_filled_for_both_of_its_consumers` pins
the small pure predicate rather than an engine built in a test.

### A contradiction in a report is the clue, not an obstacle

The operator's screenshot said SCHEDULE's ⇱ chip painted over LISTEN's body
*"while the rest of its title bar is correctly hidden underneath"*. Reading the
call sites said the opposite: `listen_window` at 1377, `schedule_window` at
1378, so SCHEDULE should be on top. **Both were right**, and the resolution was
the finding:

**The chip is an `egui::Area` ordered `Order::Foreground`**, which paints above
*every* window whatever order they are drawn in. So the **title bar** (ordinary
window order) was hidden by LISTEN and the **chip** (foreground) painted over
LISTEN's body anyway. One window, half hidden — no z-order or overflow theory
fits that, which is why the earlier readings, including three from the previous
session, were wrong.

Fixed by `Order::Middle` (`3f877e30`), and the guard is the **mechanism** rather
than the symptom: `a_tool_window_does_not_paint_above_the_window_stack` drives
two tool windows and asserts no layer above the window stack exists. *"Nothing
of the lower window shows through"* reads like the right assertion and needs a
real pointer and a compositor to mean anything; this fails the moment anyone
reaches for `Foreground` in a window again, and it names both chips when it does.

### Branch hygiene, measured

38 branches deleted — 3 merged locally, 35 merged remote — taking `.git` from
**417 MB to 339 MB**. What stays: every `upstream-pr/*` branch **still backing
an open PR**, per the house rule, and `claude/funny-wright-aujoii`, which holds
an unmerged DAB probe and is the operator's to decide on.

**2026-10-18, and the disk was the story rather than `.git`.** The operator asked
for the SDRuno branches out and "what else can go", so: `fork/sdruno-clone` and
`fork/sdruno-look` deleted (local and remote), the three `claude/*` branches whose
PRs are merged (#94, #96, #97), the stale `wt537` worktree pruned, reflogs expired
and `gc --prune=now` run — **`.git` 362 MB → 333 MB**. The remaining 209 MB of it
is `.git/modules`, the vendored submodules' own histories, which are left alone.

**`target/` was 999 GB** — `debug/` alone 615 GB of it, and `release/deps` held
**2839 linked executables at 266 GB**, most of them stale test binaries from older
builds that cargo never prunes. Deleted: `target/debug`, the extensionless
executables in `release/deps` (never the `.rlib`s or the shared libraries),
`release/examples`, `wasm32-unknown-unknown`, `doc`. **`target/` is now 106 GB**,
the gate is still silent, and the test binaries *relink* rather than recompile —
34 seconds for a package's tests. **What is left is `target/release`: 374 GB → 43
GB of rlibs + 38 GB of vendored-C build output.** The lesson for next time: the
number that grows is not the repository, it is the linked test binaries, and they
are the cheap thing to delete because relinking is seconds.

**Two branches are deliberately kept, and neither is an oversight.**
`fork/antenna-tool` holds **18 commits of an unmerged antenna-analysis tool** (a
NEC-style solver validated against a published OWA Yagi design, ground models, an
SVG pattern preview) — it has no AGENTS record at all, which is exactly why it
must not be deleted on a housekeeping pass: it is work, not history.
**The reference the operator sent for it (2026-10-19):
`https://summitradiodesigns.com/`** — a browser-only NEC2++ antenna studio
(KE2GYH). What is worth taking is the *approach*, because the operator's reason
for sending it is the thing to copy: **"people love it for its simplicity, even
a layman can use it."** Three things about how it does that:

* **One screen, two panes.** A setup form on the left, the result on the right,
  and three numbers across the top — feed height, peak gain, peak elevation.
  Nothing is behind a menu that a first-time visitor needs.
* **It says what it does not do.** The model notes are a `<details>` block
  naming the ground presets, the segment limits, and plainly that the checks are
  "numerical, not measured hardware validation" — the same discipline this
  project's handovers keep.
* **Browser-only, no account, no calculation server**, which is the offline rule
  in this file seen from the user's side.

Its palette is now the **Calmer** theme and a waterfall scheme of the same name.
**No licence is stated on the site**, so it is a reference for the interaction
and nothing else — no code, no solver, no assets.

`claude/funny-wright-aujoii` is remote-only, one commit (a probe that lists a DAB
capture's FIC data services) on a base 25,000 lines behind `main`. **Both stay, by
the operator's word on 2026-10-18: the DAB probe is Kevin's and is not to be
touched, and the antenna tool is still WIP.** A housekeeping pass that takes
either is a pass that has thrown away work.

### An issue template that cannot be ignored

We **had** a version field and it failed three ways: last in a list of six,
optional (a markdown template cannot require anything), and its example said
`v1.6.0` — **upstream's** scheme, so it taught people to answer with a version
that does not exist here. It is now a GitHub issue **form** where
`required: true` is real. Five fields, the rest optional.

## What this repository is

A fork of [sdroxide](https://github.com/dividebysandwich/sdroxide) tuned for the
**11 m citizens band**, **shortwave listening** and **decoding**. All live in
one program: the **CB band used to the full** — voice and the digital modes, a
two-way band that needs no licence for the CEPT channels in most countries, not
a receive-only extra — with its WSJT-CB interoperability and channel plans; and
the listener's tools — the broadcast schedule, the SWL log, time-shift replay,
scheduled recordings, ECSS, the receive tone and the scan bands. Upstream is the
original; everything here is upstream's program plus those additions. CB is a
first-class citizen of this fork, and CB and amateur radio are neighbours on the
same spectrum rather than rivals.

The listener work used to live in a listener-only fork,
`madmedicnl/sdroxide-swl`. It has been **retired**: merged into this fork and
archived on GitHub with a note pointing here. Everything is on `main` now.

## Next session (2026-09-28, later still): first, then what is left

**Read this section before anything else: the direction changed mid-session.**
New work is **fork-only** and no more upstream PRs are to be opened — see "Stop
opening upstream PRs" below. The queue is closed to additions.

**The 2026-09-28 merge took nine of ours (`90733eb2`).** Upstream moved 34
commits (`807b0fcf..9257c363`) and **merged #558, #575, #579, #580, #583, #588,
#590, #591 and #593**, each with the maintainer's own review commits on top.
**Eleven PRs are open**: #537, #545, #554, #557, #559, #561, #568, #569, #572,
#573 and #586 — the old list minus the nine that landed. Twenty files
conflicted; the resolutions are in the merge commit message, and three
are worth remembering because they are **decisions, not mechanics**:

1. **The spoken alerts diverge on purpose.** Upstream took #591 and then
   changed it twice (`aa3971de`): speech became **focused-only**, and a
   voice-only alert that cannot speak **falls back to its tone**. This fork
   keeps speaking **whether or not its window is in front** — a new DXCC that
   merely rings is the case the voice was added for. So `frame.rs` keeps the
   unfocused path, `AlertFired::sound` is **left out** (it existed only to ring
   that fallback, and there is no reader for it here), and `AlertRuntime::ring`
   goes **with it** — its only caller was the background-radio branch.
   `AlertCore::on_ft8` still plays the tone itself for a `Tone` or `Both`
   rule, so tone-only alerts are unaffected. We **did** take the transmit gag:
   `Announcer::on_alert` pushes `Priority::Notable`, not `Alert`, so a phrase
   cannot go out through the microphone. **Do not "fix" the divergence** — if
   it is ever revisited, it is a product decision for the operator, not a merge
   to reconcile.
2. **#575's band dock did not replace the fork's band menu.** Upstream docked
   *its own* simpler `band_mode_menu`, so the two are still different menus
   and ours stays. What was taken is the surrounding machinery:
   `band_dock_room` is now the single decision about whether there is room (so
   the chip and the panel cannot disagree), the column's width range is
   re-applied every frame, and a window too narrow **hides** the column rather
   than undocking it. `atsmini` and the fork's `band_menu_tab`/`band_filter`
   arguments are unchanged.
3. **The 3D window's saved position is no longer clamped** (`c51b70e5`).
   `monitor` is only a *size*, so clamping a desktop coordinate to `0..monitor`
   pulled a window left on a second monitor back onto the first. Our test
   asserted the clamp and was replaced by his second-monitor test. The fork's
   **owner cap and `keep_alive`** — the part #580 deliberately did not carry —
   are kept, along with the `emit()` that holds them and the `FLAVOR` title.

**#557 was behind and needed a rebase; it has had one (2026-09-28, later).**
#568 and #561 are still behind (the merge made them conflict; #545 and #554 were
already behind before it). Checked with
`git merge-tree --write-tree upstream/main <branch>`: #568 conflicts in
`panels/cw.rs`, #561 in `panels/mod.rs`. Both are the same files the main merge
just resolved, so the rebase is small. #557 (which conflicted in `app/mod.rs`,
`top_bar.rs` and the manual) is done — see the recording silence auto-split
section below for what it answered, and note that its corrections are on the
fork's own `main` as well.

**The maintainer is evaluating again (2026-09-28, latest), so rebase rather than
leave them behind.** The standing "do not push new commits onto the open ones"
restriction is **lifted for rebases onto current `upstream/main`** — that was its
only purpose, keeping a moving target from accumulating noise on a queue nobody
was reading. **New features still do not go upstream**: the direction is
fork-only for *new work*, and that is unchanged. When a PR branch is rebased,
say so in a comment on the PR so the maintainer is not surprised by a force-push,
and **check the thread for a maintainer comment first** — if he has already
reviewed the old head, the rebase may answer a point he raised, and the comment
should say which.

**Do first, every session.** `git fetch upstream` and merge if upstream has
moved, and read the open upstream PRs for maintainer comments (`gh pr list
--repo dividebysandwich/sdroxide --author madmedicnl --state open`, then each
thread). The queue was worked on 2026-09-27 with upstream still one commit ahead
(the CI-fix commit `807b0fcf`, merged as `ec16b72f`); the **fifteen** open PRs —
#537, #545, #554, #557, #558, #561, #568, #569, #572, #573, #575, #579/#580,
**#583**, **#586** and **#588** — had **nothing waiting on us** then: every
review point raised so far is addressed and the rest are drafts. Check before
starting anything new.

**Three more PRs opened (2026-09-28), and all three have since landed.** Upstream
was re-checked at `807b0fcf` and had not moved, and no maintainer comment had
arrived on any of the fifteen, so the session added **#590** (the REC popup's
**Quick clip** row), **#591** (a **spoken reply** per alert event, so a new DXCC
can be heard, not only rung) and **#593** (the grey line's contrast — shade the
continents too, and stronger) — **eighteen** open at that point. All are
behavioural/UX changes with no wire change; #590
and #591 were cherry-picked cleanly onto `upstream/main`, and #593 is the
upstream-shaped two-file version (upstream has no ADS-B/AIS/APRS/HFDL overlays).
The fork's `main` carries all three (`cf6dfde1`, `b38125d5`, `ac4811dc`), and
upstream merged all three in `90733eb2` — **eleven open now**. Note #591 landed
**in a changed form**; the spoken alerts are the fork's one deliberate
divergence, spelled out at the top of this section.

**Stop opening upstream PRs; new work is fork-only (2026-09-28, later).** This is
the operator's standing direction and it overrides the upstream-first rule in
"Keeping up with upstream" below: **do not open another upstream PR.** There was
already too much open
(#590, #591, #593 plus the fifteen older), and — the operative reason — **this
fork now serves a different user group**, so most of what this build adds is
*their* need rather than a general one and has no upstream audience to serve.
Only a genuine upstream *bug* found in the course of fork work is worth raising,
and even then raise it as an issue rather than a PR. Everything else goes on
fork `main`, which is the only repository to push to. **Still in force after the
2026-09-28 merge, which took #590, #591 and #593.**

**Amendment, same day: the "do not push to the open PRs" half is lifted.** It
was there to keep a queue nobody was reading from accumulating noise, and the
maintainer is evaluating the open PRs again, so leaving them behind their own
merge is now the wrong trade. **Rebasing a PR branch onto current
`upstream/main` is fine** — force-push it and leave a comment saying so. **New
features still do not go upstream**; that half stands, and a rebase must not
become a place to slip one in.

**Three more commits on fork `main` (2026-09-28, later), all fork-only:**

1. **`0a4033ab` — the SWL report pre-fills from the schedule.** The reception
   log's **+ NEW** entry form arrived blank where the operator already had the
   station in the EiBi schedule. `prefill_station` (`app/swl_log.rs`) now fills
   station, language, site, email and address from
   `broadcast::at_dial(&self.broadcast, freq, now_unix())` — the schedule entry
   *at the frequency*, scheduled first — and falls back to the logged station
   name. Pinned by `the_dial_pre_fills_from_the_schedule_then_the_log`. **Not
   offered upstream**: `swl_log.rs` is the fork's own file.
2. **`1e35b512` — an alert's spoken phrase is shown, and can be heard.** Choosing
   **Voice** / **Tone + voice** in Settings → Alerts was a leap of faith: nothing
   on screen said what would be said, and the only hint was small print pointing
   at another settings tab. Each event that speaks now shows
   `speaks: JA1ABC, new DXCC, Japan` beneath its row, built by the same pure
   `announce::alert::phrase` the announcer uses — so the displayed wording *is*
   the wording spoken — plus a **SAY** button that speaks it on demand. Gated on
   both the reply speaking and the announcement voice being on; when greyed it
   says which half is missing, so it never silently does nothing. The
   "switch it on in Settings → UI" note is gone, its job now done inline. This is
   the fork's discoverable version of #591 and stays here.
3. **`6025b2bc` — the last compiler warnings.** `cargo check --workspace
   --all-targets` is now silent; keep it that way, since a tree that warns trains
   everyone to read past warnings. Nine real ones, none behavioural: two skimmer
   tests carried a duplicated `#[test]`, the skimmer's `debug_dump` was dead, the
   ADS-B corpus counted a `got_total` no assertion read, the TCI pacing probe
   initialised a `mode` it re-read before ever looking at it, two needless
   `mut`s, and the WSPR busy-band test's `dB` in its name behind an
   `allow(non_snake_case)` as `smeter.rs`/`theme.rs` already do. Net 10
   insertions, 23 deletions, all of it code nothing read.

**The fmt decision is settled by evidence, not preference (2026-09-28,
re-measured 2026-09-29).** The 2026-09-28 note examined "the tree's 19
`cargo fmt` diffs" and concluded a sweep would make the tree *less* consistent.
The conclusion held; the number was wrong, and it is now measured properly —
see "Formatting" under the house rules. Two things came out of the re-measure:
the tree is far cleaner than the old note implied (26 dirty files, not a
fifth of it), and every dirty file is one upstream also edits, so the *fork-only*
half could simply be swept. It was, on 2026-09-29, in one style-only commit
(13 files, 42 hunks). A token-stream comparison confirmed the sweep changed no
behaviour; the only non-whitespace edits were rustfmt's own normalisations —
import reordering, braces added around single-expression closure bodies, and one
redundant paren dropped.


**Maintainer capacity (2026-09-27).** The maintainer is not keeping up with the
queue — several PRs have sat unreviewed for days — so do not read silence as
disinterest or re-ping. Let the branches sit; they are already rebased. When time
allows, a *small* tidy-up (like #583) is the kind that gets picked up; do not add
to the backlog faster than it drains.

**The SWL-completeness queue is done** (2026-09-27, committed to `main`). A
review of the listener side had found the core complete — SCHEDULE, the reception
log, ECSS and the receive tone, time-shift replay, scheduled recordings,
broadcast scanning, SIG ID, the propagation tools — with three gaps; all three
are closed:

1. **QSL / report tracking.** `SwlEntry::report_sent_unix` and
   `qsl_received_unix` (`swl.json`-only, `#[serde(default)]`, no `PROTO_VERSION`),
   ticked in the entry form, stamped when `REPORT` is written, and shown as a
   fixed-width **sent**/**QSL** mark on the row.
2. **Export and filter.** `swl_log_to_csv` / `swl_log_to_adif` in
   `sdroxide-types` (the decode-log shape, `SWL=Y`), and the LISTEN window's
   **Show** row — find, band, day, **Pirates only**, **CLEAR** — with a header
   stats line (heard · reported · QSL · pirates). Filters are session-only.
3. **A manual section.** §10.6 gained **#### The LISTEN window** (log, `rcl`,
   locator, pirate, antenna, REPORT, the QSL loop, filters, exports); SOLAR TIME
   stays with SCHEDULE.

The cheap extras also went: the header stats line, and a **listening
quick-start** in EN/NL/FR/IT (`docs/listening-quickstart.*`, PDFs regenerated
with the pandoc + headless-Edge pipeline in "Regenerating the quick-start PDFs"),
linked from the README beside the CB/FT8/SSTV ones.

**What is left for the listener side:** nothing is queued. **DAB/DAB+ is
SHIPPED and Kevin's** (see the delegated note at the top of this file): it was
withdrawn on 2026-10-01 over a faad2 fault, that fault was found and fixed
(faad2 built with `DRM_SUPPORT` refuses any frame whose fill element carries
anything but SBR, and DAB+ encoders pad almost every Access Unit that way), and
Kevin confirmed it on the air on 2026-10-11. `DAB_ENABLED` is `true`. The
history and the licence-shaped options that turned out to be unnecessary are in
[`ROADMAP.md`](ROADMAP.md) under DAB — kept as history, not as the state.

## Repository layout and how to work on it

- `main` → this fork, `origin` = `madmedicnl/sdroxide-brown`. The only repository to
  push to; the old `swl` branch and its fork are gone.
- The plan for the listener side lives in [`ROADMAP.md`](ROADMAP.md).

## What this build is called (fork identity, deliberately light)

The build names itself **`SDR Oxide Brown`** where a person reads it: the
native window title (`src/gui_main.rs`), the solar-system viewport title
(`crates/sdroxide-ui/src/solar3d/mod.rs`), the web `<title>`/manifest and the
browser's solar tab (`crates/sdroxide-web/`), the Settings → General header, and
`--version` (`sdroxide-version`'s `FLAVOR`/`LONG_VERSION`; `-V` stays bare).
Release **tags** carry the matching suffix, `vX.Y.Z_brown`, and releases are
titled `SDR Oxide Brown <tag>`.

**What was decided and why (2026-09-27).** Testers were running this fork and
upstream side by side and mixing them up. A **full rename was considered and
rejected**: the executable name, the `ProjectDirs` config directory, the macOS
bundle id and the release asset names were all upstream-shared, and it would
cost on every upstream merge, which is the wrong trade for an identity string.
So only the *displayed* name is the fork's.

Two of those "shared" items were **later separated anyway**, because sharing
them was an active bug rather than a cost: the **config directory** (both builds
writing one `config.toml`/`radio.json` let upstream's read-modify-write drop the
fork's extra fields — see "Running both builds") and the **Windows MSI product
identity** (one replaced the other — see "The Windows installer is its own
product"). The lines that remain genuinely shared are the **executable name**
(`sdroxide`/`sdroxide.exe`), the **macOS bundle id**, and the **release asset
names** (which the README's stable download links depend on).

**Why "Brown", not "CB/SWL" (2026-09-27, second pass).** The first name said
what the fork *is* — "SDR Oxide CB/SWL" — and operators objected: several run it
as an ordinary amateur station and did not want **CB/SWL** in a screenshot or a
screen share, where it reads as a different program than the one they are using.
The name is now a **variant** name, **Brown**, the same convention SDR++ uses
for its second build: it says "this build, not upstream" without saying what is
in it. The suffix is display-and-tag only — `LONG_VERSION` and the tag get
`_brown`, while `CARGO_PKG_VERSION` and every wire identity stay plain `1.9.2`,
because `1.9.2_brown` is not valid semver and the SSTV id, the WSPR/PSK Reporter
fields and ADIF `PROGRAMID` are parsed by other people's software.

Do **not** re-introduce the word "fork", a bare lowercase `sdroxide`, or the
old **CB/SWL** label into a user-facing title; `FLAVOR` is the one string, use
it.

## The Windows installer is its own product (2026-09-27)

`packaging/windows/main.wxs` gives Brown a **fresh `UpgradeCode`**
(`372490E0-6A0F-46AB-B44B-5447BA0F3ECF`), `Name`/`Manufacturer` **SDR Oxide
Brown**, its own install folder (`sdroxide-brown`), its own
`Software\sdroxide-brown` registry key and Start-menu/Desktop shortcut names,
and **fresh component GUIDs**. Its own install folder exists because the
executable is still called `sdroxide.exe` in both builds — a shared folder
would let one install overwrite the other's binary.

**Why (a real bug, found 2026-09-27).** The MSI first shipped with upstream's
`UpgradeCode` and product name, so Windows treated the two as **the same
product**: installing one replaced the other, and Add/Remove Programs showed
the wrong version (the reporter saw `1.9.2` from a Brown `1.9.3` install). The
general lesson: an `UpgradeCode` is a product identity, and two products a
person runs side by side must not share one.

**Consequence to remember:** a tester who still has an old `_CB` build installed
has *that* product (the old shared id) registered; the Brown MSI will not touch
it, and they should uninstall the old entry first for a clean state.

## Running both builds

**The builds isolate themselves now (2026-09-27).** This fork keeps its own
config directory, `org.sdroxide.sdroxide-brown` (i.e. `~/.config/sdroxide-brown`,
macOS `…/org.sdroxide.sdroxide-brown`), beside upstream's `org.sdroxide.sdroxide`
— so running both is safe with nothing to configure, and upstream's
read-modify-write can never drop the fork's extra fields again. This is the half
of the "full rename" that *was* worth doing: just the config path, not the
executable or the package ids.

- `config_dir()` resolves to the `-brown` directory; `recordings_dir`,
  `solar_cache_dir` and every other `config_dir()`-relative path follow it.
- **Migration is automatic and non-destructive.** `migrate_shared_config_once()`
  (called first thing in `main`, before `Settings::load`) *copies* an existing
  shared directory's contents into the `-brown` one and leaves the original
  untouched, so an existing user keeps their stations, logbook and memories, and
  upstream still finds everything. It runs once (a guard-by-existence: the
  `-brown` directory existing means done), skips when `SDROXIDE_CONFIG_DIR` is
  set, and only adopts a directory that carries `config.toml`/`radio.json`.
- `SDROXIDE_CONFIG_DIR` still overrides everything, for tests and for anyone who
  wants a third profile.
- A second `--server` still wants a different `--port`; both default to 4950.
- The window title (`SDR Oxide Brown`) tells the two windows apart once they are
  up.

The **config directory** is the one identity string that had to change for this
to work at all; the executable, the Windows MSI `UpgradeCode`, the macOS bundle
id and the release asset names remain shared with upstream, which is why the
merged code still calls everything `sdroxide`.

## Keeping up with upstream

- `dividebysandwich/sdroxide` is the original. Fetch and merge rather than
  cherry-pick where possible, so the history stays recognisable.
- **The fork is first; upstream is a courtesy — and work that came from an
  upstream issue is offered back** (operator, 2026-10-09; amended 2026-10-12).
  Upstream has fallen too far behind to be the trunk we build on, so new work is
  built here, on this fork, and shipped here. **If the change is derived from an
  upstream issue — a fix or a feature someone asked for there — open an upstream
  PR for it too**: a branch of its own off `upstream/main`, one idea per PR,
  clean and mergeable. A change that is purely the fork's own (a CB/SWL feature
  upstream has no interest in) stays in the fork. When an upstream PR *is*
  opened it is goodwill, never the reason for a decision, and the "split before
  opening / lead with the evidence" rules just below still apply.
- ⛔ **Merging upstream *into* the fork does NOT happen any more.** This entry is
  kept because it was once true, and the stale form of it is exactly the trap:
  it told an agent to go and merge. **Measured 2026-10-12, upstream is behind
  us, not ahead** — the fork is at `2.0.9` / `PROTO_VERSION 205`, upstream at
  `1.6.10` / `179`, and a trial merge threw **47 conflicts** across files
  including `Cargo.toml`, where taking upstream's side would drop the vendored
  `mfsk-core` panic fix and the version. **See the guard at the top of this
  file.** If a specific upstream fix is wanted, the maintainer cherry-picks that
  one commit by hand; an agent must not merge the branch to get it.
- **Open upstream PRs as one idea each, split before opening.** The maintainer
  has twice asked for a PR of ours to be split (**#507** and **#524**), and the
  pattern is consistent: he keeps the half whose correctness he can verify by
  reading and sets aside the half he would have to reason about or trust. In
  #507 that was the rig-keys-itself flag versus the sidetone; in #524 the
  `DECODING OFF`/dial UX versus a DSP resampler. So split **before** opening,
  into:
  - the **UX/behavioural** change (obvious-correctness) — he takes these fast;
  - the **DSP / protocol / transmit-path** change (needs trust, or a test he
    cannot see) — open separately, and lead with the *evidence*, not the
    symptom: "feeding the reference capture straight into the decoder gives 1
    event at 24 000 Hz and 0 at 25 000, deterministically" belongs in the
    opening body, not in a reply after he pushes back.
  Put the contested change last, or in its own PR. When a fix rests on a
  diagnosis that is not fully proven, say so and name the experiment that would
  settle it, rather than bundling it as settled. This is not a request to do
  less — he merges tidy contributions quickly — and it is not a style mismatch;
  he applies our commits verbatim. He just wants them decomposed, and doing it
  ourselves saves the round trip. Anything with a `PROTO_VERSION` bump, a new
  decoder, a resampler or a transmit-path change is in the "isolate it" group.
- **Review lessons — the maintainer's recurring points.** From his reviews of
  #537, #557, #558 and #561 (2026-09-25). Each of these was a returned PR item,
  so check them before opening, not after:
  - **The wire enum is append-last, always.** A variant inserted mid-enum shifts
    every discriminant below it, whatever its own note claims; #537 put
    `ServerMsg::BandOpenings` after `Spots`. Bump `PROTO_VERSION`, say what
    moved, and add a postcard round-trip test for the new variant.
  - **`VERSION_BYTE` rides `PROTO_VERSION`, and nothing reminds you.** It is a
    separate `const` in `crates/sdroxide-proto/src/lib.rs` — the framing byte
    every binary WS message starts with — and it is *not* derived, so a branch
    that bumps the version and forgets the byte ships a client that passes every
    test and is refused at the socket. **Every register entry in that file names
    both** (`205 -> 206, VERSION_BYTE 0x19 -> 0x1A`); match it. It went
    unmentioned in this file until 2026-10-12, which is how it gets forgotten.
  - **Take the next `PROTO_VERSION` yourself — check it, do not wait to be
    told.** This has been misread at least once (Kevin, 2026-10-12), after a
    guard in this file said the number was bumped "by the maintainer, by hand",
    which read as *do not bump it yourself*. It does not: **you take the next
    number and check it against the register before you start.** Two branches
    doing that correctly and independently will still collide, and that is the
    register working rather than failing — on 2026-10-10, PR #43 took 199 → 200
    and the client-answers branch took 200 → 201, each having checked; the
    renumber (`6ce02943`) moved the later one on and said why. **Where they
    collide, the later one moves.**
  - **A UI addition must respect the reserved width.** A chip's label is priced
    by `RxChip::width_label`; changing `REC` to `REC AUTO` overflowed the RX
    strip by 30 pt. Show a state with a tint, an outline or the hover instead of
    widening the label, and add a phone-width layout test for every new chip row.
  - **Carried state needs all its edges.** A per-frame gate must handle an
    operator's manual stop (do not undo it next frame), a start that never took
    (do not re-send it every frame), and every condition the engine itself uses
    (the tone squelch and our own transmit, not only passband power).
  - **Feed-derived state is per path, per band, and per radio.** PSK Reporter is
    only polled for the band the dial is on, so a warm-up span is per (band ×
    continent); a dedupe key must include a time bucket or a station still active
    after the retention window counts as new again; and anything the manager
    derives from the feeds must ride `adopt_spot_feed` or it is empty on every
    tab but the station radio.
  - **Declare levels honestly.** `fsk441_generate_audio` is full-scale, so
    `tx_peak` must be 1.0 — the engine scales by `1/peak`, and a default 0.5
    doubled it into the limiter. Releasing transmit must stop the loop, and an
    empty box must refuse the key and say why.
  - **Third-party code carries its licence.** A module adapted from an MIT
    project includes that project's full notice in the file.
  - **Do not trust feed order or leave caches unbounded**, and **no comments
    that talk about the review, the fork, or a "first version"** — upstream code
    should read as if it were always there.
- `PROTO_VERSION` in `crates/sdroxide-proto` is a fork superset of upstream's:
  the last upstream number folded in was **v170**, and the fork's `main` is now
  at **197**. The fork's extras are
  the listener identity (`NetworkConfig::swl_id`, `RadioConfig::callsign`,
  `RadioConfig::hide_tx`), `Command::ResetModeDefaults`, and the per-radio
  additions — **the register's full story is in `crates/sdroxide-proto/src/lib.rs`,
  which is the only place it is kept current**; the run of v171–v197 is
  documented there, one entry per bump. Upstream's v157/158 (SSTV styling and
  the (tr)uSDX family), **v159 (NR2's three `NrLevel` variants)**, **v160
  (`CwStatus::rig_keys_itself`)** and **v165** (the band-decoder relay outputs,
  #442) were folded in by earlier merges. On the **2026-09-25 merge**
  (`5ed9e2d3`) upstream's **v166–v170** folded in — MSK144, JT65/JT9, FST4,
  Q65 and FSK441, the fork's own six modes that upstream took (see below) — so
  the fork-only entries renumbered and now sit at **v171**–**v177**:
  `auto_idle_stop_min`, `SpotKind::HeardMe`, the (tr)uSDX nG family,
  `ServerMsg::BandOpenings`, DSC, UVPacket and the ATS Mini, with **v178**
  (`DigiStatus::tx_refused`, the FSK441 transmit review fix ported to `main`)
  **v179** (the CW key's five appended `DigiConfig` fields) and **v180–v197**
  (the wide CB grammar, the contest layouts, the decode depth, the per-radio
  state, the ALC/gain switch, the client screen, the client bindings opt-in, the
  KNOWN window's question-and-answer pair, the CW paddle contacts, the signed-in
  profile, the contest layout command, the Retro Radio removal, the client
  panadapter view fields, DAB, SSTV KEEP DIAL and the eQSL QTH nickname) on top.
  **Upstream moved again on 2026-10-08** — its own `PROTO_VERSION` is now
  **173** (the WSPR-per-radio #615 work bumped it after the fork's last merge),
  so it has *not* stopped at v170. The fork keeps its own numbering (`main` at
  197); upstream's v171–v173 are upstream-only and are **not** folded in. Fold
  them under the fork's register if a future merge needs to, exactly as the
  2026-09-25 merge folded v166–v170 — and read `crates/sdroxide-proto/src/lib.rs`
  first, it is the current one. Append at the tail and
  add a register line, and if upstream ever moves again, fold its new entries in
  under ours as the 2026-09-25 merge did (the latest worked example, after the
  2026-09-23 and 2026-09-20 ones).
- Watch list:
  - `dividebysandwich/sdroxide` — upstream moves; merge regularly. Merging
    after each upstream release, or monthly, keeps the conflicts small; 46
    accumulated commits made one merge twenty conflicted files.
  - **The 2026-09-25 merge (`5ed9e2d3`) took a large batch upstream.** Upstream
    merged the fork's **#541** (grey line), **#542** (meteor calendar),
    **#543** (IBP beacons), **#544** (Kp history), **#549** (MSK144), **#550**
    (JT65/JT9), **#551** (FST4), **#552** (Q65), **#555** (FSK441), **#556**
    (VDL2 window rate) and **#562–#564** (FT4/FT2/JS8 successive-interference
    cancellation), each with its own review commits. The fork's copies of the
    features dropped out; the fork keeps its mfsk-core 0.11 pin (CB grammar and
    UVPacket) so the reviewed FSK441 DSP was taken but the 0.10-coupled
    controllers/modems were not. Still open upstream from this batch: **#545**
    (draft, (tr)uSDX nG), **#554** (draft, UVPacket), **#557** (recording
    silence split), **#558** (SAVE decoded text), **#559** (band-menu captions,
    fork-only on purpose), **#561** (FSK441 transmit) and **#537** (band
    openings).
  - `jl1nie/mfsk-core#373` — the fork's CB (11 m) grammar. **Declined and
    closed 2026-09-20.** The maintainer first asked for the feature to become a
    caller-supplied predicate, wrote that design up as #383, then withdrew even
    the "plain `pub fn` here" offer: a dialect's grammar is application policy
    and mfsk-core is a port of WSJT-X, so they will not carry it, inert or not.
    The grammar therefore lives in this fork as
    `sdroxide_types::is_cb_callsign` (`crates/sdroxide-types/src/cb_callsign.rs`,
    with the 25-case WSJT-CB table). The field-based hook it was waiting for —
    `DecodeRequest::also_accept(|m| m.callsigns().all(f))`, which yields only
    callsign *fields*, so a grid or a report cannot be fed to the grammar — is
    their **#386**, merged on 2026-09-20 and released as **mfsk-core 0.11.0**.
    **The fork is retired: `madmedicnl/mfsk-core` is gone from the build.**
    `sdroxide-digi` pins `mfsk-core = "0.11"` and calls the hook with
    `is_cb_compatible_call(call) = wsjt77::is_plausible_call(call) ||
    sdroxide_types::is_cb_callsign(call)` on the FT8 and FT4 decode requests,
    and `is_packable_call(call) = wsjt77::is_valid_callsign(call) ||
    sdroxide_types::is_cb_callsign(call)` on the encode side (stock 0.11
    `is_valid_callsign` refuses CB calls, so both the decode gate and the pack
    ladder needed the union — the fork's widening of the validator itself was
    the thing the pin supplied). 0.11 also removed the FT4 `sniper` (the FT4
    targeted pass is now a ±250 Hz wide-band request with an a-priori hint) and
    #386 dropped the text-based plausibility filter that had been silently
    discarding the FT8 EU-VHF contest exchange (`i3 = 5`) — which is why the
    `ft8_eu` rescue pass is gone too. (Their earlier `cb_ok(&str)` sketch was
    their own retracted error: unpack77 discards the fields, so tokenising the
    rendered text runs grids and exchanges through the grammar.)
  - Upstream PRs, branched from `upstream/main` and merged into the fork's
    build: **#514** (HD-on-AM), **#524** (the HFDL lane-rate fix and the
    DECODING OFF wording) and **#532** (the REC auto-stop timer). **#500**
    (the WEFAX auto start/stop fix,
    #496) and **#508** (the LimeSDR Mini board-name fold) were **taken
    upstream** on the 2026-09-20 merge, so the fork's copies dropped out (a
    follow-up comment tweak of the maintainer's on the WEFAX shape test came
    with it). #500's review is the part worth keeping: the maintainer caught
    that its strict mid-picture rephase test (pulse in the last 10 % of the
    line) regressed #276, and the fix now rests on the shape (line nearly
    black, narrow near-white pulse) plus `note_phasing_line`'s cross-line
    consistency, never on the pulse's position — mid-picture the buffer is cut
    on the old transmission's clock, so a new pulse lands at an arbitrary
    offset. A narrow, clean, *static* stripe is indistinguishable from a
    phasing pulse from line data alone; the shape and the eight-line run are
    what carry it. **#498** (the (tr)uSDX family) and **#501** (the Icom WFM
     mode byte fix, #494) were taken earlier. (#499 is closed as superseded by
    #498.) **Taken upstream on the 2026-09-20 merge from `upstream/main`:**
    **#519** (the PureSignal IO-board warning gate, as `29ed7539`), **#512**
    (the frequency type-in, `43ce0543`), **#522** (the auto-upload master/target
    trap, `e6ffbe4b`), and the **rig-keys-itself half of #507** (`bcef7787`) —
    all four landed as direct commits rather than merged PRs, so the fork's
    branches for them are done and their fork copies dropped out.
  - **The 2026-09-21 merge took the rest of #507 and all of #509 upstream.**
    **#507** (the CW sidetone, `cw_tx_idle_s`, and the straight-key read-back)
    merged as PR `1948e656`, with the maintainer's own fixes on top: the
    P-glyph read-back correction, the sidetone's resampled remainder, a
    bindings migration for the straight key's default, and rustfmt. **#509**
    (HFDL) merged as `bbc47e0a`, likewise with review work: a manual section,
    README/mode-table entries, rustfmt and doc corrections. Both PRs show OPEN
    on GitHub because the changes landed as commits from a rebased branch
    rather than via the merge button — #509 has a comment saying so. The
    fork's copies of both dropped out on the 2026-09-21 merge.
    **Two HFDL follow-ups upstream did not take** are the fork's to offer:
    the **lane-rate fix** (the decoder decodes only a 24 000 Hz lane, and the
    DDC reaches it only for some sample rates — 2.0 Msps gives 25 000 and
    decodes nothing) and the **DECODING OFF** status wording. Both are in the
    fork (rate fix `6809a68f`, wording `847423c3`) and offered as **PR #524**
    (branch `upstream-pr/497-hfdl-rate`, rebased on `upstream/main`). The rate
    fix is confirmed on real hardware (an RSP1A at 2.000 Msps, 47 decodes, 2
    aircraft) — it was merged upstream on 2026-09-22, so the fork's copies
    dropped out with no net change on that merge.
  - **The 2026-09-22 merge (`0a417ab8`) took #524 and #532 upstream, and
    brought the nightly builds.** Ten upstream commits since `661bd0ab`: the
    **REC preset-lit refinement** (`8293d05a`, on the `recording_stop_at`
    deadline the fork had already merged as #532) and **#524**'s HFDL rate fix
    and DECODING OFF wording — the latter two were already the fork's own
    code, so `engine.rs` merged byte-identically and nothing dropped. New to
    the fork: **nightly builds and the `sdroxide-version` stamp crate**
    (`762eca62`, `02fa526f`) and dielectric-coder's **#531 worldmap
    seam-streak fix**. The fork's own **"Remove the sdroxide.com update
    check"** (`c01d27bc`) is **kept**: the merge base and upstream both carry
    the update banner, so the resolution had to drop upstream's newer wording
    rather than reintroduce it — the one file where "take upstream" is wrong.
  - `dividebysandwich/sdroxide#580` — **the 3D window's geometry across a
    rebuild**, opened 2026-09-26 from `upstream/main` (branch
    `upstream-pr/solar3d-window-geometry`, one commit). The solar-system
    viewport is destroyed and rebuilt when its radio tab goes behind another, so
    the window came back at the hardcoded 1180×760. The geometry is carried in
    `UiSettings::solar3d_window` (`Solar3dWindow`), seeded only on the rebuild
    frame, captured from `content_rect`/`outer_rect` (Wayland gives neither
    rect, so only the size returns). The fork's copy is the same change plus the
    owner cap and `keep_alive` (see "The 3D window in the multi-radio shell"),
    which are **not** in this PR — the geometry half stands alone and is the one
    Windows users feel. **The fork's `keep_alive`/owner cap is deliberately
    held back (#2 of three); offer it separately if #580 lands and he wants the
    window to survive a tab switch.**
  - `dividebysandwich/sdroxide#586` — **FT8's checkpointed signal
    subtraction**, opened 2026-09-27 from `upstream/main` (branch
    `upstream-pr/ft8-sic`, one commit, 58 insertions in `modem.rs`). Not
    CB-specific and, unlike most of ours, it is an **upstream bug**: upstream's
    FT4 calls `.sic_rounds(2)` while its FT8 is the bare single pass, and
    mfsk-core's default does *no* subtraction while WSJT-X runs multi-pass — so
    a weak signal inside a stronger neighbour's 50 Hz bandwidth is not decoded
    for anyone. The fix is one `.sic_early()` call (see "FT8 runs signal
    subtraction" above for the measurements: 12 → 22 decodes, none lost; floor
    unchanged; ~26 ms → ~1.2 s). No wire change. The fork's `main` carries it
    (`a19a82a7`). **If he pushes back on the ~1.2 s cost**, the stated
    alternative in the PR body is gating it on decode depth rather than always
    on — offer that rather than arguing the unconditional default.
  - `dividebysandwich/sdroxide#583` — **a hand-picked sign-off is not undone
    by the DX repeating**, opened 2026-09-27 from `upstream/main` (branch
    `upstream-pr/hand-picked-signoff`, one commit, 70 insertions in
    `qso.rs`). Not CB-specific: `set_step` (RR73/73 from the Tx buttons) sets
    `self.step`, but `advance`'s `_ =>` arm blindly does
    `self.step = reply_step(payload)` on every decode, so a DX repeating the
    report we have decided not to answer drags it back to `TxRReport` and the
    sign-off never leaves. `manual_signoff` holds the step still (their report is
    still recorded) until it goes out, the contact ends, or another step is
    picked. Two tests; the first fails on the old code at "the sign-off the
    operator picked still stands". Behavioural/UX, no wire change.
    **Confirmed on the air** (2026-09-27, CRT SS9900v) — picked 73 mid-QSO at
    R+report, it went out and stayed, and the contact logged. The fork's `main`
    carries the same fix (`fbe11818`). **It landed in `90733eb2` and the fork's copy
    dropped out** — but the maintainer's own test is stronger than the one the PR
    shipped (`aa46db61`): he made the release test send an **RR73**, so it fails if
    the hold is *kept*, where the PR's test only proved a report did not release
    it. That is the version now in the tree.
  - `dividebysandwich/sdroxide#588` — **the IQ channel probe reads the opened
    PCM's stream**, opened 2026-09-27 from `upstream/main` (branch
    `upstream-pr/582-iq-stream-channels`, one commit, 76 insertions in
    `crates/sdroxide-audio/src/lib.rs`). An upstream bug in the maintainer's own
    mono-for-IQ guard (`c94afb84`): it read
    `/proc/asound/cardN/stream0` whatever PCM was opened, so a USB card with a
    mono demod on stream0 and a stereo I/Q on stream1 — a Malachite DSP, issue
    #582 — had the stereo input judged by the mono demod and refused as mono.
    The guard now selects the stream from the PCM's own `DEV=` index, falling
    back to stream0 for `sysdefault:CARD=X` names so single-stream cards are
    unchanged. Not tested on the reporter's hardware; the two-stream layout is a
    unit test. The fork's `main` carries the same fix (`804109e7`); it **dropped out
    when it landed** in `90733eb2`.
  - `dividebysandwich/sdroxide#591` — **a spoken reply per alert event**,
    opened 2026-09-28 from `upstream/main` (branch `upstream-pr/voice-alerts`,
    one commit, ~286 insertions across `sdroxide-types`, `sdroxide-speech`,
    `sdroxide-ui` and the manual). The audible alerts are five synthesised
    tones and the offline announcement voice (`sdroxide-speech`) was never
    joined to them, so a new DXCC could only ring. `AlertRule` gains
    `reply: AlertReply::{Tone, Voice, Both}` (default `Tone`, so configs are
    unchanged); a Voice rule speaks the phrase through the announcement voice —
    "<call>, new D X C C, <country>", "<call>, calling you", "<call>, new one on
    20 metres" — with the callsign leading, "DXCC" spelled `D X C C` for the
    TTS, and the country from `entity_name`. It fires from the **alarm path**,
    not the focus-gated `Announcer::on_ft8`, so it is heard when the window is
    not in front; `AlertCore::on_ft8` returns the fired `AlertFired` for the
    caller to speak, and only plays the tone when the reply asks. The wording
     is a pure `announce::alert` function. Settings → Alerts gains the per-event
     reply. No wire change. The fork's `main` carries it (`b38125d5`).
     **Taken upstream on the 2026-09-28 merge** (`91595abd`), but the
     maintainer then changed the design in his own review commit **`aa3971de`**:
     speech became **focused-only** (a background radio's phrase would be read
     out late, as news no longer so) and a voice-only alert that cannot speak
     **rings its tone instead**, which is what `AlertFired::sound` is for. He
     also kept the phrase behind the **transmit gag** (`Priority::Notable`) and
     wrapped the settings row. **The fork deliberately does not take the first
     two** — see the "decisions, not mechanics" note in the session section
     above. So `AlertFired::sound` and `AlertRuntime::ring` are **gone** from
     the fork (nothing reads them), and the fork's own discoverable half — the
     `speaks: …` preview and the **SAY** button, `1e35b512` — is the reason the
     operator does not miss what a Voice rule will say.

  - `dividebysandwich/sdroxide#590` — **the REC popup's Quick clip row**,
    opened 2026-09-28 from `upstream/main` (branch `upstream-pr/quick-clip`,
    one commit). The **Stop after** chips only appeared once a recording ran
    and their shortest preset was **15 min**, so a short sample was unreachable
    in one action. A new **Quick clip** row (**30 s**, **1 min**) starts the
    MP3 recording if none is running and stops it at the end of the span.
    `recording_stop_at` now carries its preset in seconds; a clip's deadline
    cannot be armed at the press (the timer tick drops any deadline whose
    recording is not running), so it rides a small `rec_clip` until the
    recorder is seen running. A clip disarms **Stop after** and **Auto-record**
    and vice versa. No wire change, UI only. The fork's `main` carries it
    (`cf6dfde1`); it **dropped out when it landed** in `90733eb2`, though the
    maintainer's review commits changed it first — `REC_CLIP_START_TIMEOUT_S` is his
    rename and the `(time, seconds)` pair is now the named `RecSpan` (clippy's
    `type_complexity`).
  - `dividebysandwich/sdroxide#596` — **editable message buttons for the keyboard
    modes** (upstream issue #463), opened 2026-09-29 from `upstream/main` (branch
    `upstream-pr/digi-macros`, one commit). Adds `DigiConfig::text_macros` — the
    same `CwMacro` label+text shape the CW panel already has, on a list of its
    own — and draws the row and editor on the PSK/RTTY/Olivia/THOR panel, with
    the control extracted to `panels/macros.rs` and shared by both. The fork's
    `main` carries it (`1e93799f`) with `PROTO_VERSION` 181 → **182**; the field
    sits after `cw_macros` (the PR's own placement, so the merge is clean). The
    fork's only delta is the version number — **when it lands, the fork's copy
    drops out; do not re-add the feature.**
  - `dividebysandwich/sdroxide#603` — **the contest logger**, opened 2026-09-30
    from `upstream/main` (branch `upstream-pr/contest-logger`, one commit). The
    logger model + window from the fork, **stripped of the CB bits**: the
    `CbActivity` variant becomes a generic `Text` exchange so nothing fork-only
    rides it. No `PROTO_VERSION` change — a session is session-only and the
    QSOs are ordinary `QsoRecord`s. Points are 1 per QSO and the score says
    "estimate" on screen (the PR body flags this as a deliberate choice).
  - `dividebysandwich/sdroxide#604` — **selectable FT8 decode depth**, opened
    2026-09-30 from `upstream/main` (branch `upstream-pr/ft8-depth`, one
    commit). `Ft8Depth` (`Fast` / `Normal` / `Deep`) on `DigiConfig`, default
    Deep; the three measured strategies and times are in the PR body. **Builds
    on the staging in #586** (the plain single pass is split off there, so a
    reply is not held up) — this governs only the extra batch, so if #586 lands
    in a different shape it is a small rebase. Appended field, so upstream's
    `PROTO_VERSION` 170 → 171. The fork's own `main` already carries both, with
    its CB additions intact; the fork's copies drop out when these land.
  - `dividebysandwich/sdroxide#597` — **JTTY, the WSJT-X 3.2 asynchronous text
    mode** (issue #584), opened 2026-09-29 from `upstream/main` (branch
    `upstream-pr/jtty`, **one squashed commit**, 46 files, ~2,700 lines). A
    **hand-port, not a cherry-pick**: the fork's JTTY commits were written against
    the fork's mode set, so replaying them drags `UvPacket`/`Dsc`/`Hfdl` into
    upstream's tables. The upstream form adds **only** `Mode::Jtty` (four new
    files verbatim: `dsp/jtty.rs`, `types/jtty.rs`, `digi/jtty_controller.rs`,
    `ui/panels/jtty.rs`; then the one-line `Jtty` arms across the ten CAT/TCI/
    rigctld/smartsdr/speech tables, `mode.rs` + `ALL`/`DIGITAL`, `proto`
    `PROTO_VERSION` 170→171, `digi.rs` `DigiStatus::jtty`, `dsp` demod/modulator,
    `engine.rs`, the panel registry, `frame.rs`, `save_text.rs`, the manual).
    **The fork's copy is *not* this one**: the fork's JTTY still carries the
    `DigiStatus::tx_refused` refusal line and the CB identifiers, which the
    upstream port dropped to stay isolated. When #597 lands, reconcile rather
    than drop — the merge will bring upstream's Jtty plumbing and the fork must
    keep its extras. **Test status stated in the PR**: golden-vector and synthetic
    verified, **never off-air**.
  - `dividebysandwich/sdroxide#593` — **the grey line's contrast**, opened
    2026-09-28 from `upstream/main` (branch `upstream-pr/greyline-contrast`, one
    commit, two files). The night overlay was painted *before* `draw_base`, so
    it darkened only the ocean and the propagation heat and left the continents
    lit, and its darkest alpha (0.62) read dark-on-dark on the dark themes —
    the operator reported it as "hardly visible". It is now painted after the
    base map with `NIGHT_MAX_ALPHA` 0.78, so land darkens with the sea;
    coastlines and borders darken too, the deliberate trade for a terminator
    that reads. Upstream carries the overlay only in `worldmap::show`, so the
    PR is the two-file change; the fork's `main` also moved the fork-only
    `paint_night` on ADS-B, AIS, APRS and HFDL. The fork's `main` carries it
    (`ac4811dc`); it **dropped out when it landed** in `90733eb2`, with his own
    review commits on top — the night is now also **scaled by the palette's**
    `map.night_max` (so a light theme does not get ink-on-ink), the **city names are
    drawn over it**, and `NIGHT_MAX_ALPHA` became `pub` for that. The fork's
    ADS-B/AIS/APRS/HFDL `paint_night` calls stay.
  - `dividebysandwich/sdroxide#579` — **decodes into the propagation field**,
    opened 2026-09-26 from `upstream/main` (branch `upstream-pr/prop-decodes`,
    one commit). `PropStore::observe_decodes` exists and places a decodes station
    by grid or country, but nothing in the UI called it: the 3D globe's BANDS
    OPEN chart read only the RBN feed, so it was empty on any band RBN does not
    carry (11 m, freeband). The FT8-family decodes now fold in as
    `PropSource::Ft8`. General, not CB-specific.
  - `dividebysandwich/sdroxide#575` — **the band/mode dock**, opened
    2026-09-26 from `upstream/main` (branch `upstream-pr/band-dock`, one
    commit). A `DOCK` chip in the Band/Mode popup moves the same
    `band_mode_menu` into a resizable column beside the waterfall, with
    UNDOCK/× in its header and the band chip toggling the column; desktop and
    tablet only (`band_dock_allowed`). **Adapted, not copied**: upstream's
    `band_mode_menu` is the simple three-row one, so the fork's own dock
    (`17467d57`, tangled with the fork-only band-menu clarity — tabs, filter,
    metre bands) did **not** drop out, and the prediction held: they are two
    different menus, so the fork kept its version. **It landed in `90733eb2`** and
    what came across was the machinery around the dock, not the menu —
    `band_dock_room` as the single room decision, the width range re-applied every
    frame, and a narrow window hiding the column instead of undocking it. The call
    still passes the fork's own arguments.
  - `dividebysandwich/sdroxide#537` — **the band-opening detector**, opened
    2026-09-22 from `upstream/main` (branch `upstream-pr/band-openings`, based
    on the current `upstream/main`, no 11 m feed). A pure
    `sdroxide_types::band_openings` tracker (ported from OpenHamClock) over the
    existing spot feeds, shown in the SPOTS window behind an `OPENINGS` chip
    with a draggable split, relayed as `ServerMsg::BandOpenings` (v165 on that
    branch). The **fork's C.B. side feeds the same detector its own 11 m
    WSJT-CB decodes**, and adds the WSPRnet 403 wording — both fork-only and
    deliberately not in the PR; they live on `fork/live-band-openings` until
    #537's detector lands, then the fork's copy drops out and only the 11 m
    feed and the WSPRnet wording remain.
  - **#514** (HD-on-AM, `upstream-pr/489-hd-am`) was **rebased on current
    `upstream/main` on 2026-09-21 and marked ready for review**, and was
    **taken upstream on 2026-09-22** (`08:41`), so the fork's copy dropped out
    (it is upstream's `HdRadio` AM wiring now). The earlier draft carried stray
    vendored gitlinks (`vendor/nrsc5`, `vendor/xng`), which the rebase dropped —
    `vendor/xng` in particular belongs upstream now (from #509) and deleting it
    would have broken the build. Read "no `libnrsc5` on this machine, no
    decodable HD-on-AM station" as the standing caveat: the pure parts are
    unit-tested, nothing end-to-end is.
  - `dividebysandwich/sdroxide#495` — the IC-7610 LAN straight key. Diagnosed
    as **by design**, not a bug: the keyboard straight key is disabled when the
    rig is keyed by its own keyer (`cw_controller.rs`), which the LAN backend
    is, so the workaround is Sound card (MCW) keying. Commented and left to the
    maintainer. The reporter came back that MCW works but he can hear no
    sidetone, the carrier hangs too long after the last character/key, and he
    wants the keying key to be selectable. All three are fixed in the fork:
    `DigiConfig::cw_sidetone` (local sidetone monitor, `SIDETONE` chip),
    `DigiConfig::cw_tx_idle_s` (configurable hold, `IDLE` chip), and the
    straight key is now `Action::CwStraight`, assignable in Settings → Controls
    with Space as its default. `PROTO_VERSION` 159 -> 160 in the fork (both
    DigiConfig fields appended). **Offered upstream as PR #507** (branch
    `upstream-pr/495-cw-followups`, nine commits based on `upstream/main`),
    where the two fields take 156 -> 157 and the read-back below 157 -> 158;
    the fork's copy is the same change, so it drops out on the next merge.
    The sidetone needed two fixes after first shipping: gate the monitor on the
    digi engine's mode rather than the rig's (MCW commands a sideband, so the
    gate could close mid-key), and play each block live from the TX loop, since
    on a half-duplex rig the receiver is never read during transmit and a queue
    drained only on the RX speaker path starved — the tone arrived as one beep
    after PTT dropped.
    The operator also could not see what he was keying, so the straight key now
    reads back where the text keyer's box is (`CwSelfRx`, a small immediate
    decoder in `crates/sdroxide-dsp/src/cw.rs` fed the transmit block —
    `CwStatus::sent_text`; the classic `CwRx`'s six-second window and
    three-second catch-up were unusable for that, and the receive tap never
    carries our own sidetone). Both CLEAR controls empty it, and the message
    editor's `MSG` chip moved up beside SIDETONE and SEND ON RETURN. Not
    verified on air here — **though the blocker is gone**: the operator is on a
    licence-free frequency and transmitting is authorised (see "The bench"), so
    this is testable on 11 m CB with a short keyed transmission through the
    MCW/VOX route. The maintainer took the
    **rig-keys-itself** half upstream on the 2026-09-20 merge (`bcef7787`,
    `CwStatus::rig_keys_itself`, upstream's 160) — the CW panel keeps both that
    and the fork's sidetone/read-back, since `CwStatus` carries both fields.
    What remains fork-only is the sidetone, the configurable idle hold and the
    `sent_text` read-back; #507 is still open for those.
  - `dividebysandwich/sdroxide#497` — a request for an HFDL decoder. Scoped on
    the issue; see "The HFDL core" below. The requester answered the two
    questions (2026-09-19): he defers to our judgment on git-dependency vs
    vendored port and on scope, so Route A with **decoder + decode log first,
    aircraft map second** is green-lit. In the fork as `crates/sdroxide-hfdl`.
    **Offered upstream as draft PR #509** (branch `upstream-pr/497-hfdl`); the
    maintainer asked for `xng` to be vendored rather than a cargo git
    dependency, which is done (`vendor/xng`, a pinned submodule).
    PROTO_VERSION 158 -> 159 on that branch. (Related: #512 the frequency
    type-in and #514 the HD-on-AM wiring are both now **taken upstream**.)
  - `dividebysandwich/sdroxide#518` — a Hermes/ANAN reporter's PureSignal log.
    The HPSDR PureSignal startup warning fired on any board whenever
    `io_rx_input` was not the IO board's PureSignal jack, but that input is
    Hermes-Lite 2 only — the value is applied only on a Protocol 1 board with
    an LNA gain register, DDC 0 — so a Hermes/ANAN operator is told to change
    a setting the radio has no input for and the UI does not offer. **Taken
    upstream on the 2026-09-20 merge** (`29ed7539`, direct commit — the fork
    had offered it as draft PR #519, branch
    `upstream-pr/518-puresignal-io-warn`), so the fork's copy is upstream's
    canonical form: the warning is now gated on `board.has_io_board()`, and
    the info line carries the real guidance — the first receiver is the
    feedback path, a coupler into RX2/ADC2 cannot lock the loop (issue #510).
    **2026-09-22:** the reporter (it is a Red Pitaya, not an ANAN) bridged
    the coupler into RX1 and the loop now locks (~0.9 dB, 75 % correction) —
    confirming the model; he floated "make RX2 work for PS", which is the
    RX2/ADC2-as-feedback feature scoped on #510 (Protocol 1 streams one ADC;
    the backend would need a two-ADC mode and a feedback-source choice), no
    hardware here to verify against.
  - `dividebysandwich/sdroxide#520` — a request for a stop timer on the
    on-air MP3 recording with preset durations. Offered as **PR #532**
    (branch `upstream-pr/520-rec-timer`, based on `upstream/main`). The
    deadline is UI-owned, not engine state: an `Option<i64>` on
    `SdroxideApp` ticked once a frame, which sends `SetRecording(false)`
    when it passes. The REC popup's "Stop after" row has 15/30/45/60/90
    minute presets, a "no stop" cancel and a live mm:ss countdown. It is
    cleared the moment the recording stops any other way, so an armed
    stop cannot leak into the next recording — that contract is what
    `rec_timer_tick`'s unit tests pin; no engine or `RadioState` change.
    Already merged into the fork's `main`, so it drops out here on the
    next upstream merge if #532 lands.
  - `dividebysandwich/sdroxide#503` — the fork's RADE receive-reporting fix,
    for upstream issue **#502**. Two things: the RADE panel never drew the
    callsign decoded from the End-of-Over frame (it was in `DigiStatus::dx_call`
    all along), and we never sent the empty-callsign `rx_report` that says
    "hearing something", which is what tells a transmitting station it is being
    heard before either end has identified the other. Branched from
    `upstream/main` (branch `upstream-pr/502-freedv-rade-rx`), so the fork's
    copy is the same commit and drops out on the next merge. **Taken upstream;
    reconciled to its canonical form on the 2026-09 merge.** The DX_CALL_HOLD
    hold-off and the "hearing something" report are confirmed in the merged
    code; not confirmed on air — the blocker is gone, since transmitting is
    authorised on the operator's licence-free frequency (see "The bench"), so
    this is now testable rather than blocked.
  - `dividebysandwich/sdroxide#505` — the fork's SSTV picture styling, offered
    upstream (branch `upstream-pr/sstv-style`, one squashed commit). One new
    `DigiConfig::sstv_style` (`SstvStyle`): strip gradient, banner text colour,
    gradient and outline, message ink/outline, and a rainbow override for all
    the picture's text. Bumps `PROTO_VERSION` 156 -> 157 on the branch, which
    **collided with #504's branch**, which also claimed 157 — whoever landed
    second had to move up. **Taken upstream; reconciled on the 2026-09 merge**
    (both #505's 157 and #498's 158 now upstream registers), so the fork's copy
    is upstream's canonical form.
  - `dividebysandwich/sdroxide#504` — an ANAN-7000DLE (OpenHPSDR) report that
    the per-band drive matrix does nothing and the drive slider is dangerous on
    a high-gain SDR. Diagnosed: the matrix *is* dB of output and does apply to
    I/Q radios (`crates/sdroxide-radio/tests/tx_drive_by_band.rs` proves a
    −20 dB row, TUNE included), and the HPSDR FPGA drive register is pinned at
    full scale on purpose — the amplitude is scaled in software. The real gap
    was that nothing capped the absolute drive.
    **Taken upstream on the 2026-09-20 merge** (commit `ffba2cea`):
    `RadioConfig::tx_drive_max`, an operator ceiling applied after the band
    calibration. The fork carries upstream's version now, appended *after* the
    fork's own `RadioConfig` tail fields (`callsign`, `hide_tx`,
    `auto_idle_stop_min`) to keep the positional layout; our own prework on
    `upstream-pr/504-tx-drive-ceiling` (`c178390a`) is superseded and can be
    dropped.
  - `dividebysandwich/sdroxide#483` — a request for a DAB/DAB+ decoder, so it
    can be used over a SpyServer like the other decoders. Scoped on the issue;
    the plan now lives in [`ROADMAP.md`](ROADMAP.md) under Phase 3, and it
    supersedes the details below where they differ. The enabling find is
    **`dabradio`** (MIT, `xoolive/desperado`) — the full OFDM/FIC/MSC/Viterbi/RS
    chain plus pure-Rust MP2. Re-checked against **0.5.0** (2026-09-21): still
    a TUI **binary, not a library** (`has_lib: false`, 8.4k LOC), and it now
    declares **`fdk-aac` a hard, non-optional dependency** (which sdroxide does
    not link — swap in the vendored **faad2**), on top of `ratatui`/`crossterm`/
    `viuer`/`tinyaudio`/`clap`/**`desperado`** (rtlsdr/airspy/hackrf front ends
    we do not want) and `tokio` in full. The reusable part is the DSP + FIC/MSC
    state machine; extracting it from an async app that owns its own radio and
    terminal is the work. **The crate question is largely answered: the author
    (`xoolive`) said on the issue (2026-09-21) he does not mind splitting
    `dabradio` into a library plus a thin executable, and is himself decoding
    HD Radio with an eye to a shared lib for both DAB and NRSC-5.** **Drafted
    2026-09-25 — monitor:** the split is open as draft PR
    [`xoolive/desperado#52`](https://github.com/xoolive/desperado/pull/52)
    (branch `lib/split-dabradio`, fork `madmedicnl/desperado`), no decoder
    logic changed: `src/lib.rs` exposes the pipeline modules (the crate docs
    moved there), `main.rs` keeps the TUI/device setup and the resampler and
    uses `dabradio::…`; **`fdk-aac` becomes an optional feature** so the DAB+
    super-frame layer (`audio::SuperframeDecoder` — Fire code, RS, Access
    Units) always builds and a consumer with its own AAC decoder can take the
    Access Units (sdroxide's **faad2** is exactly that caller); the TUI/device
    deps move behind a default `bin` feature. 67 lib + 9 bin tests pass,
    `--lib --no-default-features` passes the same 67 without fdk-aac, clippy
    clean both ways. A high-level `DabDecoder::process(...) -> events` facade
    is deliberately **left to the author's API taste** — he should shape it,
    and the PR body says so. Still **not costed** and nothing commits either
    side. DAB Mode I needs the full **1.536 MHz** /
    ~2.048 Msps, a wideband lane like ADS-B's rather than the 12 kHz `on_rx_iq`
    tap. No code, no commitment; still the maintainer's call. Interest
    re-confirmed 2026-09-21/22 — a `+1`, and **pvanderp offered to record
    off-air I/Q for validation** (unknown size, but the demand and now the test
    material are real). **The capture landed 2026-09-22** in
    reply `5778225835`: a 7z-compressed raw `.cs16` of channel **12C** at a
    **227.360 MHz** centre (SDRconnect put the rate at 2048 ksps and left a
    ~40 kHz tune offset that the decoder has to absorb), readable by dabradio;
    `xoolive` noted dabradio reads `.zst` directly (smaller fixture than a
    `7z`) and is adding filename-inferred `--center-freq`. A **second,
    independent capture** arrived 2026-09-22 from **kevin2008-01** (Nancy,
    France): raw `.cs16` in a `.zst`, ~30 s of channel **8B** at **197.648 MHz**,
    2.5 Msps, taken with `iio_readdev` against a PlutoSDR (Tezuka firmware) so
    there is no SDR container to strip. `xoolive` confirmed both decode — 12C on
    the SDRconnect file, 8B on the Pluto one (`--service "BFM BUSINESS"`) — and
    pointed at the **dabradio 0.5.0 release binaries**, so either capture can be
    replayed without a local build. pvanderp runs a dozen other Dutch ensembles
    if 12C does not exercise whatever comes next.

(HD Radio landed upstream with #466 and the fork's duplicate is retired: the
faad2 submodule is back on `knik0/faad2`, `crates/sdroxide-faad2` patches it at
build time and `madmedicnl/faad2-hdc` is gone. See "The HD Radio capture
harness" below.)

### `jl1nie/mfsk-core#386` landed (the CB decode hook)

#373 was declined and closed (see the watch list); the grammar is ours now, in
`sdroxide_types::is_cb_callsign`. #386 added the field-based hook —
`DecodeRequest::also_accept(|m| m.callsigns().all(f))` — where `callsigns()`
yields only callsign *fields*, so a grid or an exchange cannot be fed to the
grammar (their first `cb_ok(&str)` sketch could not do that). It merged on
2026-09-20 and is in **mfsk-core 0.11.0**. The migration is done
(2026-09-22):

1. `crates/sdroxide-digi/Cargo.toml` pins `mfsk-core = "0.11"`; the
   `madmedicnl/mfsk-core` git pin is gone.
2. The FT8 and FT4 decode requests carry
   `.also_accept(|m| m.callsigns().all(is_cb_compatible_call))`, where
   `is_cb_compatible_call(c) = wsjt77::is_plausible_call(c) ||
   sdroxide_types::is_cb_callsign(c)`. The union matters: the hook bridges
   `base || predicate`, and a message only lands when *every* callsign field
   passes, so a mixed "standard + CB" pair needs the coexist gate, not the CB
   grammar alone. The encode ladder instead uses
   `is_packable_call(c) = wsjt77::is_valid_callsign(c) ||
   sdroxide_types::is_cb_callsign(c)` at the three `rung4`/hashing gates,
   because stock `is_valid_callsign` refuses CB calls (the fork pin used to
   widen the validator itself).
3. `Cargo.lock` carries mfsk-core 0.11.0 from crates.io; the
   `madmedicnl/mfsk-core` source is gone.
4. The 11 m CB decodes still pass — the `cb_calls_pass_the_decode_gate`,
   hashed-pair pack and sensitivity tests cover them.
5. Two knock-ons of 0.11.0 worth remembering: the FT4 `sniper` is gone (the
   targeted FT4 pass is now a ±250 Hz wide-band `DecodeRequest` with an
   `ap_hint`), and #386's field-based filter no longer drops the FT8 EU-VHF
   contest exchange (`i3 = 5`), so the dedicated `ft8_eu` rescue pass over the
   FT8 slot was removed and `ApHints::eu_vhf` no longer gates the decoder —
   `contest_selected()` still feeds it, but decode_slot ignores it.
   The `ft8_eu` module itself stays: packing, the eu hash table and the
   exchange parsing (`eu_vhf` in modem.rs) are all still live.

### Modifier suffixes on an 11 m call (fixed 2026-09-29 — read this before the wide grammar)

**WSJT-CB accepts `/P`, `/MM`, `/QRP`, `/F1` on an 11 m call, and so do we
now.** This is plain WSJT-CB behaviour, not a fork experiment, and nothing has
to be switched on. Their README says it plainly: the CB regex was used to
**extend** `Radio::is_callsign` "so CB calls are treated as valid callsigns" —
they *widened* the standard rules rather than replacing them, so the
portable-style suffixes the standard rules already knew ride on a CB base call
unchanged.

**The bug was ours.** The fork's gate was `is_valid_callsign || is_cb_callsign`
— a *union of two narrow predicates*, where theirs is a *widening*. Neither half
of ours knew a suffix, so `19DC373/P` was rejected even though `pack77_type4`
packs it. The fork could **decode the station and then not answer it**, on a
two-way band. Fixed inside `cb_shape`, so the decode gate, the pack ladder, the
QSO machine's bare-call recognition and the country lookup all get it through the
one grammar; no call site changed. No wire change, no `PROTO_VERSION`.

- The suffix is `1..=4` of `A-Z0-9`. That is deliberately the general rule, not
  a transcription of the ham suffix list, which is open-ended for event callouts
  and is not ours to pin.
- **`CB_CALL_MAX_LEN = 11` is now enforced in the grammar**, from the wire
  rather than from taste: a non-standard call is a 58-bit base-38 number and
  `38^11 < 2^58`. One character past it, the pack ladder drops the exchange and
  sends the call alone — pre-existing degradation, not new.
- **The 11 characters are the WHOLE identifier, suffix included** — this is the
  part that is easy to get wrong, and the operator has to be told. `pack77_type4`
  caps `nonstd.len() > 11` on the **whole string**, `/zzz` and all, so
  `19DC3733/P` (10) and `19DC373/QRP` (11) go out and `19TST1001/QRP` (13) is
  **unsendable by anyone**. The wide grammar changes the call's *shape*, never
  its *length* — the ceiling applies identically with the toggle on or off.
  **Proven on the bench (2026-09-29):** `19DC373/QRP`, `19DC373/P`,
  `19DC3733/P` and `19DCG3733/P` all transmit; `19TST1001/QRP` does not activate
  transmit, and that is correct rather than a regression. The last two are
  wide-grammar shapes, so the operator had the **WIDE CB CALLSIGNS** toggle on.
  Because an **activation callsign is chosen by typing it**, the rule is
  enforced where it is typed: `cb_call_length_problem` (`cb_callsign.rs`, with
  `looks_like_cb_call` — CB calls open with a digit, amateur calls with a
  letter, which is what keeps the warning off ham operators) drives a live
  amber line under **Settings → General → Callsign**. It is **CB-only** by
  construction. Pinned by `the_eleven_character_ceiling_counts_the_suffix`,
  `the_length_warning_is_citizens_band_only` (types) and
  `an_over_long_eleven_metre_callsign_is_flagged_as_it_is_typed` (ui). The
  manual carries it as a boxed rule with a worked table, and the README's
  WSJT-CB section states it for activation organisers.
  A too-long 11 m call can still **arrive as free text** and will be shown as an
  unflagged line: `cb_callsign_in` returns `None` for it, so it gets no country
  and cannot be answered. That is the honest ceiling, not a display bug.
- The compound `N{1,3}L{1,2}/L{2}` is a **base shape, not a modifier**; the two
  are told apart by shape, and `999ZZ/ZZ` still resolves to country 999.
- A modifier never rescues a base that is not CB-shaped: `G47OXF/P` and
  `ABC373/P` are still refused.
- **`cb_country.rs` had its own hand-rolled copy of the shape check**, which is
  how the two drifted: the copy predated modifiers, so a modified call named no
  country and lost its flag on the decode row. The duplicate is **deleted** and
  the country is asked of `is_cb_callsign`. Do not reintroduce a second shape
  check — that is the bug class.

Tests: `a_modifier_rides_on_a_legal_cb_call`,
`a_modifier_does_not_need_the_wide_grammar`,
`a_modified_call_still_fits_the_type4_field` (types);
`a_modified_call_still_names_its_country`,
`finds_a_modified_call_in_a_message` (`cb_country`); the modified-call
assertions in `cb_calls_pass_the_decode_gate` and
`a_modified_cb_call_can_be_answered` (digi — the last one drives the whole
`CQ` → identity → `R+12` → `RR73` → `73` sequence through `pack_message`).

**The lesson worth keeping.** An earlier session read the `/LL` compound rule
off WSJT-CB's README, concluded modifiers were a protocol limitation, and spent a
long time drafting an upstream issue about it. The premise was false and the
whole argument rested on it. **The gate being narrower than the thing it
mirrors is the first hypothesis to test, not the last** — read what the reference
says it *does*, not only the pattern it quotes.

### The experimental wide CB callsign grammar (fork, 2026-09-29)

WSJT-CB's grammar is `N{1,3}L{1,2}N{1,3}`, mirrored by
`sdroxide_types::is_cb_callsign`. The 11 m community has outgrown it — three
letter groups and four-digit unit numbers are in use — so
`is_cb_callsign_wide` adds `N{1,3}L{1,3}N{1,4}` (and the slash form
`N{1,3}L{1,3}/L{2}`) as an **opt-in, off-by-default** superset. Both are picked
through `is_cb_callsign_with(call, wide)`; the strict 25-case WSJT-CB table is
untouched, so the two grammars stay separately pinned. A **modifier suffix is not
part of this** — both grammars accept it, so the toggle buys nothing there.

It is switched by `DigiConfig::cb_wide_callsigns` — appended, so
`PROTO_VERSION` 180 -> **181** (the field rides `DigiConfig` whole in
`SetDigiConfig`/`DigiStatus`, the same break as v179's CW-key fields). The flag
threads from the config to `Ft8Modem::set_cb_wide` (decode gate + pack ladder),
through `DecodeJob::cb_wide` to the decode worker, and into the QSO machine's
bare-call recognition. The toggle is on Settings → General, under the WSJT-CB
callsign grammar heading, labelled **experimental**, with the caveat in its
hover; the manual's 3.2.8 and the README's 11 m row carry the same.

Why it is experimental, and the limits to keep in mind:
- **WSJT-CB itself rejects the wider shape**, so the toggle widens what this
  station *hears* and lets it pack a wider pair, but a call that needs it may
  not be understood by a WSJT-CB station. That is the user-facing caveat.
- **11 characters** is the hard cap for a Type-4 non-standard call
  (`wsjt77::pack77_type4`), and it is the cap for the **whole identifier**,
  `/zzz` included — see the modifier section above. The widest *base* shape the
  wide grammar admits is ten, so `19DCG3733` fits and `19DCG3733/P` is exactly
  at the limit. No new wire format. The same cap is enforced in `cb_shape` as
  `CB_CALL_MAX_LEN`. An earlier note here claimed the widest shape fitted "with
  a character to spare" and was wrong once a suffix rode on it — that is what
  the bench test caught.
- **A CB callsign is not hashed on the main path — it is carried in the
  clear.** This corrects an earlier claim here, which said the "22-bit hash
  (4.2 M) is shared by every non-standard call, so a larger call set makes
  collisions likelier". That is wrong for the layout that carries a CB
  contact. `wsjt77::pack77_type4` encodes the non-standard callsign as a
  **58-bit base-38 number** — the actual characters, space-padded to eleven —
  and 38^11 (2.386e17) is under 2^58 (2.882e17), so the mapping is injective:
  every distinct callsign gets a distinct bit pattern. There is no collision,
  and no possibility of one call being decoded as another. The 12-bit hash in
  that function is on the *other*, standard, station — not on the CB call.
- The **one** place a CB callsign is hashed is the both-ends-CB, grid-less
  layout (`modem.rs`, the `h1 && h2` overlay), which writes each call as a
  28-bit `ihashcall(call, 22)` — the whole call, suffix included, which is what
  WSJT-X does. There a wider call population does marginally raise the odds of
  two calls colliding, and a collision can only mis-resolve a callsign the
  receiver has already heard. That is the pre-existing property of that layout.
  **This is documented upstream**: WSJT-CB's README has an "Important Note"
  describing exactly this, third parties seeing `<...> 26AT016`, and calls it
  "a protocol/encoding behavior, not an AutoSeq bug". We independently arrived
  at the same design, and the same `genStdMsgs` sequence.
- The strict grammar's "four-digit unit only behind a one-digit prefix" coupling
  is **dropped** in the wide grammar (confirmed 2026-09-29: any prefix).

Tests: `the_wide_grammar_adds_the_three_letter_shape`,
`the_strict_grammar_is_untouched_by_the_wide_one` (types),
`the_wide_grammar_packs_a_three_letter_cb_pair` and the widened gate assertions
in `cb_calls_pass_the_decode_gate` (digi).

**Tested and confirmed live on the bench (2026-09-29).** The operator switched
it on and the wider shape decoded and keyed as intended, so the toggle is
verified end to end, not only in the unit tests.

**What we took from WSJT-CB, in one place** (asked after the modifier
misdiagnosis, because it was not written down anywhere):
1. **The callsign regex and its 25-case acceptance table** — `cb_callsign.rs`.
2. **The CB country numbering and names** — `cb_country.rs`, their
   `cb_NNN_to_country` table in `logbook/AD1CCty.cpp`; the DXCC prefix on each
   entry is ours, added to reuse the flag/continent machinery.
3. **The message layouts and etiquette** — the `<HISCALL> MYCALL` identity
   opener, the one-call free-text answer, `R±NN` / `RR73` / `73` handling, and
   answering a CQ on the frequency it was heard on (`modem.rs`, `qso.rs`).
4. **The both-hashed, grid-less CB exchange** — the `h1 && h2` overlay and the
   28-bit `ihashcall`; theirs documents the same thing.
5. **The 11 m band entry and its default dial** — 11 m is a full two-way band,
   so every mode is offered on it, and **FT8 sits on CB channel 26
   (27.265 MHz)** as the WSJT-CB community settled (`CB11_DIALS` in
   `band_segments.rs`; the band accepts ~25 modes, `Band::M11` in `band.rs`,
   **not** the four the README used to claim).
6. **Nothing else.** The decoder is `mfsk-core` 0.11, the DSP is ours, and the
   grammar is our code — theirs was never a dependency.

### FT8 runs signal subtraction (2026-09-27)

`Ft8Modem::decode_slot`'s FT8 request now ends `.sic_early()`. mfsk-core 0.11's
**default strategy is a bare single pass — no subtraction** — while WSJT-X and
WSJT-CB run a multi-pass by default, so our FT8 was on the weaker path (FT4
already called `.sic_rounds(2)`). `sic_early` is mfsk-core's port of
`ft8_decode.f90`'s checkpointed `ndec_early` decode, a recall superset of flat
SIC; the message policy rides the checkpoint engine, so the CB gate above still
applies to every checkpoint's candidates.

The evidence, because "it decodes more" is not one:

- **Deterministic regression test** `a_weak_signal_under_a_strong_neighbour_is_recovered`
  (`modem.rs`, not `#[ignore]`d): a signal ~14 dB down, 10–30 Hz off a strong
  one, decodes with SIC and not without. It fails on the bare single pass —
  verified by neutering the `.sic_early()` call.
- **Real busy slot** (`mfsk-core/embedded-poc/assets/qso3_busy.wav`, the
  WSJT-X sample corpus, NOT committed here — mfsk-core owns it): single pass 12
  decodes / 20 ms, `.sic_early()` **22 decodes / 864 ms**. The ten extra are the
  weak ones the method exists for (−18 … −7 dB), several masked by strong
  neighbours at −4…+16 dB within 50 Hz. Cost ~6 % of a 15 s slot, on the decode
  thread.
- **Sensitivity floor unchanged**: the `sensitivity` sweep still reads −24 dB at
  the same sigmas, and the CB layouts (`CQ 26AT715`) still decode — SIC adds
  recall without moving the single-signal floor.

**WSJT-CB 1.4.0 "MBDecoder" — checked 2026-09-27, and it is half ours already.**
WSJT-CB's `1.4--scarlet` branch brands its decoder work **MBDecoder** (an
artwork logo, not a symbol). Its substance is a backport of WSJT-X 3.2.0-rc1's
FT8 **multithreaded-decode (MTD) reliability** fixes — coordinated residual
buffers, per-worker spectra, the `known`-dedup that stops concurrent threads
re-deriving the same message — plus CB-callsign filter fixes in MTD. **mfsk-core
carries all of that** in its own staged engine
(`decode_frame_subtract_staged_with_ap_inner`: checkpoint A/B/C buffers, the
issue-#253 pre-subtraction scoping, the `known` atomic gate), and its `parallel`
feature is on in our build. So the MTD half is a dependency, not work to do.

What the measurements settled (scratch benches, 2026-09-27):

- **`sic_early` is not thread-bound.** On `qso3_busy.wav` it returns the same
  **22 decodes at 1, 2, 4, 8 and 16 rayon threads**, and barely speeds up
  (896 ms → 806 ms) — the checkpoints are *sequential by construction*
  (A → subtract → B → subtract → C), so MTD's value is *correctness under
  concurrency*, never extra recall for a single operator. A quiet/sparse sample
  (`191111_110130.wav`) still doubles (3 → 6) for 384 ms. **The recall needs no
  cores** — which is the good news for an 11 m receiving node.
- So the whole recall win we were missing was the one-line `.sic_early()` (see
  above); nothing about thread count or worker pools needs touching.

**If WSJT-CB 1.4 ships with an 11 m MTD claim we cannot reproduce**, the first
check is *not* thread count (measured flat) but whether their gains are the
same checkpoint recall we now have — i.e. compare decode sets on a shared
recording, not the count alone.

**It is now the operator's choice (2026-09-30).** The ~1.2 s price is real and
the checkpoint chain cannot be parallelised, so `DigiConfig::ft8_depth`
(`Ft8Depth`) selects the strategy and ships **Deep** — the recall above.
Measured on `qso3_busy.wav` with `.osd(true)`: **Fast** (one pass) 16 decodes /
31 ms, **Normal** (`.sic_rounds(2)`, flat) 19 / 412 ms, **Deep**
(`.sic_early()`) 22 / 1.14 s. The flat pass plateaus at **20** however many
rounds (2…8 measured), so only the checkpointed pass reaches 22 — and FFT-cache
reuse across the passes bought nothing (1.169 s → 1.137 s). `DigiConfig` rides
whole, so the appended field is a wire change: `PROTO_VERSION` 183 → 184. The
control is on the FT8 setup dialog; the plain single pass is still emitted first
whatever the depth, so the auto-reply staging is unchanged.

### The HD Radio capture harness

Upstream took `dielectric-coder`'s harness with #466, so it is no longer ours to
add: `crates/sdroxide-nrsc5/examples/hd_capture.rs` holds the
capture-to-channel-rate conversion, and upstream's own `decode_sample_capture`
in `crates/sdroxide-nrsc5/tests/decode_sample.rs` checks the chain end to end
against nrsc5's `support/sample.xz`, including **`HdDemod::backlog_drops() == 0`**
— the assertion that caught the one-value-per-frame pacing bug on air. It is
`#[ignore]`d (48 MB of fixture, real-time decode); run it with
`cargo test -p sdroxide-nrsc5 --release -- --ignored --nocapture`.

Off-air captures stay out of the tree: they are copyrighted programme material
and tens of megabytes. Keep one beside the tree and point the example at it.

### The ACARS decoder

The ACARS mode (`crates/sdroxide-dsp/src/acars.rs`) is the fork's, offered
upstream as #465. Two things about it are easy to get wrong, and were:

- **The block check is a reflected CRC-16** — polynomial 0x8408, initial value
  0, over the bytes **as received with their parity bits**, from the mode
  character through `ETX`, with the low BCS byte first. It is not CCITT-FALSE
  over parity-stripped bytes; that checks out on the decoder's own encoder and
  on nothing on the air. Verified against real frames in acarsdec's `test.wav`.
- **The demodulator needs carrier and bit-clock recovery.** It is ported from
  acarsdec's `msk.c` (half-sine matched filter, VCO bit clock, PI carrier
  loop), and the input is resampled to the 12 kHz those constants are defined
  at. Re-deriving the constants per rate does not work — 48 kHz produced
  nothing, and 48 kHz is what the engine feeds the decoder.

The end-to-end test is an ignored fixture over an off-air recording —
`an_off_air_recording_decodes` with `SDROXIDE_ACARS_SAMPLE=/path/test.wav`;
acarsdec's own `test.wav` works and decodes a real `F-GTAE H1` frame at 12 kHz
and 48 kHz. The synthetic end-to-end tests were removed: they fed the decoder
its own encoder's output, which is exactly what hid both bugs (acarsdec cannot
decode that audio either). Not ported: acarsdec's error correction, the
syndrome search that fixes a few parity/CRC errors; only clean frames decode.

### The FSK441 decoder

FSK441 is the original high-speed meteor-scatter mode and MSK144's older
sibling — 4-FSK at 441 baud on 882/1323/1764/2205 Hz, carrying the 43-character
PUA-43 alphabet (three dits a character) plus the single-tone `R26`/`R27`/`RRR`/
`73` shorthand, in a 30 s (and 15 s) T/R period. It is **not in mfsk-core**, so
this is the fork's own decoder, `crates/sdroxide-dsp/src/fsk441.rs`, a port of
the MIT [`Nythbran23/FSK441-PLUS`](https://github.com/Nythbran23/FSK441-PLUS)
reference (© Roger Banks GW4WND) against K1JT's 2001 definition. Four things
about it are easy to get wrong:

- **The rate is 11 025 Hz, not 12 kHz.** 441 baud is exactly 25 samples a dit at
  11025, and the whole front end (the 25-sample matched-filter window, the tone
  spacing, the sync search) is defined at that rate. `Fsk441Controller`
  resamples the 48 kHz tap to it, exactly as the mfsk-core modes resample to
  12 kHz — do not re-derive the constants at another rate. (The reference's
  own author switched from 48 kHz to 11025 for a −195 Hz systematic offset the
  non-integer ratio caused.)
- **The ping search is tone-selective.** The detector is a 10 ms block envelope
  of the *strongest of the four matched filters*, thresholded at 4× its median.
  A raw-power envelope would trip on broadband noise; the tone envelope does
  not. Runs shorter than 40 ms (the reference's `wmin`) are dropped, and a run
  whose decode has mean dit confidence below 0.35 is dropped too, so a noise
  burst cannot reach the decode list.
- **The shorthand is checked at the nominal tones, before the frequency
  refine.** A whole ping on one tone is `R26`/`R27`/`RRR`/`73`, not text — but
  `refine_frequency`'s four-tone objective is degenerate when only one tone is
  present and mis-aligns the tone set, so the shorthand gate runs on the
  nominal filters first and uses `tone_offset` for its displayed frequency.
- **Tone 3 never starts a character**, so the character boundary is the mod-3
  phase where tone 3 is least often dominant (`jsync`); the alphabet's `d0` is
  only ever 0–2.

The dit trim matters: the ping run is padded by a block either side, and
counting the silence dits at the ends drags confidence down and truncates the
last character — `extract_dits` trims on dit energy, not on the matrix extent.
The alphabet table is MSHV's (` 123456789.,?/# $ABCD FGHIJKLMNOPQRSTUVWXY 0EZ*!`),
which fills the reserved slots 46/47 with `*!` where K1JT's table leaves them
blank.

Receive only. `Mode::Fsk441` is appended (discriminant 51, `PROTO_VERSION` 175 →
176); `Fsk441Period` (15/30 s) is a `DigiConfig` tail field, so the panel's clock
comes from the period exactly as FST4's and Q65's do, and `Mode::slot_timing`
answers `None`. Tested with synthetic pings and the fork's own generator
(`a_clean_ping_decodes_to_its_message`, `a_mistuned_ping_is_refined_and_decoded`,
`a_weak_ping_still_decodes`, `a_single_tone_ping_is_the_shorthand`,
`noise_alone_does_not_decode`, plus the digi adapter round trip). **Checked
against an off-air recording:** the Sigidwiki *FSK441Burst* sample — a received
meteor ping from **YO2NAA** — decodes to `YO2NAA` and its `RRR` rogers through
the ignored fixture test `an_off_air_burst_decodes`
(`SDROXIDE_FSK441_SAMPLE=…`, a mono 11 025 Hz WAV; convert a capture with
`ffmpeg -i capture.mp3 -ac 1 -ar 11025 burst.wav`). The *FSK441TX* sample is a
**continuous** transmission rather than received pings, so the burst search does
not report it — that is what a ping detector is for — but its front end reads
`CQ` and callsign fragments through the heavy clipping. A live 6 m/2 m ping is
still the bench check. Offered upstream as an "isolate it" PR, **#555**, branch
`upstream-pr/fsk441` (upstream's `PROTO_VERSION` 165 → 166 there). **Taken
upstream on the 2026-09-25 merge** (`30af3e3a`), with review that tightened the
detector — a run longer than a meteor ping is judged strictly so a steady
carrier is never the shorthand, the noise floor comes from live blocks, the
frequency search is ±200 Hz, and space is `033` — so the fork carries that DSP.
What stays fork-only is **FSK441 transmit** (the message loops for the length of
the over), still open upstream as **#561**.

### The VDL2 window on a narrow front end

A fixed window target does not land on the same rung of the decimation ladder
on every front end, because `Ddc::rate_for` rounds to the *nearest* whole
decimation. `WINDOW_TARGET_RATE_HZ = 500_000` is fine on a 2.4 Msps RTL-SDR
(480 kHz, all fourteen channels) but on a **768 kSPS Airspy HF+** it rounds
1.536 up to 2 and lands on **384 kHz**, which reaches only **ten** of the
fourteen channels and drops **136.975 MHz, the worldwide Common Signalling
Channel** — so the HF+ appeared to decode only the middle of the plan while the
RTL-SDR decoded all of it (upstream issue **#548**). The target is now derived
from the device rate, `plan::window_target_rate_for`: the nominal target when
`rate_for` lands at or above the plan, else the device rate, which is the one
rung that always holds the plan when any does. Large front ends are unchanged;
768 k and 912 k now reach all fourteen. `plan.rs`'s own test only covered rates
≥ 2 Msps, which is how it slipped through — the sweep
`every_wide_enough_front_end_reaches_the_whole_plan` is the guard now. Offered
upstream as PR **#556**, and **taken upstream on the 2026-09-25 merge**, with a
review tweak to prefer the *narrowest* decimation rung that holds the plan. (The
engine already reports the shortfall in `vdl2_degraded` — "reaches N of the 14
channels" — so check that sentence on a report before reaching for the
arithmetic.)

### The recording silence auto-split

The MP3 recorder taps post-squelch audio, but the tap is filled every block
whether or not the squelch is open, so recording a quiet channel produced one
file that grew all afternoon through the gaps (upstream issue **#546**). The
REC popup now has an **Auto-record** row: armed, the recording follows the
receiver's squelch — a file starts when it opens and closes after 2/3/5/10 s of
silence — so each transmission becomes its own UTC/frequency/mode-stamped file
(the naming already carried that; a continuous session never split to use it).
It is **UI-owned and session-only**, the same shape as the "stop after"
deadline from #520: `rec_gate_tick` is the whole decision, unit-tested as
`rec_gate_splits_on_the_squelch`, and `poll_recording_gate` reconstructs the
engine's squelch (`passband_dbfs >= squelch_db`) from the published meter — no
wire type, no `PROTO_VERSION` bump. The squelch defines silence, so the chips
are only offered when one is set; a CAT radio's squelch belongs to the radio
and its meters carry no passband level, so it gets a sentence rather than a
recorder that would never close. Offered upstream as PR **#557**.

The maintainer's review of #557 (the chip width versus the reserved
`RxChip::width_label`, the tone squelch in the gate, our own transmit counting as
signal, the carried `RecGate` state, frames while the window is hidden, and
arming one timer clearing the other) is addressed on `upstream-pr/rec-silence`.
A follow-up on the same branch makes the **REC chip breathe** while a file is
being written: armed is a steady red outline and recording is a moving red fill,
so the two are told apart without widening the label the RX strip has no room
for. The fork carries the same chip change on `main` (`6052b3ad`); the pieces
are `rec_chip_fill`, the 120 ms frame clock while recording, and
`the_recording_fill_breathes_the_alert_red`.

**#557 is rebased and pushed (2026-09-28, later); #568 and #561 still are not.**
`upstream-pr/rec-silence` was reset onto `upstream/main` (`9257c363`) and
force-pushed as **`c9d6589c`**, one commit — the rebase squashed the three
originals, because the review fixes amended the shape the first commit
introduced and per-commit replay fought itself. The net code is the same. It
answers the **latest** three review points, which are the ones worth
remembering, because all three are still defects the fork had:

1. **The gate depended on something being drawn.** `poll_recording_gate` ran
   from a radio's *own* frame loop, and only the radio whose tab is on screen
   runs one — `multi` gives a hidden tab `drain_events` and a sign-in, nothing
   else. So a gate armed on a radio behind another tab never ticked at all, and
   the repaint was being asked from inside the loop that was not running. This
   is the review lesson "carried state needs all its edges", one level up: the
   state was correct and simply never consulted. The tick is now in the
   hidden-tab loop, and the gate asks for its own frames from inside the tick
   (`GATE_POLL_MS`) rather than from a panel.
2. **The gate's own stop looked like a manual one.**
   `was_recording && !recording && signal` cannot tell "the gate closed this
   file after the hold" from "the operator stopped it", so the gate held off and
   **the transmission that ended the silence run was the one dropped**.
   `RecGate.stop_asked` says which, and is carried until the recorder is seen to
   have stopped — two frames, because the answer comes back late.
3. **Clicking "off" cancelled a running Stop-after.** Arming the gate is
   choosing one of the two answers to when a recording ends; turning the row off
   is choosing *neither*. The decision is the named `gate_arm_clears_stop_after`.

**This is provable on the bench and was not proved.** The gate reconstructs the
squelch from the published meter, which is exactly the part a synthetic number
cannot vouch for; the **RSP1** on the bench can, against a keyed signal. See
"The bench" for the two registered radios, the loopback, and the squelch
setting that has to change first or the run proves nothing. Point 1 in particular
needs a *second* radio, and the bench **has** one — the SS9900v is a
`UsbAudio` radio — so it is testable as configured; an earlier note here said
otherwise and was wrong, having been written from a truncated reading of the
`Backend` enum.

**All three are on fork `main` too (`712871cc`)**, ported by hand — a
cherry-pick of `c9d6589c` would be wrong, since the fork already carries the
feature and only the corrections are new. (2) and (3) were losing recordings on
an unattended bench, and (1) split files only for whichever radio was being
looked at, which is the wrong way round for a monitoring fork. A fourth thing
deliberately **not** done, same as on the PR: a window the OS is not drawing at
all. `recording_stop_at`, auto mode and the reconnect countdown all run off that
same frame loop, so a real fix is a tick outside the UI thread — an app-wide
change, not something to smuggle in with a recording fix.

#568 (`panels/cw.rs`) and #561 (`panels/mod.rs`) are stale the same way and are
still to be rebased under the lifted restriction. **Read each thread before
rebasing**: if the maintainer has commented on the old head, the comment after
the force-push has to say how the rebase answers him, not just that it happened.

**Done (2026-09-24): upstream issue #533** (save decoded text). Every text
panel now carries a **SAVE** chip beside **CLEAR RX**, plus WSPR, PI4 and the
skimmer. The formatters live in one place, `app::save_text`: free-running text
(CW and the keyboard modes) is written as itself, the structured logs (ACARS,
DSC, NAVTEX, VDL2, HFDL, FSQ, UVPacket, packet, JS8) as one line per message
with a UTC stamp, and WSPR/PI4/skimmer as their spot lists. The file goes
through the existing `download::save` (native `rfd` dialog + browser Blob), so
the dialog and destination are the logbook export's. `digi_has_log` is the
cheap test that greys the chip. UI-only: no wire type, no `PROTO_VERSION`
bump. Offered upstream as PR **#558** (there without the DSC/UVPacket branches,
those modes being separate PRs). APRS and AtCHAT logs are the one gap left —
they have their own status structs and were not wired in this cut.

### The band menu's clarity pass (fork-only, 2026-09-24)

The fork's band popup had outgrown its own information architecture: the
**HF/VHF/UHF** filter chips and the **ALL** (`Band::Gen`) clear-band chip sat on
one unnamed row and read as one control though they do different things; the
OPERATE tab drew a **Primary modes** row and then a **Mode** row repeating the
same four chips; and the LISTEN tab's metre-band shortcuts looked like bands.
The pass renames the filters `HF BANDS`/`VHF BANDS`/`UHF BANDS` under a **Show
bands** caption, moves the clear-band action out to its own **ALL — no band**
chip with a hover saying it is not the filter, gives the metre bands an **SW
metre bands** caption, and folds the four reach-for modes into the Mode row
behind a divider. No behaviour change; the tests
`all_clears_the_range_filter_too` and
`the_band_filter_says_it_filters_and_toggles_off` pin it.

**Later — 2026-10-09, the operator's call.** The `BANDS` suffix came back off:
the filters read plain **HF** / **VHF** / **UHF** (the **Show bands** caption
above them already says what they are, and the shorter chips sit closer
together), and the clear-band chip is plain **ALL**, now **beside UHF** on the
same row (a divider and its own hover keep it from reading as a filter — the
operator asked for it there). The hovers are unchanged, so the distinction the
suffix was there to draw is still said in words.

**Later still, same day: no dropdown, and the mode chips in aligned rows.** The
dropdown was **abandoned** and the mode chips put back, laid out in **aligned
columns** — the operator's "trimmed" look — through `mode_chip_grid`. OPERATE is
**Mode** (15 analog/voice/broadcast-demodulator chips) then **Digital**
(`Mode::DIGITAL`) then **Wideband**; LISTEN is **Receive modes** (9) then
**Digital** then **Wideband**. `MODE_GRID_COLS = 5`, so it is five across where
the width allows.

**Do not use an `egui::Grid` there, and do not budget a cell from `chip_width`
alone.** Both grew the **docked** column over the operating panel below it —
`the_operating_panel_stays_inside_its_column` (the #643 guard) caught it,
`Rifp at 960 pt … crosses the dock edge at 681`. `mode_chip_grid` lays
fixed-width cells (`chip_width + MODE_GRID_CELL_SLACK`) in plain `ui.horizontal`
rows instead, wrapping when even one column will not fit.

**The docked column cannot be much wider than 280 pt — that is the constraint to
remember.** The operating panels below are built for ~680 pt (the FT8 QSO pane
runs on under ~677, the image panel's floors and the `digi_panel` split have been
made to scale but the rest have not), so a wider dock squeezes them and they
overrun the dock. A genuinely wide, five-across band menu wants to be its **own
window** (SDRuno's separate control panel), not a docked column — the open item.

**Fork-only on purpose — do not offer this upstream.** Upstream's band menu is a
single flat band row: no `BandFilter`, no LISTEN/OPERATE tabs, no Primary-modes
row, no metre-band shortcuts. Every problem this fixes was introduced by the
fork's own additions, so there is nothing for the maintainer to take. (Checked
2026-09-24: the cherry-pick onto `upstream/main` conflicts structurally because
`band_mode_menu` there is a different, simpler function.)

### The (tr)uSDX nG feedback (DL2MAN, 2026-09-24) — fixed 2026-09-26

DL2MAN reports that against a real nG radio the one-cable mode connects and
streams, but **the audio is distorted and the waterfall shows only a sliver**,
and **the dial does not follow the radio** (a 40 m radio showed as 20 m). The
two standing assumptions this fork made for nG were both wrong, and DL2MAN
answered the two blocking questions (2026-09-26); the fix is in
`crates/sdroxide-cat/src/trusdx.rs` and `src/lib.rs`, no wire type and no
`PROTO_VERSION`:

- **The nG receive rate is 7812.5 B/s** — the same as 2.00x, 8-bit unsigned
  (mid = 128), `0x3B` escaped to `0x3C`, one byte per sample. So
  `TRUSDX_RX_RATE_HZ = 7812` was **not** the problem and is unchanged. (His
  earlier CAT table's "4812 samples/s" was wrong.) One exception, now modelled
  rather than merely noted: **CW with the 1450 Hz ("1K4") filter runs at
  3906.25 B/s**. The profile cannot read the filter, so `TrUsdx::rx_rate`
  infers the half rate from nG + CW (the safer of the two guesses, and CW is
  where the pitch is watched) and reports it through
  `Protocol::rx_audio_rate_hz`; `AudioCatSource`'s `StreamResampler` follows
  `CatHandle::stream_rx_rate_hz` and brings the stream back to the nominal
  7812 Hz, so the analyser, the speaker resampler, the recorder and the
  decoders never see the rate move. `sample_rate()` therefore stays nominal.
- **A CAT command is safe mid-stream in nG**, which is what broke both
  symptoms. The framing is `[audio] ; FA00007074000; US [audio]` for a query:
  nG pauses its own stream, writes the reply *with* the trailing `;`, and
  resumes with `US`. 2.00x is the same shape but **omits the trailing `;`**
  (`FA…US…`), which the old parser, written for a `;` on every reply, did not
  take. So:
  - `poll_requests`/`dial_requests`/`tx_state_requests` are now **non-empty for
    nG one-cable** (the gate is `one_cable && !is_ng()`); before, the dial was
    never asked and so never followed.
  - The serial thread no longer brackets nG control frames `UA0; … UA1;`
    (`Protocol::brackets_commands` — 2.00x true, nG false). The old bracketing
    stopped and restarted a healthy nG stream on every poll, which is exactly
    the distorted audio and sliver waterfall.
  - The nG poll is capped to ~1 Hz (`Protocol::poll_hz_ceiling`): each poll is
    a splice in the audio, and the firmware drops the samples it produces while
    answering.
  - The demux now ends a reply at the earlier of the `;` and the resuming `US`,
    so both generations parse.
- The 2.00x in-band mode is **unchanged**: it still polls nothing and still
  brackets its commands, because a command written into a 2.00x stream kills it
  and it does not come back.

**Still not tested on air here** — no nG radio on this bench, DL2MAN's own
firmware could not be flashed. The fix is unit-tested structurally (the new
tests are `ng_one_cable_polls_so_the_dial_follows`,
`ng_control_frames_are_not_bracketed_where_legacy_is`,
`only_ng_one_cable_caps_the_poll_rate`,
`a_reply_without_a_delimiter_resumes_on_us`,
`a_reply_split_at_the_resume_marker_waits` and the `lib.rs`
`a_poll_ceiling_slows_the_dial…`; for the CW half rate,
`ng_cw_reports_the_half_receive_rate`,
`the_radios_own_mode_reply_sets_the_receive_rate` and the source's
`a_half_rate_stream_keeps_its_tone_at_the_nominal_rate`; for the transmit
sequence and the enable echo, `ng_one_cable_keys_with_the_reference_sequence`,
`an_ng_stream_opens_with_us_and_a_high_byte`,
`the_stream_enable_echo_is_swallowed` and `a_split_stream_enable_echo_waits`).
The on-air check is clean audio, a waterfall spanning the passband, and the dial
following a frequency changed at the radio — and, on the 1K4 CW filter, audio at
the right pitch. `tools/trusdx-probe/README.md` §nG names the rest.

### The (tr)uSDX family

The fork's (tr)uSDX support is one upstream PR, **#498**, branched from
`upstream/main` and deliberately not merged here. It adds `CatFamily::TrUsdx`
(a fourth Kenwood dialect with a thin command set) and a **per-radio choice of
audio path**, `CatConfig::trusdx_audio` (`TrUsdxAudio`), because the radio has
no sound card of its own and two ways to be heard:

- **One cable** (default) — receive and transmit audio ride the CAT serial link
  as the firmware's own 8-bit stream (`UA1;`/`US` framing: ~7812 samples/s in,
  the host pacing 11520 out). The firmware **cannot take a CAT command while
  its stream is running** — one kills the stream and it does not come back — so
  this mode polls nothing and brackets every control frame `UA0; … UA1;`.
- **USB sound card** — audio from a card on the 3.5 mm jack; control still over
  USB, and the rig is polled like any other CAT rig.

Only the in-band mode streams and only it suppresses the poll; both hold DTR
high (the radio's reset line) and switch any leftover stream off at open.
`PROTO_VERSION` went 156 -> 157 upstream for the new `CatConfig` field; on the
2026-09 merge upstream moved on to **158** (SSTV styling also took 157, and
#498's (tr)uSDX took 158, both now upstream) while the fork counts on to
**164** — see the register in `crates/sdroxide-proto/src/lib.rs`. (#499 was the
sound-card-only version and is closed as superseded by #498 — the modes are
not alternatives, and the operator is the one who knows which fits.)

The bench harness is `tools/trusdx-probe/` — PySerial scripts against the
serial port, with a README of what each measures (transmit ones need a dummy
load). The findings worth not re-deriving are there: the receive rate is 7812
samples/s and not the published 7825, `0x3B` is escaped to `0x3C`, a CAT
command written into a live stream kills it, and DTR is the reset line.

### The (tr)uSDX nG family (fork, untested on air)

DL2MAN's rewritten **nG** firmware ([dl2man.de/ng](https://dl2man.de/ng),
operating guide §9) keeps 2.00x's `UA`/`US` audio-in-the-CAT-link framing but
changes the transmit side and adds a level extension. It is a **separate
`CatFamily::TrUsdxNg`** ("(tr)uSDX nG"), on branch `fork/trusdx-ng` and merged
to fork `main` on 2026-09-22 (before any on-air confirmation — the user's call,
it is niche and fixable after release); the 2026-09-25 merge renumbered it to
**v173**. The profile is the
same `trusdx.rs`, parameterized by generation (`TrUsdx::new_ng`), so the
receive demultiplexer and the `UA`/`US` framing are shared code. What differs
from 2.00x, each a silent failure (or a visible one) if got wrong:

- **Receive half rate in CW.** With the 1450 Hz filter nG runs its receive
  chain at half rate, so the link carries 3906.25 instead of 7812.5 B/s. The
  profile cannot read the filter, so it infers the half rate from nG + CW and
  reports it through `Protocol::rx_audio_rate_hz`; the source resamples back to
  nominal, so the engine never sees the rate move. See the feedback section
  above.
- **A CAT command is safe mid-stream.** nG pauses its own stream, answers with
  the trailing `;`, and resumes with `US`; 2.00x kills its stream on any
  command and omits the `;`. So nG is **polled** (dial/mode/tx-state,
  `Protocol::brackets_commands` false there so the serial thread writes its
  frames bare, and `Protocol::poll_hz_ceiling` caps it to ~1 Hz), while the
  2.00x in-band mode polls nothing and is still bracketed `UA0; … UA1;`. See
  the feedback section above.
- **Transmit rate 4807.69 B/s** (`TRUSDX_NG_TX_RATE_HZ = 4808`), not 2.00x's
  11520 — the transmit slot is `20 MHz / (64 × 65)`, and 2.00x's surplus is
  thrown away. The serial thread's `TxPace` now takes the rate from
  `Protocol::tx_audio_rate_hz()` rather than the old constant.
- **Transmit delimiter escape `0x3B → 0x3A` for both generations**
  (`TRUSDX_TX_ESCAPE_TO`), reconciled to the reference client's shared
  substitution; the radio does not reverse it, so the value only has to avoid
  `0x3B`.
- **The transmit sequence is `;TX0;` → `US` + audio → `;` → `RX;`.** Taken
  from DL2MAN's own ARDOP client (`dl2man.de/ARDOP/client`, v0.4.34, source
  inline in the page as JS) and his device specification. The leading `;` on
  the key ends the running receive audio first, or the firmware reads
  `T`/`X`/`0` as samples (his v0.2.0-a40 fix). `US` opens the transmit frame,
  the first byte **≥ `0x80`** guarantees the stream is not mistaken for
  commands (`Protocol::on_tx_stream_start` + `TRUSDX_NG_TX_START_BYTE`), and
  the bare `;` of `Protocol::tx_stream_close` ends it *before* the unkey — omit
  it and the firmware is still reading samples when `RX;` arrives and swallows
  it. 2.00x keeps the bare `TX;`/`RX;` (the serial thread brackets its frames),
  so the sequence is gated on `one_cable && is_ng`.
- **The stream-enable echo is swallowed.** Re-asserting the receive stream
  injects a bare `UA1;`/`UA2;` (no `US`) into the running audio; its `;` is not
  a frame delimiter, and reading it as one leaves the following audio to be
  parsed as a frame with no terminator — a stall. `TrUsdx::parse` strips the
  exact 4-byte sequence anywhere in the audio and holds a split one at the tail;
  the reference client needs a whole echo state machine (`ngHardSwitch`) for the
  same thing. 2.00x is unaffected (it never re-asserts mid-stream).

Level control (the user asked for it): `AG0nn;` volume 00–31
(`CatConfig::trusdx_ng_volume`), `GTn;` gain 0 off / 1 on / 2 DIGI
(`CatConfig::trusdx_ng_agc`, `TrUsdxNgAgc`; `Auto` follows the mode on the mode
frame), and `UA2;` to switch the radio's own speaker off
(`CatConfig::trusdx_ng_speaker`). nG answers neither command and stores neither,
so they go out in `open_requests`.

Upstream's form of this family is draft **PR #545**, still open; on fork `main`
the family sits at **v173** and the band-openings addition (`ServerMsg::
BandOpenings`) at **v174**, so they no longer collide. Band-openings is upstream
**#537**, still open, and the fork's copy drops out once that lands.

**Not tested on air here** — the fork's radio is not calibrated for nG, so the
firmware could not be flashed. It is unit-tested structurally (`trusdx.rs`
covers the rate, both escapes, the `;TX0;`/`US`/`;` transmit sequence, the
stream-enable echo and the receive half rate); the on-air checks are named in
`tools/trusdx-probe/README.md`, and
forum testers are willing. Confirm on a real nG radio before offering it
upstream. It is a new family + `PROTO_VERSION` bump, so it is an "isolate it"
PR from `upstream/main` when the time comes.

### The HFDL core (issue #497)

An HFDL (ARINC 635) decoder has been requested upstream as #497 and scoped on
the issue. The enabling find is that the hard part already exists under a
permissive licence: **`airframesio/xng` is MIT/Apache-2.0**, and its
`xng-mode-hfdl` crate (`HfdlChannelDecoder::process(&[Complex<f32>])`) runs at
`CHANNEL_RATE = 12_000` with a +1440 Hz subcarrier, which is exactly the
complex tap the engine already hands VDL2, ADS-B and AIS (`on_rx_iq`). Its
`PROVENANCE.md` records a clean-room implementation from ICAO Annex 10 /
ARINC 635, with dumphfdl consulted as facts only, so it is safe to depend on
from this GPL-3 project.

Route A (using the xng crates) was validated off-air in a scratch build: the
reference 21 931 kHz capture decodes its squitter field-for-field (GS 4
Riverhead, frame 2397, systable 52). The dependency tree is modest (rustfft,
num-complex, chrono, serde, crc; no protobuf). The requester answered both
questions on #497 (2026-09-19): he defers to our judgment, so **Route A is
taken**, staged as decoder + decode log first, then the aircraft map, then the
system table. It began as a cargo git dependency; the maintainer asked on the
PR review to vendor it, so **xng is now a pinned submodule at `vendor/xng`**
(rev `096c805`), reached by path like `vendor/rade_c` — the build takes no
network dependency of its own. One knock-on: `cargo metadata` reads every
workspace member's manifest, so the path dependency means even the *wasm* CI
job needs the submodules checked out (fixed in `release.yml`/`windows-msi.yml`).
Work is in `crates/sdroxide-hfdl` (types in `sdroxide-types/src/hfdl.rs`, lane
in `sdroxide-radio`'s engine, panel in `sdroxide-ui`'s app). The demod's own
receive chain expects a 24 kHz lane centred on the channel (the validated
off-air input), USB subcarrier +1440 Hz handled inside xng's
`HfdlChannelDecoder`.

  Offered upstream as draft **PR #509** (branch `upstream-pr/497-hfdl`, based
  on `upstream/main`, PROTO_VERSION 158 -> 159 there). The fork's copy is on
  `main` with PROTO_VERSION 164.

  The second stage — the **aircraft map** — is in, on top of the decode log:
  xng already lifts a normalized `details.position {lat, lon, aircraft_id,
  icao, flight}` out of a performance-data (0xD1) or frequency-data (0xD5)
  HFNPDU and drops the all-zero not-yet-acquired fix, so the worker parses that
  into the typed `HfdlFix` a decode now carries (`HfdlDecode::position`). The
  UI keeps its own latest-fix-per-identity table (`crate::hfdl_map`, keyed by
  `HfdlFix::key()` — ICAO, then GS-local alias, then flight, then position) so
  an aircraft stays plotted after its earliest decodes scroll out of the log's
  rolling window; a plot is retired after 30 minutes of silence. The window is
  now a draggable split — log left, map right, fraction in
  `ViewState::hfdl_split_fraction` — and the log row shows the fix rather than
  the JSON it arrived in. Only unit-tested here: the off-air capture carries a
  squitter and no aircraft positions, so the map has **not** been seen on real
  HFDL traffic. The still-open question for the third stage (the system table)
  is what it adds over the squitter's frequency list.

  **HFDL is now a `Mode`** (`Mode::Hfdl`), not a floating window, so its panel
  docks under the waterfall like ADS-B/AIS: the System box's HFDL chip (bottom
  row — an eighth *top*-row chip pushed the desktop strip to a third row) now
  *selects the mode* rather than toggling a window, and the band menu's Digital
  row offers it too. It is a panel-owning lane (`has_bottom_panel`) but not
  `is_digital` and not a `is_wideband_lane`; entering it brings the dial onto
  `HfdlSettings::frequency_hz` (chip and panel both push `SetVfo`), and the
  channel choices live in the panel because HFDL is a plan of assigned
  frequencies, not one worldwide channel. On a phone the panes are DECODES and
  MAP. `Band::Sw` now accepts `Mode::Hfdl` (amateur bands and Gen already
  accepted anything). Adding the variant rippled through the CAT/TCI/smartsdr/
  rigctld/speech mode tables — all map it with the other receive-only lanes,
  since no rig has an HFDL position and the lane is fed raw I/Q.

  Separately, the ⚠ banner above the spectrum now offers a receive-only radio a
  **Listening controls** button: public SDRs and any other source that answers
  `is_transmit_capable()` false get the full transmit UI otherwise, and the
  button switches this radio to its listening screen (per-radio `hide_tx`, the
  same switch as Settings → Radio), retuning nothing. Dismissing it holds for
  the session.

### The LISTEN tab offers every mode on every band

The band/mode menu's OPERATE tab greys out a mode the current band does not
carry (`Band::accepts_mode`, whose service-band table says an FM broadcast is
not amplitude modulated and so on), and the engine refuses the same pair at
`Command::SetMode` so a remote client cannot pick it either. The **LISTEN**
tab deliberately does neither: its mode chips never grey for the band, and
they send **`Command::SetModeListen`** — appended after `SetHdProgram`, same
mode change without the band rule. The point of the listener's screen is to
explore the dial, and trying a decoder where the table would not put it is
the exercise. Transmit legality is untouched: `SetModeListen` only chooses
what is received, and the band lockout and the TX rails still decide what may
leave the radio. The station's own limits still grey a chip (HD Radio with no
`libnrsc5`, issue #488), because those are not a band opinion.

`crates/sdroxide-radio/tests/listen_mode_unlocks.rs` pins both halves: AM is
refused on the FM broadcast band via `SetMode`, and applied there via
`SetModeListen`.

The band/mode menu's **Band** row leads with **HF / VHF / UHF** and **ALL**
together, as the coarse choices. HF/VHF/UHF are a filter (`BandFilter`,
session-only on the app, not persisted) that narrows the band chips shown;
the lit chip toggles itself back to all, so there is no separate "show
everything" chip. ALL is the band `Band::Gen` — general coverage, which
clears the band so the dial goes anywhere — and it rides with them rather
than in the band list, which is why `band_chip` no longer special-cases it.
Its label is "ALL" in `Band::label()` (it reads "GEN" nowhere the operator
sees now; the bandplan overlay was updated too, and the wire/JSON name stays
`Gen`). The filter classifies by the *middle* of `Band::edges()`, so the
military airband (225–400) reads UHF and FM broadcast (87.5–108) VHF, and a
band with no edges (`Gen`) is in every slice. Neither the filter nor ALL
moves the dial (ALL clears the band; the filter only hides chips).

### The eight listening tools audited from OpenHamClock

The ROADMAP's Phase 3 audit of [`accius/openhamclock`](https://github.com/accius/openhamclock)
(MIT) listed eight tools worth adapting. Five are in the fork as of 2026-09-23,
each on its own branch off fork `main`, **local only — not pushed, no PRs**, and
**not yet merged into `main`** (they are for the operator to test first):

- **Gray line on the flat maps** (`fork/gray-line`) — the flat maps drew day and
  night identically; the terminator lived only in the 3D scene's shaders.
  `sdroxide_solar::ephem::night_shade_rgba(width, height, unix)` produces an
  equirectangular RGBA day/night/twilight image off `subsolar_point` — the same
  Sun `is_daylight_at` reads the band-conditions half from, so the shade and a
  "day"/"night" verdict cannot disagree. `night_shade(elev)` is the ramp (0 in
  daylight, 1 below −14°). In the UI a `NightShade` texture
  (`crates/sdroxide-ui/src/prop_map.rs`, rebuilt at most once a minute) is painted
  over the heat and under the continents by the new `paint_world_texture` helper
  in `widgets/worldmap.rs`, behind a `ViewState::map_night` **NIGHT** chip that
  sits with HEARD ME (independent of PROP). Wired into all three flat-map callers
  (FT8/FT4/FT2, WSPR, JS8). Tests recover the subsolar cell being unshaded and
  the antipode at full night; the ramp is monotonic.
- **Meteor-shower calendar** (`fork/meteor-calendar`) — `sdroxide_solar::meteor`:
  a static IMO table (15 major showers, windows/peak/ZHR/radiant/velocity/parent)
  and pure `radiant_altaz(lat, lon, ra, dec, unix)` built on `gmst_deg`. The
  Earth-fixed radiant frame is the same one `subsolar_point` uses — pinned by a
  test that recovers the Sun's own RA/Dec from its ecliptic longitude
  (`sun_geocentric` returns `(lon, r)`, **not** RA/Dec — the first test to assume
  otherwise is what caught it). `active_at` returns what is active now, strongest
  first; the BANDS window's `meteor_section` lists it with peak ZHR, a PEAK flag
  and whether the radiant is above the station's locator.
- **IBP beacon checker** (`fork/ibp-beacons`) — `sdroxide_types::ibp`: the 18
  NCDXF/IARU beacons, the 5 bands with offsets `0/17/16/15/14`, and the 180 s
  cycle aligned to UTC midnight (`slot_at`, `seconds_left_in_slot`,
  `active_at(unix, from)`). Geometry from `geo::bearing_deg`/`distance_km`. The
  BANDS window's `ibp_section` lists each band's current beacon with bearing and
  distance, and the window now calls `repaint::after_ms(ctx, 1000)` while open so
  the slot and countdown stay live. The offsets are the easy thing to get wrong:
  a beacon steps *up* a band every 10 s, so the band N slots earlier is
  `(18 - N) % 18`, which puts YV5B on 17 m at slot 0, not 4U1UN. A test walks a
  whole cycle per band and asserts every beacon is visited exactly once.
- **Space-weather trend** (`fork/space-weather-trends`, first slice of ROADMAP
  item 4) — the planetary-K product already carries a week of observed bins in
  front of the three days predicted, and only the forecast half was drawn. New
  `aurora::recent` returns the observed bins that have *ended* (`p.unix + 10_800
  <= now`), which is what keeps it disjoint from `upcoming`'s in-progress bin —
  the first cut used `p.unix <= now` and leaked that bin into both halves, which
  the test caught. The AURORA panel now draws observed (solid) + forecast (wash)
  as one trend with the boundary marked. **Still open from item 4:** solar-wind /
  Bz / proton sparklines (a new SWPC product to fetch and parse) and the
  solar-cycle chart.
- **Local solar time** (`fork/local-solar-time`, ROADMAP item 5) —
  `broadcast::local_solar_hhmm(utc_hhmm, lon_deg)`: four minutes a degree, from
  the site coordinates EiBi carries. A **SOLAR TIME** chip on the SCHEDULE window
  puts it on each row. The operator's call was to label it **solar, not local**:
  it is mean solar time with no DST and no zone borders, and a true civil time
  zone would need a country-polygon dataset (OpenHamClock's `geo-tz`) this fork
  will not carry.

Not started: **D-RAP absorption map** (item 6, a new SWPC feed) and the
**azimuthal map** (item 8, a QOth-centred projection — the flat map widget is
equirectangular throughout, so it is a rework of `widgets/worldmap.rs` rather
than a bolt-on).

**Offered upstream 2026-09-23**, each a single commit branched from
`upstream/main` (the fork's `main` was pushed at the same time, `1995973d`).
**All four non-draft PRs were taken on the 2026-09-25 merge** (`edbd2961`,
`999292fc`, `ec959372`, `7e6a353b`, `830465e4`), each with review work, so the
fork's copies dropped out:

- **PR #541** (`upstream-pr/gray-line`) — the grey-line shading, no HEARD ME
  (fork-only) and no JS8 map (upstream has none), so it is just the two panels
  upstream carries. **Taken.**
- **PR #542** (`upstream-pr/meteor-calendar`) — the IMO table and
  `radiant_altaz`; upstream corrected the Quadrantid rate and the Taurid and
  Geminid windows and added the Daytime Arietids. **Taken.** The fork's per-label
  hover fix (a `Label` swallows the row's hover) is still only ours.
- **PR #543** (`upstream-pr/ibp-beacons`) — the 18-beacon schedule; upstream
  placed the beacons at the NCDXF's published locators. **Taken.**
- **PR #544** (`upstream-pr/space-weather-trends`) — the Kp observed history.
  **Taken.**
- **PR #545** (`upstream-pr/trusdx-ng`), **draft, still open** — the nG CAT
  family, marked honestly untested on an nG radio with the on-air checks named
  in the body. `PROTO_VERSION` 164 → 165 on that branch (upstream's register).

**Local solar time is fork-only, not offered:** `broadcast::local_solar_hhmm`
and the SOLAR TIME chip live on the SCHEDULE window, which upstream does not
have. The cherry-pick for it did not apply, and it should not — the feature has
no upstream home.

The band-openings fork build stays on `main`; its upstream form is still
**#537**, and the fork's copy drops out once that lands.

Post-review follow-ups (2026-09-23), all merged into local `main` and pushed:

- **NIGHT is on every flat map.** ADS-B, AIS, APRS and HFDL each gained a
  `night: Option<TextureId>` parameter to their `show`, painting the overlay
  under the base through a shared `widgets::worldmap::paint_night` helper, and
  their own **NIGHT** chip (`SdroxideApp::night_chip`) above the chart. The
  operating panels use the same chip from `prop_map_controls`. One shared
  `ViewState::map_night` flag means switching it on anywhere lights the
  terminator everywhere.
- **The SWL switch is in Settings → UI**, not only the per-radio Radio tab:
  `settings_ui_tab` now takes `radio: Option<&mut RadioConfig>` and writes
  `hide_tx` directly, so a listener can turn SWL mode on without a restart.
- **No OpenHamClock reference is UI-visible.** The only one that ever was —
  the OPENINGS chip's tooltip in `spots.rs` — is gone; references now live in
  code comments and the README's Acknowledgements only.
- **The band-opening detector is merged into local `main`** from
  `fork/live-band-openings` (the fork build with the 11 m decode feed). The
  merge took **PROTO_VERSION 167 → 168** for `ServerMsg::BandOpenings`: the
  (tr)uSDX nG family already held 167 on `main`, so band-openings moved up.
  The `upstream-pr/band-openings` PR (upstream #537) is still open and is the
  thing to land first; it will need a rebase onto the post-2026-09-25
  `upstream/main` (the fork's copy is now v174).

Second review round (2026-09-23), same branches now on `main`:

- **The terminator is stroked.** `ephem::on_terminator(elev)` and a
  `NIGHT_LINE_INK` band in `night_shade_rgba`: cells within 1.2° of the solar
  horizon take a bright neutral line ink at ~full alpha, because the soft shade
  alone read poorly over some themes' land colours. The line ink is deliberately
  not a theme colour — the overlay is uploaded as bytes the same on every theme.
  **Reversed 2026-09-23:** baked into a 360×180 raster, one cell is ~1° of
  latitude, so the "line" reads as a fat blocky band at map scale and looked
  worse than the shade. Reverted to shade-only. A thin terminator would have to
  be stroked as a vector polyline along the zero-elevation locus by each map
  widget in screen space, not painted cell-by-cell into the texture.
- **Meteor and IBP hover target the labels, not the row.** A `Label` defaults to
  `Sense::hover()` (and `selectable_labels` adds click+drag), so it *takes* the
  hover and the enclosing `ui.horizontal(...).response` is never hovered while
  the pointer is over any text. That is why `.response.on_hover_text` did
  nothing; the tooltip now hangs on each label.
- **The BANDS window's table is in the 3D view.** `SolarUi` carries
  `band_conditions` / `band_activity` / `psk_activity`, published by the host in
  `viewport` each frame like `prop`, and `bands_info_panel` draws the
  CONDX/WSPR/PSK/PATHS/REACH table under the `BANDS OPEN` chart. So conditions
  can be read from the globe without opening the BANDS window over the main
  view. Note the browser `/solar-ws` relay does not carry these yet, so the
  panel is absent in the browser tab. The right-hand corner boxes share one
  width: `Place::Corner` carries a `w`, and `SolarUi::corner_w` is published by
  the BANDS table (one-frame settle) so the stack reads as one column. The
  table's footer is laid out wrapped to the table width, not `layout_no_wrap`.

### The LOG11DX WSJT bridge (for the auto-mode and DX-radar work)

The bridge the CB side interoperates with is installed in the Wine prefix on
this machine: the app in
`~/.wine/drive_c/users/druid/AppData/Local/LOG11DX WSJT Bridge/`, its config and
caches in `~/.wine/drive_c/users/druid/AppData/Roaming/LOG11DX WSJT Bridge/`.
It is a PyInstaller one-file Python 3.14 program; `pyinstxtractor-ng` unpacks
its two modules from the `.exe`, and this box's Python 3.14 unmarshals them.
What the unpacking settled:

- **It never fetches a spot feed.** The bridge listens to WSJT-X UDP and does
  two things: uploads logged QSOs and dupe-checks calls. Its "DX Radar" tab is a
  **local** map — the WSJT-X decodes it hears, placed by callsign prefix from
  `assets/dx_radar/dxcc_prefixes.json` (id → country) against
  `world_110m_countries.json` (GeoJSON) and `world_bitmap.png`, with alerts
  keyed `new-call:`, `new-prefix:`, `new-grid:`, `strong:`, `opening:`. The API
  token is not needed for the radar; it is needed for the upload and dupe calls.
- Endpoints: `POST /api/wsjtx/upload-qso.php` (already mirrored in
  `crates/sdroxide-net/src/upload.rs`), `GET /api/wsjtx/token-status.php`, and
  **`GET /api/wsjtx/check-dupe.php`** — query `call` (required) plus optional
  `mode`, `band`, `freq`; header `Authorization: Bearer <token>`,
  `User-Agent: LOG11DX-WSJT-Bridge/0.2`; timeout `min(config, 4 s)`; JSON with
  `ok` and either `alert {title, body, details[]}` or `last_qso
  {mode, band, frequency, date, time}`. Update channel:
  `/wsjt_bridge/latest.php`, `/wsjt_bridge/download.php`.
- Its local `dxradar_recent_spots.json` entries carry `call`, `mode`, `snr`,
  `df`, `grid`, `message`, `low_confidence`, `off_air`, `calls`, `utc`,
  `cache_time`.

`check-dupe.php` is the find that matters for auto mode: on 11 m the operator's
authoritative log is LOG11DX, not the local `qso_log`, so "not already worked"
can be answered server-side with the user's own token.

The auto-mode code this work feeds is `sdroxide_types::auto` (the pure policy:
`pick_cq`, `auto_ready`, `auto_block_reason`) plus
`crates/sdroxide-ui/src/app/auto_mode.rs` (the per-frame loop, run from the app
update rather than a panel so an unattended run survives switching pane or tab).
It is session-only and never persisted. "New" is
`LogIndex::novelty(..).new_call` for now; wiring in `check-dupe.php` is the
follow-up.

### ON RESEARCH, and NOT OURS TO BUILD: the ATS Mini's RadioScript firmware

**The constraint, and it is not a technical one: hjberndt's RadioScript firmware is
closed source with no licence of any kind.** No licence means the author retains
every right, so there is nothing here we may copy, bundle, ship, link or
reimplement — and the operator's ruling (2026-10-07) is that the fork treats it
that way. It is also, by his account, **the most popular firmware for these
boards** because it decodes CW, RTTY and WEFAX on the device itself, which is
precisely why he keeps it: *"that developer keeps full control of his firmware and
knows and uses the full limits of that tiny device."* That is a good outcome for
the hardware and not something for us to route around.

**So this section is a record, not a plan.** Everything below was read out of an
image Kevin attached to fork discussion #14; **the image is not in this tree**
(it is third-party firmware, unlicensed, and must never be committed), and
nothing here is to be built, adapted or reimplemented against. Keep the URL and
the findings so the next person asking is answered in a minute instead of an
afternoon — and so nobody mistakes this for groundwork for a feature.

**What is licence-clean, and the whole of it:** the **stock**
`esp32-si4732/ats-mini` firmware, which is **MIT**, which `Backend::AtsMini`
already speaks, and which is what the fork's ATS Mini support is built and
bench-measured against. If a control path to *that* firmware is wanted, it is
already there.

**The operator's ruling: research only, and expect a lot of questions about it**
because the board is cheap and popular. The reason is not scepticism about Kevin's
work, it is the thing people keep arguing with: **it is not an SDR dongle, however
badly people want it to be.** The Si4732 is a demodulator and both firmwares hand
over **demodulated audio** — there is no I/Q anywhere in it, and never will be. So
every wideband feature this fork has (ADS-B, AIS, APRS, HFDL, VDL2, DAB, the
panadapter's own view of the band) is **structurally unavailable**, not merely
unimplemented. Answer it that way, and do not let a request become a feature.

**We already support this class of device.** `Backend::AtsMini`
("ATS Mini (Wi-Fi control, sound card)"), TCP `60000`, `atsmini.local`, the
firmware's single-character command protocol and CSV telemetry — see the section
above for its bench-measured quirks (band steps of 370–450 ms, no direct band
select, one controller at a time). So fork discussion **#14** is **not a new device
class**: it is a *second firmware for the same board*, and the question that
decides whether any of it is our work at all is which one the board is running.

**Sources, and what may be read from them.** The developer's own page
(`http://www.hjberndt.de/soft/espbasic/espRadioScript.html`, read 2026-10-07 —
note it is **http**; the https upgrade fails) is the reference for all of this, and
Kevin's discussion **#14** post is the other half of it. Discussion #14 also
carries a firmware image
(`https://github.com/user-attachments/files/33118374/targetMiniRadioScript32.zip`,
a 1.4 MB ESP32-S3 application image). **It is third-party firmware and is not to
be committed, quoted from or worked against** — the reason this section now points
at the publisher's documentation instead of at strings lifted out of the binary is
precisely that the answer turned out to be *documented*, which is both cleaner to
use and clean to publish. What the image *was* good for is one identification
that the page confirms: ESP32-S3, esp-idf v4.4.5, `RadioScript esp32-S3
hjberndt.de, based on ESPBasic8266 3.0.Alpha 69`.

What it is, confirmed rather than assumed: ESP32-S3 (`e9 03` image header,
`xtensa-esp32s3-elf` build paths), **esp-idf v4.4.5**, and
`RadioScript esp32-S3 hjberndt.de, based on ESPBasic8266 3.0.Alpha 69`. Kevin's
identification is right.

**The licence picture, one layer deeper, and it decides everything.** There are
three pieces here and only one of them is licence-clean:

| Piece | Licence |
|---|---|
| `esp32-si4732/ats-mini` — the stock ATS Mini firmware, **MIT**, which `Backend::AtsMini` speaks | **MIT** — ours to use |
| The board itself — Sunnygold's design, open hardware on oshwhub | **CC BY-NC-SA 3.0** — open to read and non-commercially remix; not a software licence |
| The RadioScript interpreter — mmiscool's `esp8266Basic` (hjberndt's build extends it; his own pages link the source "auf Github") | **no licence file at all**, so all rights reserved by default |

So the **language's own interpreter source is unlicensed too**, which is the fact
that closes the door nobody had noticed: we may not fork the interpreter, patch
it, or derive a firmware from it, at either layer. The developer's caution about
his tools is worth keeping in mind as an attitude rather than a rule — *"Werkzeuge
können nützlich sein, aber in falschen Händen auch Schaden anrichten"*.

**What is nevertheless open to us, and it is the whole answer.** Three things,
none of which touches anybody's code:

1. **The radio's own hardware and the stock firmware are open** — and we already
   support the stock one.
2. **The language is documented.** His pages publish a command reference
   (`esp32Script`) and dozens of worked **examples in plain text**, including a
   browser front end that drives the whole radio in FM/AM/SSB with no cable, and
   a "command & response" low-level example that queries the chip. Reading a
   published specification is not reimplementation, and **a script is its author's
   own work** — the licence attaches to the interpreter's source, not to the
   language's syntax nor to programs written in it.
3. **The developer has already documented a control interface on the serial
   port**, in his own words: *"Über die serielle Schnittstelle steht der gesamte
   Befehlssatz dieser espRadioScript32-Sprache zur Verfügung, wenn kein Programm
   ausgeführt wird"* — the entire command set is available over the serial
   interface when no program is running.

**So the route I said did not exist, does — and it is the route he documented
rather than one we reverse-engineered.** A client that speaks that serial command
set is implementing a published interface, not reading his firmware. Nothing of
his is redistributed by us; an owner who wants his board in sdroxide flashes
whatever firmware they like themselves, which is their decision about their own
hardware and not ours.

**The practical shape, which is small and fork-only.** The board is
ESP32-S3 + Si4732 with a **USB-C serial port** (an Espressif VID/PID, flashing
and control over the same cable) and a 320×170 display. A short RadioScript on
the radio that answers a few lines over that serial port — frequency, mode,
volume, and the existing `rx.rssi` for a meter — is **plain text the owner can
read line by line**, and sdroxide's side is a second ATS-style source profile over
plain TCP or serial: no new `Backend`, no I/Q lane, and on that reading no
`PROTO_VERSION` bump. **Nothing has been built and nothing is scheduled**; this is
the shape, recorded so that if the operator ever says yes it is not re-derived.

**Three honest limits, from his own documentation rather than from us.** WiFi is
assumed throughout and *"eine schlechte WiFi-Verbindung zur Programmierzeit
verursacht unkontrollierte Hänger"* — hence the serial route, which does not.
Some ATS Mini boards ship an ESP32-S3-WROOM with **no WiFi antenna fitted, only a
connector**, and one must be attached before the browser can program the radio at
all. And he says plainly of the scripting firmware that debugging is slow, that
bugs are to be treated as features until reproduced, and that use is at one's own
risk.

**DiamondRadio — asked about 2026-10-14, and it is the same answer as RadioScript.**
Kevin (F4NRM, discussion #14) pointed at `khanfar/DiamondRadio` for his ATS 25
ULTRA: a whole custom ATS-25 firmware with FT8/FT4, CW, RTTY, SSTV, the EiBi
schedule, a map and propagation, all **on the radio**. Three things settled it:

- **Not open source.** Their own licence: *"free to download, install, and share
  as an unmodified binary. The source code is private and not published."*
  Freeware with closed source — RadioScript's category, only stated plainly. So
  nothing of it is ours to take, adapt or ship.
- **It is a competing firmware for the same boards, not a build target.** Our
  `Backend::AtsMini` speaks the **stock** (MIT) firmware's TCP interface; a radio
  flashed with DiamondRadio will not answer it. And the ceiling is unchanged
  either way — FT8/CW/RTTY/SSTV decode on the device because the Si4735 is a
  demodulator with no I/Q, so every wideband feature here stays out of reach on
  that board.
- **The one thing of ours it does point at:** every tool they ship is an
  in-browser client (installer, SDR monitor, phone view), so their **USB serial
  and WiFi protocols are publicly observable** rather than buried in a binary.
  That is a better position than RadioScript's binary, though not as clean as a
  published document — and it is still not a licence to copy their code. If a
  radio's dial and meter were ever wanted on the big screen inside sdroxide, that
  is the seam: our side only, nothing of theirs touched.

**Decided: not doing it.** Not for the licence alone — the case is thin. Same
board, same ceiling, and it duplicates what this program already does at the
other end of the cable. The first honest question, if anyone ever wants the two to
talk, is whether DiamondRadio still answers on the stock firmware's port; if it
does, the existing ATS Mini backend may already reach it for free.

**The boundary, stated once so it is not re-litigated.** There is exactly one way
this becomes ours, and it does not involve his firmware: **the developer himself
publishes a documented control interface** — his firmware can open a TCP server
(`tcpbegin`, port 8081) and answer lines of BASIC, so an owner-friendly
CAT-like surface is well within reach for him — and we then write *our* side
against **that documented interface**. A second ATS-style source profile over
plain TCP: no new `Backend`, no I/Q lane, and on that reading no
`PROTO_VERSION` bump, which Kevin also guessed correctly. Until such an interface
exists and is documented, there is nothing to build, and the answer to the next
person who asks is the two sentences above.

**One thing that is genuinely worth saying to a requester**, because it is what
this board *is* rather than what anyone hopes: the CW, RTTY and WEFAX decoding
happens **on the radio**, not here. It has no I/Q and no audio sdroxide can
digitise differently. What an sdroxide client could add is the operator's
surround — tuning, band and memory management, the SWL log, the schedule — not
decoding.

### DECIDED (2026-10-07): the ATS Mini RadioScript goes on the bench this week

**The operator has one of these boards himself** — *"That little radio got me
started in SWL again in the first place"* — and he runs hjberndt's firmware
because it decodes CW, RTTY and WEFAX on the device, which is the whole point of
that firmware for him. **So we do not need Kevin for the first testing at all.**
We have the hardware; he only has to confirm or deny what we find once there is
something to confirm, which is a far better use of his bench than being the first
tester again.

*"I would really love this too. we could really really test this on the bench
before any release."* That is the shape this fork prefers, and it is why this
item is worth the week: it is the one piece of work outstanding that can be
**proven before a release rather than after it**, it is fork-only, it is small,
it needs no browser, and it answers a question that will be asked repeatedly.

**What was told to Kevin**, in discussion #14 (the two earlier replies there
were deleted — see below — and this is the only answer that stands): we will look
at it and test it this week on our own board, and we will come back to him when
there is a beta to confirm or deny. Thanks for the pointer.

**The licence boundary is unchanged and is not negotiable**: nothing of
hjberndt's or mmiscool's is copied, bundled or reimplemented, and the firmware
image stays out of the tree. The client is built against the **developer's own
published documentation** — the command reference linked as *espScript32ReferenzH*
and the worked examples he publishes as plain text — which is the one route that
needs nobody's permission. A **script on the radio is the owner's own work**, and
that is what makes this testable: plain text the operator can read line by line,
with the firmware untouched.

**The design fork, unresolved, and it is the first thing to settle on the bench:**
the documented serial command set is available *"when no program is running"*, so
either we speak it directly and the radio is not running its on-device decoder
(the operator loses the feature he actually wants), **or a short script answers a
few lines on serial while doing the decoding itself** — one program at a time, so
the script is the one that has to do both. The second is what makes this worth
doing and it is a RadioScript problem before it is an sdroxide one.

### The reference, read (2026-10-07): what a client would actually send

His **command reference** is `espScript32ReferenzH.htm` (linked from the main
page as *espScript32ReferenzH*, German, ~118 000 characters, "powered by
mmiscool's ESP8266Basic", H. Berndt 2024/25). It has a **Radio Interface** section
for exactly this board, and it is the whole basis for a client. What it settles:

- **The hardware it covers: Si4732 on an ESP32-S3 LILYGO, 320x170 colour
  display, rotary encoder, backlight LED and encoder button.** FM stereo
  64–108 MHz with RDS, and **AM/SSB 150 kHz – 30 MHz** — so longwave, medium and
  short wave are in range, which is why this is an SWL board and not just an FM
  novelty.
- **Frequency units, which was one of Kevin's three open questions and is
  answered in the open**: *"mit Frequenzangabe entscheidet der Wert darüber ob AM
  oder FM eingestellt wird: `RX 88.8` oder `RX 5955`"*. **A decimal is MHz (FM), an
  integer is kHz (AM/SSB).** No capture needed; `RX`/`Freq` takes the value and the
  value decides the band.
- **Mode is three commands, not one**: `AM`, `USB`, `LSB` — with the documented
  caveat that the frequency should be set in the AM range first, and that `USB`
  and `LSB` load the SSB patch. **There is no power-off command**; the radio is
  switched with GPIO (`PinOut`) if you want it off.
- **`BFO` spans −16383 Hz to +16383 Hz** — a 32 kHz SSB window, which is a dial
  offset and nothing more. No bandwidth or filter control, no AGC, no squelch, no
  band concept at all: one command covers 150 kHz–30 MHz, which is *simpler* than
  the stock firmware's band-cycling we already speak.
- **Read-back is a script-side function, and this is the fork, answered**:
  `Rx.Freq()`, `Rx.RSSI()`, `Rx.SNR()`, `Rx.AM()/FM()/SSB()`, and the RDS set
  (`Rx.Station()` = 8-character station name, `Rx.Text()` = up to 64 characters of
  radiotext). His own published example `Simple3Radio.txt` reads them back exactly
  this way — `f = rx.freq()` after a `FreqUp`, `v = rx.vol()` after a `VolUp`.

**So the shape is now concrete, and the constraint is the interesting part.**
Writing to the radio needs **no script at all**: `RX`, `Freq`, `Vol`, `AM`/`USB`/
`LSB`, `BFO`, `FreqUp`/`FreqDn`, `SeekUP`/`SeekDN` are commands, and the serial
port carries the whole command set when no program is running. **Reading it back
does** — a meter needs a few lines that print the values. And **only one program
runs at a time**, so if the operator wants the board's own RTTY/CW decoding *and*
a meter, **one script has to do both**. That is the first thing to settle on the
bench, and it is a RadioScript problem before it is an sdroxide one.

**What our side would be.** A second ATS-style control source over plain serial
(or TCP 8081), not a new `Backend` and not an I/Q lane — so no `PROTO_VERSION`
bump. The audio half is already solved and is the one Kevin is already running on
his Zastone: the board's headphone jack into a sound card's line-in, sdroxide in
demod-audio mode. The command set sdroxide needs is short: frequency, mode,
volume, and one read for the meter — plus `Rx.Station()`/`Rx.Text()`, which are
RDS extras a listener would actually want.

**The honest unknown, and it is a bench question rather than a research one:**
whether the serial command set echoes anything useful when a program *is* running.
If it does not, the bridge script is the only route to a meter, which is fine —
it is plain text — but it decides whether the operator's decoder and our control
can coexist.

### Deleted answers on #14, and why that is also the record

Two replies were posted there and **both were deleted at the operator's
instruction**, and the instruction was right. The first was a blueprint — the
implementation shape read out of the firmware image — and posting *how to build
against* a closed, unlicensed product is not the same as answering a question.
It was the right analysis and the wrong place to leave it.

What replaced it is better in every way, and it is in the section above: the
answer turned out to be **documented** by the publisher, so the note cites his
pages instead of describing his internals. **That is the general shape to keep**
— when the answer to "how would we support X" is on X's own website, link to it;
do not publish a dissection of X's binary, and do not treat having read one as a
licence to write against it.

### The ATS Mini (SWL extras, fork-only)

A cheap, ubiquitous SWL receiver — ESP32-S3 + **Si4732**, firmware
[`esp32-si4732/ats-mini`](https://github.com/esp32-si4732/ats-mini) (MIT) —
driven from sdroxide as a receive-only source: control band and frequency from
the computer, do all the demod-dependent listening and decoding on the PC. It is
**audio-only** (the Si4732 demodulates in hardware, no I/Q), so `audio_mode`
applies and the wideband lanes are out. `Backend::AtsMini` is appended last,
receive-only (no TX UI). The scratch probe (telemetry parser, band-cycle mapper,
scanner) is `tools/atsmini-probe/`.

**Audio is analog, always.** There is no digital audio path in the stock
firmware — BLE is a Nordic UART service, the web server is config/OTA only, and
the Si4732 audio is not routed back to the ESP32 on V3 hardware — so the audio
runs 3.5 mm → a sound card on the PC. The route is **sdroxide-side only, no
firmware changes**. **The receive sound card must be chosen explicitly**: left on
the system default the source captures the mic, and the waterfall is flat while
the FT8 controller warns "no receive audio". That tab only offered a card at all
after `free_device_probe` gained a `Backend::AtsMini => DeviceProbe::RadioAudio`
arm.

**Control** is the firmware's "ad hoc" character protocol over **TCP
`atsmini.local:60000`** (or serial, or BLE), no auth, and **one controller at a
time**. `F<Hz>\r` sets frequency and is **band-locked** — rejected with an error
unless the frequency is in the *current* band — and there is **no direct band
select**, only `B`/`b` cycling, so the source tries `F` and, on the out-of-range
error, steps the band cycle until it is accepted, confirming from telemetry. `t`
turns on a 500 ms CSV telemetry monitor: `version, freq, bfo, bandCal, band,
mode, step, bw, agc, volume, rssi, snr, tuningCap, voltage, seq` (`freq` is
10 kHz units in FM and kHz in AM/SSB; `rssi` dBµV, `snr` dB, `voltage` already in
volts). Two band names to keep apart: the firmware's **`11M` is the
25.6–26.1 MHz broadcast band**, **`CB` is 27 MHz**; `ALL` is the 15–30 MHz
catch-all.

Tuning has no fast path: **each `B` writes NVS and takes ~370–450 ms a step**,
measured on the bench (an 11-step VHF→49M pick took ~3.5 s). So a band pick's
settle window *and* its tune deadline scale with the step count (`BAND_STEP_TIME`,
`band_settle`), the tune waits for the burst, and any dial arriving while a pick
is in flight is ignored — a fixed 900 ms once let a step leak out and a pick of
49M land on 60M. On connect the source **adopts the radio's first telemetry**
(dial and mode), because the engine opens it on the stored dial; the app then
starts where the radio is. The dial is shown **centred** in the waterfall — a
demod-audio front end has no RF panorama — a click on the waterfall **tunes the
dial** (there is no digital offset to set on a listening source), and the
receiver's own coarse step is called out in the settings tab, because a finer
change rounds away and the dial snaps back. The tune-in-flight note
("tuning — the radio is catching up") is drawn centred on the panadapter. Still
open: **battery voltage** (needs a `Meters` field) and the opaque bandwidth/AGC
indices.

**Two listener controls land here.** The **EQ** chip (after REC, on
`RadioState::rx_tone` — bass/mid/treble shelves) and the **LOG** chip opening the
**reception log** are both gated on `listener_screen()` — SWL mode, or a radio
that cannot transmit — because the ham RX strip has no room for another chip (a
desktop-strip two-row test pins it) and a listener's log is the reception one.
The same EQ is also on the LISTEN window's Tone row.

## ⛔ No release happens by itself — and none by an agent (2026-10-16, tightened 2026-10-18)

**The operator's standing rule, and it overrides the rhythm below: a release is
cut only when he asks, and after Kevin has tested.** Neither an agent nor he
tags one on his own initiative. He is not at home with his radio, the work has
got technical, and a build reaching a tester unasked is the thing he is avoiding.

**There are no exceptions left, and that includes the nightly (2026-10-18).** The
operator: *"somehow there was a release last night.. i want you to disable all
auto releases."* `nightly.yml`'s 18:00 UTC cron is **gone**; the workflow is
`workflow_dispatch` only, so a nightly is now something somebody asks for
(`gh workflow run nightly.yml`). **No workflow in `.github/workflows/` carries a
`schedule:` trigger any more** — that is the check to run if this is ever in
doubt: `grep -rn cron .github/workflows/` should print only prose.

**What happened on the night of 2026-10-17/18, because "somehow" had two
answers.** Neither was the Friday release:

- **The nightly cron fired late.** GitHub's scheduler started it at **20:21 UTC**,
  not 18:00 (it is best-effort), and it republished the `nightly` pre-release.
- **Two tags were pushed** — `v2.0.13.3_brown` (its run was cancelled) and
  `v2.0.13.4_brown` (published) at 00:36/00:42 UTC, carrying Kevin's WEFAX fix.
  **Kevin has `write` on the repository**, and the merge commit for his own PR
  #101 is authored by him, so this was almost certainly his doing rather than an
  agent's. Both are **pre-releases**, so `/releases/latest` still points at
  `v2.0.13.1_brown` and the README's stable links are untouched.

**The arrangement that replaces the rhythm, and it is the operator's (2026-10-18):
Kevin cuts the pre-releases**, and a full release is cut when everything is good
**and Kevin says so**. That is what he asked to be told, in as many words, on
fork #72.

**The `nightly` release and its tag were removed the same day**, at the operator's
instruction — *"remove the nightly that was not created by kevin"*. It was the one
release on the page the workflow had made: author `github-actions[bot]`, title
*SDR Oxide Brown nightly 2026-10-17*, 15 assets, created 2026-10-03 and reused
every night since (`gh release delete nightly --cleanup-tag`). **Every remaining
pre-release is Kevin's** — `v2.0.13.2` and `.4` are authored by F4NRM — which is
worth knowing before wondering whether one appeared by itself.
`nightly.yml` still exists and still moves the `nightly` tag when **run by hand**,
so a nightly can be had on request; nothing does it on its own.

**`v2.0.13.3_brown`'s release was removed too** (2026-10-18, *"remove the one
without assets"*): its run was cancelled, so it was a release page with **0
assets** — a tag and nothing to download. Its **tag was kept** deliberately
(`--cleanup-tag` *not* used), so the version number can never be re-cut onto
different content; the release can be recreated from the tag if it is ever wanted.
**The check turned up a second one of the same shape: `v2.0.9.4`** — Kevin's,
2026-10-12, pre-release, **0 assets** — and that one was **left in place**, because
the operator asked for a specific release rather than for a sweep. Offer it.

**A release with no assets is the shape to watch for.** It happens when a run is
cancelled or fails before its uploads and `create release` still publishes the
page (it is deliberately tolerant of failed build jobs, so the platforms that did
build are not lost). Listing every release and its asset count is three lines of
`gh` and is worth running after any cancelled release run.

**What still runs by itself, so it is not mistaken for a release:** Pages deploys
the `docs/` site on every push to `main` (`pages-build-deployment`, event
`dynamic`). That publishes a website, not a build, and it is not to be confused
with a release.

**An agent must not tag a release.** Not a Friday one, not a patch one, not a
pre-release, not "because the queue looks clean". Ask. (Kevin's own pre-releases
are his to cut; that is a different thing from an agent cutting one.)

## The release rhythm (decided 2026-10-07, and it is a decision, not a habit)

The operator's own words, kept because the reasoning is the whole point: *"we'll
be more mindful instead of riding the wind."* Three rungs, and what each is for:

> **⚠️ Superseded on 2026-10-18 — read the section above first.** The *calendar*
> is gone: nothing is cut on a schedule any more, because the operator asked for
> **all auto releases** to be disabled. **Kevin cuts the pre-releases**, and a
> full release is cut when everything is good **and Kevin says so**. What
> survives from the table below is the *shape* of a build — a nightly carries
> only what people downloaded, a release carries the full matrix — and the
> reasoning, which is why the table is kept rather than deleted.

| When | What | Why |
| --- | --- | --- |
| **Every evening** | a **nightly**, carrying **only the builds people actually downloaded from the last release** | room to test. Fourteen platform builds and thirty-odd assets for nobody is not a nightly, it is a cost |
| **Every Friday, cut at noon** | a **normal release**, so it is downloadable in the afternoon | testers get something real to play with while there is still a day in front of them |
| **In between, only if a serious bug appears** | a **patch release** — `2.0.3_brown` — cut then and there | a Friday's wait is a scheduled thing; a real fault is not |

**What this is actually for**, and it is not "more releases": the three of them
give testers **time** — time to test what is out, time to play radio, and us time
to do the same — which is what a program like this needs more of than features.
Expectations are managed by the calendar rather than by surprise: a tester knows
a Friday build is the one to try, and knows a nightly is tonight's changes only.
The week before, releases came out when something looked ready, which is the
wind.

**Two consequences to keep in mind, and they cut in opposite directions:**

- **A nightly may build nothing at all** if nobody has downloaded a platform. That
  is the point, and the consequence is that **a platform nobody downloads stops
  being covered**, so the mapping keeps a floor of `linux-x86_64` — the bench we
  test on ourselves — whatever its count says.
- **The narrowing is for nightlies only.** A Friday release and a patch release
  build the **full matrix**, so no platform can disappear from anything anybody
  installs. If the nightly ever looked like it was about to become the only
  thing being built, that would be the day to stop.

**Nothing here changes the standing rules further up**: never re-cut a tag, a
release notes entry is `Fixed` / `Added` / `Changed` / **`Needs feedback`**, and a
release day is still `cargo check --workspace --all-targets` **silent** plus
`cargo test --release --workspace` before the tag is cut.

## Regenerating the quick-start PDFs

`docs/cb-quickstart.{en,nl,fr,it}.md`,
`docs/ft8-11m-quickstart.{en,nl,fr,it}.md`,
`docs/sstv-11m-quickstart.{en,nl,fr,it}.md`,
`docs/listening-quickstart.{en,nl,fr,it}.md` and
`docs/pi-zero-2w-swl.{en,nl,fr,it}.md` are the sources; each matching
`.pdf` is generated and can drift. The TeX engines on this machine are unusable
(`xelatex.fmt` and `latex.fmt` are missing), so render through HTML and headless
Edge instead. Write the HTML **beside the stylesheet** — `-c` writes a relative
link, so an HTML in `/tmp` never finds `docs/cb-quickstart-pdf.css` — then
delete it. From the repo root, once per file (the example is the English CB one;
swap the stem for any other):

```sh
pandoc docs/cb-quickstart.en.md -s -c cb-quickstart-pdf.css -o docs/cb-quickstart.en.html
/opt/microsoft/msedge/msedge --headless=new --disable-gpu --no-sandbox \
  --user-data-dir=/tmp/edge-pdf --print-to-pdf=docs/cb-quickstart.en.pdf \
  --no-pdf-header-footer "file:///home/druid/sdroxide/docs/cb-quickstart.en.html"
rm docs/cb-quickstart.en.html
```

Commit the `.md` and the regenerated `.pdf` together, and say so if the `.md`
changed but the PDF was not remade. (`docs/qo100-quickstart.*.pdf` predate this
note and were rendered from a separate HTML source; leave them alone.)

## Cutting a release

> **Fresh tag history (2026-09-27, widened 2026-09-30).** After the Brown rename
> the tag list was reset, and on 2026-09-30 **everything before `v1.9.9_brown`
> was removed as well** — the older Brown releases (`v1.9.4` … `v1.9.8`,
> including `v1.9.6_brown.experimental`) and every pre-rename tag (`v0.1.0` …
> `v1.9.3`, the whole `_CB`/`CBSWL` lineage) are gone from **both** `origin` and
> the local clone. The remote now carries **`v1.9.9_brown`** and
> **`v1.9.10_brown`** plus **`nightly`**; the local clone was matched to it with
> `git fetch --prune --prune-tags`. **Do not reference a removed tag** — a
> download link to one is a 404, and the README's `releases/latest` links point
> at the newest release.

> **Version scheme (2026-09-30): step `Cargo.toml` for a real release.**
> **Amended 2026-10-06: the 1.9.x ceiling is spent — DAB landed, and `2.0.0_brown`
> is the milestone release it was being held for.** The rule was *"stay on 1.9.x
> until DAB is done, and never drift into 2.0"*, and DAB is now merged (experimental,
> not proven on air — see the DAB entry below), so the version steps to 2.0.0
> deliberately rather than by accident. Everything else about the scheme stands.
> `Cargo.toml` takes **only three numbers** — cargo rejects `1.9.8.1` — so
> the two roles are split: **a real release bumps the crate version** (1.9.8 →
> **1.9.9**; the Windows MSI and the macOS bundle take their version from
> `Cargo.toml`, so an upgrade has to step it), and **only a quick re-cut of the
> same feature set** uses a fourth point on the *tag* (`v1.9.8.1_brown`), which
> the workflow accepts and carries into the asset names while `Cargo.toml` stays
> put. So: a release with new features → bump to the next `1.9.x`; a rebuild of
> the same `main` → same crate version, a new tag point. Never reach 2.0 by
> accident; the point is the escape hatch, the crate bump is the norm.

0. **Run the gate before the tag, not after**: `cargo check --workspace
   --all-targets` (it must be **silent** — the house rule, and the only check
   that sees a `dead_code` stub standing in for a test) and
   `cargo test --release --workspace`. The suite **builds `examples/`**, so a
   scratch WIP probe parked in a crate's `examples/` fails the whole build and
   the gate never runs (see the 2026-10-04 session). `--workspace` is reserved
   for release days for the same reason; a merge wants the touched crates.
1. Bump the workspace version in `Cargo.toml` **first** and let `cargo` refresh
   `Cargo.lock`; commit it. The Windows `.msi` and the macOS bundle take their
   version from `Cargo.toml`, so a re-tag on the same version installs as the
   same version rather than an upgrade. **The Windows `UpgradeCode` is
   fixed** (see "The Windows installer is its own product"), so a new version
   is what an upgrade keys on — re-tagging the same version does not.

   **⚠️ "Let cargo refresh the lock" is not advice — the workflow builds with
   `--locked`, and a stale lock is a hard failure on every platform.** Bumping
   `Cargo.toml` and committing in one step, with `Cargo.lock` still naming the
   old version for its 59 members, is exactly the mistake made on 2026-10-11:
   `v2.0.9_brown` could not build anywhere, and the wasm job had already gone
   green before anyone looked. Locally it reads

       error: cannot update the lock file … because --locked was passed

   **Run `cargo check --locked -p <a touched crate>` against the tag's own
   lock before pushing the tag.** Seconds, and it is the only check that sees
   this. A plain `cargo check` quietly rewrites the lock and says nothing.

   **Recovery, when it happens anyway.** The tag is **not** re-cut. **Cancel
   the run first** — `create release` is deliberately tolerant of failed build
   jobs and will otherwise publish a partial release out of whichever
   platforms did build — then commit the corrected lock and cut a
   **fourth-point tag** (`v2.0.9.1_brown`) on the same feature set and the same
   crate version. That is the case the point tag exists for; `v2.0.7.1_brown`
   was the earlier one. A tag whose run was cancelled keeps no changelog
   heading — there is no release for it to sit above — so the heading moves to
   the point tag that will publish, since `release.yml` matches on the tag.
**The entry's shape: `### Fixed` / `### Added` / `### Changed` / `### Not
proven`.** **Keep it SHORT — one line per change.** The operator's standing
order (2026-10-09): *"use the short release-notes style … I hate those long reads
before you get to assets."* On GitHub the notes sit **above** the download
assets, so a wall of prose buries the thing a visitor came for: a heading and a
list of one-line bullets, then stop. The 1.9.20 and 1.9.21 entries were written
as thematic prose instead — that style came in with those two and is not this
changelog's, so it read as a revert. And the last heading is **`Needs feedback`** — renamed from `Not proven` on
2026-10-13 at the operator's word, and the reason is the whole point: *"the not
proven does bother me, what we actually want is feedback.. so why not ask for it"*.
`Not proven` is a **disclaimer**; `Needs feedback` is a **request**, and this fork
runs on field reports from people with hardware we do not have. Write each line as
the thing to try and the thing to send back — *"only ever seen on a path entirely
in night; try one that is partly daylit"* — not as a caveat. Never
`Not fixed`: everything in a build has passed its tests or it would not be in the
build, so *"not fixed"* asserts a breakage that is usually not there. A
operator's framing, and it is the right one: *"we are not shipping something
knowingly broken."*

1b. **Write the changelog entry** — in `CHANGELOG.md`, rename `## [Unreleased]`
   to `## [X.Y.Z_brown] - <date>` (the date the tag will carry) and add a fresh
   empty `## [Unreleased]` above it; commit. `release.yml` uses that section as
   the release's "What changed" (it matches `## [<tag without the v>]` and
   falls back to `--generate-notes` only when there is no entry). A re-cut of the
   same version is a fourth-point tag (`v1.9.10.1_brown`), which has no
   `CHANGELOG.md` entry of its own — leave it, the fallback covers it.
2. Tag `vX.Y.Z_brown` and push it — `release.yml` runs on the tag push
   (`on: push: tags: ['v*']`) and publishes the platform builds and the GitHub
   Release itself, titled `SDR Oxide Brown <tag>`, so no dispatch is needed. Do
   **not** also run `gh workflow run release.yml --ref vX.Y.Z_brown`: that
   dispatches a second, identical full release and the two race on the asset
   upload (cancel the dispatch if it happens). This note used to say a tag push
   did not run the workflow and to dispatch by hand; it does, and dispatching as
   well is the mistake.
3. The README's top download links already point at the stable-named Windows
   assets (`.../releases/latest/download/sdroxide-windows-x86_64.msi` and
   `.zip`), which every release now carries as copies of the versioned files;
   nothing to edit there.
4. Install locally: `cargo build --release`, `pkill -x sdroxide`, then
   `cp target/release/sdroxide ~/.cargo/bin/sdroxide`.

Uploads are per asset, with retries: each file goes up on its own, five attempts
with a growing pause, and anything already on the release is skipped, so a
`create release` job that dies partway is resumed with
`gh run rerun <run-id> --failed` and carries only what is missing. Do **not**
re-tag to recover. The all-or-nothing `gh release upload dist/*` it replaced is
what made v1.6.13_CB take four attempts: GitHub's uploads endpoint 500s
(`Error saving asset`, `Error creating asset temp dir`) on ~150 MB assets often
enough that one job should never depend on every file succeeding, and the
`--clobber` re-runs that followed deleted and re-created assets until they too
failed.

Nightlies are separate, and they are **manual only since 2026-10-18** — nothing
publishes by itself (see **No release happens by itself**, above; the 18:00 cron
is gone, and **Kevin cuts the pre-releases** now). `.github/workflows/nightly.yml`
is `workflow_dispatch` only: run it by hand (`gh workflow run nightly.yml`) and it
moves the `nightly` tag to `main` and dispatches the same release workflow against
it. Asking for one always builds one. `release.yml` publishes a `nightly` ref as a
**pre-release** with a dated title, so `/releases/latest` and the README's stable
download links keep pointing at a tagged release rather than at a nightly build.

**What a nightly builds, and why it is not all fourteen** (2026-10-07): the
**only platforms anybody has downloaded from the last three releases**, read from
their asset download counts, plus a **floor of `linux-x86_64` and
`linux-aarch64-compat`**. Three details that are the difference between this
working and not:

- **Three releases, not one.** A release is published and then downloaded over the
  days after it; counting only the newest would have made the nightly in the
  evening a Friday release was cut build almost nothing. Measured against our own
  releases the difference is stark — the latest release alone said "2 downloads",
  the three-release window said 28 Windows, 2 Apple-silicon and 1 Intel Mac.
- **The floor.** `linux-x86_64` is the bench this fork is developed on, and a
  platform that stops being built stops being tested. `linux-aarch64-compat` is
  named for a different reason: it is the one build **no release publishes**, and
  the nightly is the only way anybody gets that artifact, so it can never be left
  to a download count that is permanently zero.
- **`cat` only.** A new `variants` input on `release.yml` keeps the nightly off
  the Soapy builds, which it has never needed: `cat` is the one a tester can run
  without installing PothosSDR first.

Both inputs are validated in `targets` as non-empty JSON, because a typo in a
dispatch input would otherwise produce an empty matrix and a green run that
built nothing. **A release — Friday or patch — never passes either one, so
nothing anybody installs is narrowed by any of this.

## The Morse trainer (offered upstream as #568)

A learning tool, receive-only by construction. `sdroxide_types::MorseProgress`
the Koch curriculum (order, run-to-unlock, score) and `crates/sdroxide-ui/src/
app/morse.rs` the **morse_window** (TRANSLATE / PRACTICE / LEARN), opened by the
CW panel's **TRAINER** chip. It plays through `sdroxide_audio::start_output` —
the alerts' cpal path — with `sdroxide_dsp::CwTx` as the keyer, and never touches
the engine, so no `PROTO_VERSION` bump and no wire type. Progress persists
through the logbook's native/wasm `persist` split. Native-only playback: the
`sdroxide-dsp` dependency is behind `cfg(not(target_arch = "wasm32"))`, so the
browser build gets the drill without a speaker (and no TRANSLATE pane, the Morse
table being native-only). Offered upstream as draft **#568** (branch
`upstream-pr/morse-trainer`, from `upstream/main`); the fork carries it on
`main` until it lands, then its copy drops out.

The key is configured in **Settings → CW** (`crates/sdroxide-ui/src/app/
settings/mod.rs`, `settings_cw_usb`): source keyboard or USB, the device, key
type straight / iambic A / iambic B, the reverse switch, the keyer speed, and
`cw_key_tx`. The five fields are appended to `DigiConfig` (v179). The CW panel
(`panels/cw.rs`) runs the paddle only while **KEY** is armed and `cw_key_tx` is
on: it reads `CwKeySource::key_down()` each frame and turns it into
`Command::CwKey(down/up)` edges, arming `CwStraight(true)` through the existing
toggle — so transmit is the ordinary manual-key path and the band lockout, the
30 s watchdog and the `CwSelfRx` read-back all apply. The likely-recurring
mistake is expecting a rig that keys itself to hand-key: `cw_keying = Cat`
leaves `rig_keys_itself` true and `set_straight` refuses by design, so a
paddle/SWL test needs **CW keying = Sound card (MCW)** (the VOX/audio route the
CRT SS9900v is set up for). Known limit: the keyer runs in the UI's evdev thread
but the key-down is sampled once per frame and the engine applies commands once
per ~10 ms loop, so element edges are quantised; running the keyer in
`cw_controller` (a `Command` carrying contacts, a `PROTO_VERSION` bump) is the
refinement if a report says the sending is ragged at speed.

A fourth pane, **SEND**, drills sending with a real USB paddle, and is
**fork-only and Linux-only** — it is raw evdev, and the browser and the other
systems have no equivalent. The portable half is `sdroxide_dsp::CwKeyer`
(software iambic A/B over two contacts, `take_text`), which is general and a
candidate to offer on its own. `crates/sdroxide-ui/src/app/cw_key.rs` is the
device half: it opens a keyer interface's input nodes, takes them exclusively
(`EVIOCGRAB`, so the contacts cannot also click in other windows), runs the
keyer, and plays a local sidetone — it never keys the radio. Two things that
cost a debugging round: a composite HID keyer registers **two** nodes for one
interface (a keyboard and a mouse node) and the contacts are often on the mouse
one, so the list is one entry per interface and both nodes are opened and
grabbed; and the paddle contacts arrive as `BTN_LEFT`/`BTN_RIGHT`, so an
un-grabbed device just clicks the GUI. The bench keyer here is a CH55x
`1209:c550` ("-Yuan-3key", a CH55xduino default VID:PID) that reports the raw
contacts and does no iambic of its own, which is why the keyer is in software.
Not tested on other paddle hardware.

The portable half is offered upstream as draft **#569** (branch
`upstream-pr/cw-keyer`, from `upstream/main`), asking whether the keyer's
key-down output should also drive the transmit path. That seam is **confirmed
working on air**: a USB paddle keyed a CRT SS9900v (11 m CB) over MCW/VOX
through `Command::CwStraight`/`CwKey`, iambic and straight, so the body now
answers the question with that evidence. **The maintainer answered, and the
answer was "engine-side, iambic-only"** — so the draft has been rebased
(`aa44dc06`, one commit, `CwKeyer` straight mode dropped) and the question it
asked is now **#626** (see §10). The other upstream piece is the
no-control-link fallback as draft **#572** (branch
`upstream-pr/cw-keying-no-link`): a stored `cw_keying = Cat` with no
serial path or network address made the source report `cw_text_keying() =
Some`, so `rig_keys_itself` went true, the panel's KEY was disabled and a hand
key sent nothing — the bug that made the paddle look broken on a VOX rig.
`effective_cw_keying` falls back to `Audio` for the chunk size and `cw_mcw`,
while the commanded mode keeps the stored setting.

**#573 is superseded — do not rebase it.** The user-facing package (Settings →
CW, the evdev source, the panel transmit wiring) stays on the **fork**: the
maintainer's answer asked for the engine half alone, and offering the panel half
as a third PR would mean reviewing the same split in two places. Its branch
(`upstream-pr/cw-key`) is left stale and carries a comment saying so, with the
offer stated plainly: **he is free to open a PR against the fork** if he wants
the user-facing half, which is the cheap way to get it rather than re-reviewing
it upstream. The original rationale is kept here because it is still the reason
the split exists — it carried #569's keyer commits **as well as** the new work,
because a pull request's base can only be a branch in the upstream repo, so a
fork cannot stack a PR on another fork PR's branch. The evdev source is
**Linux-only raw evdev**, and
that is a design question raised in the body, not assumed: `hidapi` is
cross-platform but cannot take the device exclusively, so the contacts would
also arrive as clicks. The branch's `cw_key.rs` is trimmed to `key_down` +
`error` (the trainer's `take_text`/`contacts`/`marks` belong to #568's pane and
are not in this PR).

## A station calling us is never discarded in silence (fork-only, 2026-10-01)

An FT8 reply is adopted only through one arm, gated on `step == CallingCq &&
dx.is_none()` (`qso.rs`). Every other state — `Idle` after the CQ run gave up,
`WaitCq` holding for somebody else, a `dx` left from an earlier contact — falls
through to "only the station we're working", does not match, and was discarded by
a bare `continue`. That was a decode **carrying our own callsign in the
addressee field**, at a usable signal, and the program said nothing at all.

Seen on 11 m on 2026-10-01: `19AT168` answered a CQ four times over ninety
seconds, +2 dB to +12 dB, the message reading `<19DC373> 19AT168` — the
operator's callsign **resolved in the clear**, which is exactly what
`is_call_to_us` asks for, so every gate passed — and no contact was made. The
CSV export cannot show which state the machine was in, and that is the whole
difficulty: the failing state was not recorded anywhere.

Two changes, both pure observation:

- **A transcript note**, once per station, naming it and the state: *"{call} is
  calling you and we are not answering ({step}, {dx}) — press REPLY to take
  it."* Rate-limited by `QsoMachine::unanswered` because an unanswered station
  repeats and four copies of one sentence would bury the transcript it exists to
  explain. It is a note, **not a branch**: nothing is sent and nothing consumed,
  so the fall-through still handles a station we *are* working, a reply that
  resumes their exchange, and the Hound's own close. Bare `73`/`RR73` are
  excluded, as `is_call_to_us` excludes them — somebody finishing is not
  somebody calling.
- **The engine's `QsoStep` on the AUTO chip's hover**, because it is the one fact
  that decides whether an answer can be adopted. It was invisible, which is what
  made this undiagnosable from the artefacts.

**The cause, which the operator worked out and the code confirms.** The **transmit
watchdog** fires *during* a CQ run — `wants_tx()` is true in `CallingCq`, only
`Idle` and `WaitCq` opt out — and `tick` then forces `step = Idle`
(`qso.rs:646`). The adopt arm needs `CallingCq`, so from that instant no answer
can be taken however well formed it is, and `progress` deliberately does not
clear the watchdog (only `operator_acted` does), so the answer cannot revive it
either. **The operator's own hypothesis was that 19AT168 replied on the last call
before the watchdog tripped; that is exactly it.**

This is **by design, and must not be "fixed"**: an unattended station has to stop
transmitting, and one that resumed the moment somebody called would be unattended
*and* transmitting. WSJT-CB cuts TX the same way. The defect was only that this
was indistinguishable from "nobody answered", so the notice now names the
watchdog outright rather than reporting a caller we are somehow ignoring. Pinned
by `a_station_answering_after_the_watchdog_is_named_as_such`, which also asserts
that **nothing is sent** on the answerer's account.

A station that sits on its own tone and answers on his own clock makes this
likely rather than rare: his replies arrive regardless of where our CQ went, so
the timing between his reply and the watchdog is coincidence — but it is the
kind that recurs.

**`WaitCq` is load-bearing and must not be "fixed".** The obvious suggestion —
let an addressed answer be adopted in `WaitCq` too — is **wrong**, and the
operator's own objection is the reason to record it: `WaitCq` *is* the queue.
Pressing REPLY on a station that is busy holds the machine for **that** station
until they call CQ. Admitting answers from anyone else in that state would not
improve adoption, it would **annihilate the queue**, abandoning the station the
operator queued for to chase whoever spoke last. On an unattended run that means
keying at people who are not answering it — on a shared band, that is harm to
third parties, not a bug in this program.

**The fix: the run is bounded by the count, not the clock (11 m only).** The
watchdog runs off `progress_utc`, stamped only by `progress()` (a reply arrived)
or `operator_acted()`. During a CQ run **no reply is the expected state**, so the
clock measures nothing but time since the operator pressed CQ and fires on
schedule regardless of how young the run is. It also fires in units that mean
nothing: a station answering a CQ is expected inside ~30 s, and a 6-minute
watchdog cut the run at 12 calls while `max_tx_repeats` (default 10, ~5 min at
FT8's one call per two slots) was within a call or two of the same bound. The
call count is the right unit — it counts real attempts, and it is what
`max_tx_repeats`' own comment already claimed: *"Repeating a CQ is exempt — that
is the operation; the watchdog above bounds it instead."*

So `tick` skips the watchdog when `step == CallingCq && self.cb`
(`matches!(self.step, QsoStep::CallingCq if self.cb)`). **Everything else is
untouched**: every other band, and `WaitCq`, which the operator was explicit
about keeping — it is the station queue, bounded by its own deadline, and one of
the program's best features.

**A stalled exchange on 11 m is still cut, and that is the point.** The watchdog
leaves `dx` standing when it trips, so a station calling us again is the one we
were working and their message advances the exchange. Propagation dropping
mid-QSO and leaving us reporting into a dead channel is the failure it exists
for, and on 11 m it is worse than it sounds: our transmissions there are **free
text with no addressing**, so nothing but another station's 73 on that frequency
can end it. The operator has ended exactly that by hand more than once.

**What was rejected, and why it is worth recording.** The operator's own first
idea was to let the transmit through when our callsign appears within ~45 s of
the watchdog — a grace window. Right instinct, wrong mechanism, for two reasons
worth keeping: (1) it needs `watchdog = false`, and that flag exists so **only
an operator** restarts the sequencer, so any decode-triggered clearing means **a
station on the band can key the transmitter by naming us** — on a band with no
authentication and stations auto-answering each other, a remote transmit trigger;
(2) `d.to` is a 12-bit hash, so a message addressed to somebody else can resolve
to us, and wiring that to a transmit decision is a different risk class from
wiring it to a display. Exempting the run changes when a timer fires, not who may
key the radio.

**The bisect result worth keeping:** `qso.rs`, `auto_mode.rs`, `decodes.rs`,
`digi.rs` and `contest.rs` are all **byte-identical between 1.9.8 and
1.9.12**, so bisecting those two tags cannot find this and both would fail
alike. It was nearly missed because the first pass diffed `sdroxide-digi` and
`sdroxide-types` and skipped the UI, where the auto loop actually lives. When
one side "works perfectly" and the other does not, diff the **whole** path first
and believe the result before theorising.

## The KNOWN window — who your hashes can name (fork-only, 2026-10-01)

An FT8 message carries a **hash** of the callsign it addresses, not the callsign,
and a hash is one-way: there is no arithmetic that inverts it. So `<...>` is all
that will ever be recoverable from that message, and every FT8 program on the
band behaves the same. The only route to a callsign is to have **heard it spelled
out**, and the set of those was invisible — you could watch a message resolve and
had no way to see why, or to browse who you currently know. That is the feature:
a **KNOWN** chip in the general decode's filter row, and a window listing the set
**newest first, with each station's country**.

- **The list is a shadow, not a read.** mfsk-core's `CallsignHashTable` is
  lookup-only — `len22`, `capacity22`, and nothing that enumerates — so it cannot
  be asked what it knows. `remember_heard` and `seed_hashes` are the *only* two
  feeders of that table and both see each callsign in plain text on the way in,
  so `Ft8Modem` keeps a `known_calls: VecDeque<String>` fed from exactly those
  two places. Same cap (`MAX_KNOWN_CALLS`, matching `MAXHASH`) and the same
  newest-wins-a-collision order, so the two evict together.
  **`the_known_call_list_holds_exactly_what_the_table_can_resolve` asserts that
  per callsign** — a stale entry would promise a resolution the decoder can no
  longer deliver, and that is the one way this list could lie. Do not let the two
  drift into a second implementation of "who do we know".
- **The worker owns the table**, deliberately (LDPC stays off the RT thread), so
  `DecodeJob` became an **enum** carrying a question as well as a slot, and the
  answer comes back on its **own channel** so a caller cannot read a slot's
  decodes instead of a list. `DigiEngine::known_calls` **defaults to `None`** —
  a mode with no table is not a station that has heard nobody.
- **`RadioEvent::KnownCalls` carries an `Option`, and the engine sends it either
  way.** This was a real bug in the first cut: sending nothing on failure made
  the window claim an empty band for a mode that simply has no table, which is
  exactly the "control that silently does nothing" the house rules forbid. The
  window now says which half is missing.
- **The read blocks up to 500 ms**, so it is a message the operator sends by
  opening a window — never a per-frame call.
- `Command::GetKnownCalls` + `ServerMsg::KnownCalls`, both **appended last**;
  `PROTO_VERSION` 188 → **189**. Capped at 200 with a `total`, so the header can
  say "newest 200 of 340" instead of implying a truncated list is complete.
  **Fork-only**, not offered upstream — there is no upstream audience for an
  operator wanting to see his hash table.
- On 11 m the country is the point: `resolve_callsign` resolves WSJT-CB's own
  numbering **ahead of** the amateur table, so `19DC797` reads Netherlands and
  `4CB04` Argentina. Same flag machinery the decode rows already draw.
- **Not tested on air** — each end of the round trip is unit-tested, but nobody
  has watched a `<...>` resolve into the list on a live band.

## SSTV now lands on its own band's frequency (fork-only, 2026-10-01)

The operator's report was the obvious one: choose a band, choose SSTV, and be
somewhere useless, with the only way out being the **⇵ FREQ** chip in the panel
and a manual pick.

- **The machinery already existed.** `conventional_dial_for` is the rule that
  moves the dial onto a mode's published frequency, and it has done so for the
  slotted modes and WSPR for years. Its gate was
  `mode.is_slotted() || mode.is_wspr()` — and **SSTV is neither**, so it returned
  `None`. The fix is the predicate: `|| mode.is_sstv()`, covering **both** SSTV
  modes because both are one-frequency-per-band and both tables exist
  (`SSTV_DIALS` region-tagged, `SSTV_FM_DIALS` for 6 m / 2 m / 70 cm).
- **What makes it safe to do unasked** is the check already inside the rule: a
  dial **already on one of the mode's own frequencies is never moved**. So the
  only case this rescues is a dial that is nowhere useful — a deliberately tuned
  FT8 Fox window survives, and so does anything you set yourself.
- **The 11 m table had to change with it.** SSTV on 11 m has three entries —
  27.255 (ch 23), 27.375 (ch 37) and 27.700 (freeband) — and all three carried
  a note, so the "the plain calling frequency" rule (`note.is_empty()`) found
  none and fell back to the **lowest**, landing on ch 23. **27.700 is now the
  unannotated one**, which is both what the community works and what the band
  buttons should reach.
  **This inverts the usual order** — the plain frequency is normally the *first*
  entry in a band, and here it is the highest — so every rule that wants "the one
  true frequency" looks for the empty note **anywhere** in the band rather than
  taking the lowest. `DigiChannel::note`'s doc now says so, and points at
  `CB11_DIALS`. Do not "fix" a future lowest-wins assumption without reading it.
- Tests: `crates/sdroxide-radio/tests/conventional_dial.rs` pins all five
  edges — it lands on 27.700 on 11 m and 14.230 on 20 m; a dial already on an
  SSTV frequency is untouched; re-selecting the mode in force does not move; the
  **LISTEN** path (`SetModeListen`) does the same; and a band with no SSTV
  convention (**6 m**) is left alone rather than dragged across the world.
  - A trap worth remembering from writing it: the first version used 6 cm
    (5 GHz) for the last case and it failed for an unrelated reason — the mock
    radio only tunes 0–1 GHz, so the `SetVfo` was **refused** and the test was
    measuring the initial dial, not the rule. 6 m is inside the range.

## Replaying a recording (fork-only, picked up 2026-10-06 — **scoped, not built**)

Fork discussion **#10**, asked as *"replay a recorded signal — for example an
Olivia signal, to find its mode and decipher it."* Answered there; recorded here
because the shape of it is not what the request suggests, and the next session
should not start by building the wrong thing.

**The capability is already there, per mode, and it is only a test hook.** Every
decoder that can read a real signal can already read a **WAV file from disk**:
Olivia, ACARS, ALE, DRM, FSK441, HD Radio, HFDL and NRSC5 each take a path from
`SDROXIDE_<MODE>_SAMPLE` and assert on the *content* of a genuine recording. That
is how the real-world verification is done — not a fiction. But every one of them
is an `#[ignore]`d test reading an environment variable, so **an operator cannot
reach any of it.**

**What is missing is not a decoder, it is an audio file source — and the one file
source we have is the wrong kind.** `FileSource`
(`crates/sdroxide-radio/src/source.rs:1849`) plays a raw CF32 or I/Q WAV, looped
and real-time paced. That is an **I/Q** input, which is what ADS-B, AIS, HFDL and
VDL2 want. Olivia and every other text mode is fed from the **demodulated audio**
tap (`on_rx_audio`), and **there is no audio-file source in the tree at all.**
So the shape of the work is:

1. **`AudioFileSource`**, mirroring `FileSource` — a mono/stereo WAV (and MP3, or
   whatever `rfd` hands back) paced into the audio tap, with the sample rate read
   from the header the way `FileSource` already reads its own. This is the piece
   that unlocks *every* text mode from a recording at once, and it is the one
   worth doing.
2. **Reachability.** `FileSource` is reachable **only from `--file` on the command
   line** — there is no config or `Backend` path to it, so "replay a recording"
   is currently a launch flag rather than a control. If this becomes a `Backend`
   it is a wire change and takes a `PROTO_VERSION` bump; if it is a per-radio
   source field, it is `#[serde(default)]` and no bump. **Decide this before
   writing code.**
3. **The mode question, which is the honest half of his request.** Decoding a
   file for a mode you named is nearly free once (1) exists. *"Find its mode"* is
   signal classification — a different and much larger piece, with a different
   failure mode, because it must be right about a recording that is weak, clipped
   or off-frequency. **Do not ship a guesser.** The useful middle path: our own
   recordings already carry UTC, frequency and mode in the filename, so a file we
  made can be opened knowing what it is, and only a foreign `.wav` needs the
   guesser. That covers most real cases with none of the risk.

**Unverified, and worth ten minutes to check before designing anything:** an
operator's *I/Q* capture replayed through `--file` may already reach a text mode,
because the engine demodulates a wideband source into audio. If that works, the
recording side of this needs nothing at all and only the *UI* is missing. There is
no I/Q capture on this bench to try it with, so it stays a question rather than a
claim — and answering it first could shrink this from a feature to a button.

**Fit for this fork: total.** Upstream has no recording replay and no audience
for it; this is the listener asking for the tool their own receiver should have.

## The ADS-B map's stations are tunable (2026-10-18)

The operator: *"in the ADS-B you have those stations on the map with a frequency,
why can't I double-click on them and they become tuned or tuned under B in the
frequency window so I can switch between the plane and the ground station"*. Right
on both counts — the frequency was only ever *text* (the hover named a VOR or an
NDB and the operator typed it into a box), and the gesture he wanted was already
taken.

- **The hover carries the frequency**, and it is the **navaids** that have one:
  `airdata::Airport` publishes no frequency, so a double-click on an airport tunes
  nothing and says nothing about tuning. `NavHover::freq_khz` is `Option<u32>`,
  the table's kHz, and `tune_hz` is the one conversion to the dial's Hz (this fork
  has had kHz and Hz the wrong way round before).
- **The double-click had a prior owner**: `worldmap::interact` hands the map's
  view back to the auto-fit on a double-click. Answering both would undo the zoom
  the operator had just used to *find* the station — so it is
  `interact_with(..., reframe)` now, and the ADS-B map passes `false` when the
  pointer is over a station with a frequency. The check runs *before* the
  interaction, which is safe because a double-click never pans, and it reuses
  `draw_nav_layer` **with its painter as an `Option`** — the picking rule stays in
  one place instead of getting a second copy for the question. The bottom-right
  "double-click to reframe" hint is withheld over a station for the same reason:
  a hint that promises the wrong gesture is worse than none.
- **The gesture is said where the name is read** (`draw_hover_label` adds the
  line, only for the stations that can be tuned): a mark that answers a
  double-click has to look like one.
- **The click always takes the *other* dial, switches to it, and puts AM on it.**
  This started as "the active dial, Shift for the other one" and the operator
  corrected it, having traced it himself: *"the mode is ADS-B so it wants to
  switch back to the ADS-B frequency… in that mode you cannot set frequency A
  because the mode dictates"* — and *"if you want to click a frequency it always
  has to go to B"*. The engine agrees in its own log ("ADS-B is on 1090 MHz;
  tuning there", from three places), so a frequency written to a dial its mode
  owns is overwritten the moment that VFO is recalled. AM because an aid is
  amplitude modulated and a lane mode cannot carry it: a station parked on a dial
  still in ADS-B would be silence and then a retune. **The rule is general — for
  any mode with a fixed frequency, a tune goes to the dial that mode does not
  hold** — and the ADS-B map is the only tuner of that kind today (the ADS-B
  panel's own frequency chips keep aiming at the active dial: they are the
  channel's control, not a station picked off a map). Tests:
  `the_hover_carries_the_frequency_a_station_can_be_tuned_to` (verified failing
  with the frequency set to `None` — "the hover did not carry the frequency its
  own text names") and
  `a_tuned_station_goes_to_the_dial_the_mode_does_not_own`.

## The signal-identification guide (fork-only, 2026-09-25)

A listener tool, opened by the **SIG ID** chip in the LISTEN window — and, in
SWL mode, by the System box's bottom row, where it **replaces the MAIL chip**:
radio email is a transmitting ham's tool with nothing for a listener, while
"what is on this dial?" is exactly the listener's question. The swap is in
`system_chips_bottom`/`system_bottom_row`, and the box re-prices its own width
for the wider label. The pure half is `sdroxide_types::signal_id`: a
`SignalProfile` catalogue
(`name`, `family`, `modulation`, `bandwidth_hz`, `bands`, `frequencies_hz`,
`modes`, `summary`, `sigidwiki` slug) of ~60 signals, and the ranker
`identify(freq_hz, band, mode, bw_hz)`. The rank is **exact mode match first,
then frequencies and bands, then passband closeness** — implemented as a
`(mode_match, score)` sort, not one blended number, because a frequency hit
alone must not outrank the mode the operator is actually in. `search_profiles`
is the free-text search the window's **Find** box uses. The UI half is
`crates/sdroxide-ui/src/app/signal_id.rs` (ranked list, search, **sigidwiki**
link via `ctx.open_url`); no wire type and no `PROTO_VERSION` bump, so it is a
plain fork feature.

**Licence — the reason the catalogue is ours and the samples are linked, not
bundled.** The obvious source is `AresValley/Artemis` (a Python signal-ID app,
GPL-3 code) and its `Artemis-DB` crawler, but Artemis-DB has **no `LICENSE`
file**, its README says "internal use only", and its content is crawled from
**Sigidwiki**, whose own licence is unclear. Artemis's GPL covers its code, not
that wiki content. So nothing from Artemis-DB or Sigidwiki is bundled here: the
`SignalProfile` data is written from public band plans, modes and frequencies,
and the **sigidwiki** link opens the sample in the browser. The slugs in
`SignalProfile::sigidwiki` are the wiki's own page names (checked against
`https://www.sigidwiki.com/wiki/Database`); a profile with no page is `None`.
If the wiki's content is ever wanted offline, ask its admin (Carl Colena, on the
Artemis team) — do not ship it first.

**Not done, and the natural next step:** the plan's **ACF** (envelope/spectrum
autocorrelation) as a measured DSP feature to feed identification. It is an
"isolate it" DSP change and has no home until something computes it from the
receive chain, so keep it separate from the catalogue.

## Client screen settings on the server (fork-only, 2026-09-30)

A remote client of `--server` can keep its **screen** on the server, against the
profile it signed in as, instead of only in the browser — the fix for a stale
screen after every new session (fork discussion #4, kevin2008-01).

- **`UiSettings` CANNOT go on the wire.** `spot_colors` / `bandplan_colors`
  deserialize through `deserialize_with` functions that read a `Vec` where the
  derived serializer writes a fixed array — TOML absorbs the mismatch, postcard
  does not, so `UiSettings` fails a postcard round-trip (`the_screen_survives_a_
  postcard_round_trip`). The wire type is therefore **`sdroxide_types::
  ClientScreen`**, a dedicated, postcard-safe, **presentation-only** struct.
  **Do not** put `UiSettings` itself on the wire. This is the trap to remember.
- **Presentation-only is a scope cut, not a security guard.** `ClientScreen`
  carries only the look — theme, layout, waterfall/spectrum, fonts, Simple UI,
  map layers. Window geometry, display zoom, the decode-list views
  and the one-shot acknowledgements stay on the machine, so adopting a login's
  screen never moves a window off a laptop or swallows a warning. (`UiSettings`
  was never an injection risk: it carries no URLs, paths or feeds, only scalars
  — an earlier note claiming otherwise was wrong.)
- **Control bindings travel only behind an opt-in, and it is *gated*.** A shared
  station is a shared keyboard. The decision (operator, 2026-10-01) was that a
  profile *may* carry the bindings, but only behind a mandatory acknowledgement —
  the operator is told the shared-keyboard risk and that other surprises may
  follow — and the control is marked **not recommended**. Default stays off, so a
  shared station keeps them local; the fork asks, it does not assume.
  **Built on fork `main` (2026-10-01).** `UiSettings::client_share_bindings`
  (default `false`) is the flag; the Settings → UI row is a checkbox that is
  **not** a plain toggle: clicking it on only sets an `egui::Id` temp flag and
  opens `egui::Modal`, and the flag itself is set from *after* the modal closes,
  on the confirming button. Turning it **off** is immediate. The row is also
  `add_enabled_ui(false)` unless the scope is `Server`, so the operator cannot
  arm something that does nothing.
  **Two things deliberately not done.** The flag is *not* in `ClientScreen`, so
  opting in on one machine does not silently opt in another — it is a decision
  made on a machine, not a property a profile carries. And a client that has not
  opted in **ignores** stored bindings outright (`frame.rs`), rather than
  adopting what a profile happens to hold.
- **Wire, bindings:** a **separate** `ClientMsg::SetClientBindings` +
  `ServerMsg::ClientBindings` pair, not a field on the screen messages, so the
  default-off path cannot put a binding on the wire even by accident; both
  **appended last**; `PROTO_VERSION` 187 → **188**. `InputSettings` rides it
  whole and *is* postcard-safe (all `Vec`/`String`/enum/scalar, no custom
  `deserialize_with` — unlike `UiSettings`), pinned by
  `the_bindings_survive_a_postcard_round_trip` in `input.rs`. That test is the
  trap to remember for the next wire field: a `deserialize_with` is what makes a
  type postcard-unsafe, and only `UiSettings` has one.
  **One consequence to keep in mind:** `InputSettings` rides whole, so an
  **`Action` discriminant is on the wire for the first time**. Both ends must
  already agree on `PROTO_VERSION`, so the bump covers it — but an `Action`
  variant inserted mid-enum now shifts what the far end reads, so **append
  `Action` variants only** and treat one as a wire change.
- **Wire, screen:** `ClientMsg::SetClientSettings` + `ServerMsg::ClientSettings`,
  both **appended last**; `PROTO_VERSION` 185 → 186.
- **Store:** `sdroxide_config::ClientSettingsStore`, `clientsettings.json`,
  **per profile + a station default**, written through the ordinary config
  store. `sdroxide-config` had to move from the server's `[dev-dependencies]`
  to `[dependencies]`. The bindings ride the same file as two more `#[serde(
  default)]` fields — `bindings_default: Option<InputSettings>` and `bindings:
  BTreeMap<String, InputSettings>` — so a file written before this change still
  loads, and one written with no bindings still loads in an older fork.
- **Server:** pushes the stored set on connect and stores on request, both
  behind the existing sign-in. `handshake` now returns the signed-in username
  (captured from the `Auth` frame), which is the profile key. **On a passwordless
  server "signed in" means "anyone on the LAN"**, so a profile is a name and not
  a secret — the manual and the reply said so.
- **Client:** applies only when `UiSettings::client_save_scope` is `Server`
  (`RadioEvent::ClientSettings` in `frame.rs`); pushes on save via
  `RadioController::send_client_settings` (defaulted no-op for the local engine).
  The picker is Settings → UI → "Screen settings on". The bindings mirror all of
  it: `push_client_bindings_if_server` gates on scope **and** the opt-in, and is
  called both from the screen save (so *enabling* the opt-in seeds the server with
  the keys in force) and from the Controls commit (so a rebind travels too).

## The Retro Radio faceplate: removed (was fork-only, 2026-09-30; removed 2026-10-05)

**It is gone.** The operator's verdict on using it: "it's not how I intended
have to work that better" — so it was removed rather than reworked, and the
workspace is the only faceplate.

What it was: a listener's skin over the same engine, a wooden faceplate with
one big tuning scale and a needle, BAND and MODE chips, VOLUME, SQUELCH, the
receive TONE shelves, an S-meter, SCAN/SEEK and PRESET buttons. It held **no
station state** — every control pushed an ordinary `Command` — so it was
always reversible, and `Ctrl+Alt+R` / Settings → UI toggled it. Kept here
because the *reason* it was built is worth not re-deciding: a listener's screen
should not require understanding the main workspace, and the attempt to give
them one without a second program failed.

**What went, and the two wire fields with it** (`PROTO_VERSION` 192 → **193**):

- `crates/sdroxide-ui/src/app/retro.rs` (353 lines) and `SdroxideApp::retro_decode_open`.
- `UiSettings::retro_radio` **and** `ClientScreen::retro_radio`.
- `Action::ToggleRetroRadio`, its `Ctrl+Alt+R` default, its `Display` group arm,
  and its entry in the schema-2 `ADDED` list.
- The three `frame.rs` guards (top bar, band dock, panadapter branch), the
  Settings → UI row, the README bullet, the manual's list of what travels.
- `operating_panel` stays, now with one caller: it was extracted so the faceplate
  and the normal layout could share it, and it is a one-call indirection rather
  than dead code. Left alone deliberately — folding it back in is churn for
  no behaviour.

**Both wire touchpoints were the last of their kind**, which is the only reason
this is a version bump rather than a decoding incident: `retro_radio` was the
trailing field of `ClientScreen` and `ToggleRetroRadio` the trailing variant of
`Action`, so nothing above either moved a discriminant or a field.

**The trap, and it is the one to remember.** `input.json` is self-describing
JSON, so a stored `"ToggleRetroRadio"` would have made `serde_json` fail **the
whole struct** — and `load_json` on a parse failure quarantines the file and
returns defaults. The operator would have come back to a keyboard that had
forgotten all 25 bindings, with the cause in a `.quarantined` file. Removing an
enum variant is therefore never only about the variant.

`load_input_settings` now parses the bindings one at a time and keeps what it
can read, naming what it dropped — the same argument `load_json_list` already
makes for a memory list or a logbook: *hundreds of rows entered by hand over
years, each one independent of the rest*, so "all of them, because one was odd"
is not a trade anybody would take. One document and half of it unreadable is a
different case, which is why `load_json` still refuses whole. Pinned by
`one_unreadable_binding_does_not_take_the_file_with_it`, which writes a real
`input.json`, hand-adds a binding naming the removed action, and asserts the
other three survive **and the file is not quarantined**.


## The contest logger (fork-only, 2026-09-30; reviewed and fixed 2026-10-04)

A single-operator contest logger, **mode-agnostic**: the operator types the
exchange for CW, SSB or anything else, and the FT8 side auto-fills the same
entry where the contest has an FT8 layout.

**It is not "the N1MM-shaped thing the logbook was missing", which is how this
note first described it and is wrong twice over.** It is self-sufficient for
**single-operator** logging — typed and FT8 contacts, dupes, multipliers, rate,
Cabrillo, no side program — and that is the whole goal, so do not sell it short.
But it is not a replacement for what N1MM is actually bought for: multi-op
networking across stations, per-band/per-mode breakdowns, a callsign-history
database, bandmaps. On a single 11 m station none of that is in play.

**The `LogQso` datagrams are a feeder *to* software like N1MM, not a connection
to it.** `Command::LogQso` fans a finished contact out to WSJT-X UDP and to the
N1MM `contactinfo` packet (`crates/sdroxide-wsjtx`, `app = "N1MM"` so a listener
dispatches on it). Neither is interactive and neither comes *back*; the engine
keeps no logbook, because the UI owns it. A program on the other end must exist
to receive them. The Wine "LOG11DX WSJT Bridge" in this file is the same shape —
a side program listening to WSJT-X UDP — so that is not a bridge to N1MM either.

**The engine's `LogQso` does not write the logbook, and that is the bug class
below.** Any panel that logs a contact *must* also write `qso_log` itself. The
LOGBOOK window's own entry does (issue #341) and the contest window did not
until `cb7f0933`, which is why a hand-typed contest contact was silently lost on
every band. Before adding a logging path, copy that pairing rather than the
command alone.

- **Model:** `crates/sdroxide-types/src/contest.rs` — `ContestId`
  (`CqWw` / `CqWpx` / `ArrlDx` / `EuVhf` / `CbActivity` / `Generic` / `None`), an
  `Exchange` field list per contest, a `Multiplier` kind, `score` / `rate`
  estimates, and `to_cabrillo`. **Points are deliberately 1 per QSO** — the
  sponsor's band/continent weighting is theirs to adjudicate and a wrong guess
  is worse than an honest baseline; the multiplier count is real. The Cabrillo
  `CONTEST:` line picks CW/SSB/RTTY from the log's own modes.
- **Window:** `crates/sdroxide-ui/src/app/contest.rs`, opened from
  **LOGBOOK → CONTEST**. Session setup (contest picker, our exchange), one entry
  box per received exchange element, a score/rate strip, the session's log and
  the Cabrillo export.
- **State:** a `ContestSession` is the operator's own — **session-only, never on
  the wire** — and the QSOs it logs are ordinary `QsoRecord`s tagged with
  `contest_id`, carrying `stx`/`srx` and `stx_string`/`srx_string`, so scoring
  and the export read the same rows the logbook does. **No `PROTO_VERSION`
  change.**
- **`contest_id` is the sponsor's id, not the UI label** (`ContestId::log_id`,
  e.g. `CQ-WW`). It is an ADIF `CONTEST_ID` and is read by other people's
  software, so it cannot be display text — and the session's own filter reads
  the same id, so the two cannot drift apart.
- **The CB format** is `CbActivity`: a report and a **free-text** exchange, so
  it fits whatever an 11 m activity settles on. Tighten it once the exact
  exchange is known.
- **Auto-fill:** a completed digital QSO is tagged with the running session and
  gets its sent serial and our own exchange from the session, so FT8 logs itself
  (`frame.rs`, `RadioEvent::Ft8QsoLogged`). The station's **received** exchange
  comes from the digi exchange. The hand-typed path does the same write itself
  (`contest.rs`'s `log_contest_qso`, `cb7f0933`).
- **The two FT8 layouts (2026-09-30).** `ContestMode` carries **EU VHF**
  (`i3 = 5`, RST + serial + grid) and **RttyRoundup** (`i3 = 3`, RST + serial /
  state, the CQ WPX shape). `RttyRoundup` is **appended last** — it rides the
  wire, so a mid-enum insert would shift every discriminant; `PROTO_VERSION`
  184 → 185. The pack is in `modem.rs`'s `roundup` module against mfsk-core's
  own `i3 = 3` unpack order (`[tu1][h28 to][h28 from][r1 ack][s3 rpt][e13
  exch][i3]`; the exchange is a serial `1..=7999` or `8000 + state index`),
  pinned by a round trip through **mfsk-core's unpacker** — a self-consistent
  packer proves nothing. `qso.rs` runs the exchange (CQ RU → the exchange both
  ways → RR73) and logs `stx`/`srx` and the received string.
  The contest logger's **START** sets the layout: EU VHF → `EuVhf`, CQ WPX /
  Generic → `RttyRoundup`, CQ WW and the CB activity → `None` (typed by hand).
- **Review, upstream #603, worked through 2026-10-04.** The maintainer reviewed
  the upstream PR and raised twelve items; the PR is **withdrawn** (see below)
  and every item is fixed on the fork:
  - **START overwrote the digi config** and cleared an FT8 contest mode the user
    had set. One bug in two halves: the panel pushed a whole `DigiConfig` from
    its own stale copy, and `digi_contest_for` yields `None` for every contest
    but EU VHF. Fixed with `Command::SetDigiContest(ContestMode)` — one field,
    appended last, `PROTO_VERSION` 191 → **192** — so the blast radius of the
    write is the field. This is the **general shape to remember**: `DigiConfig`
    is positional and every build adds fields, so a whole-struct write from a
    panel rolls back fields that panel has never heard of.
  - **The exchange.** One box for the whole received exchange kept whichever
    element was typed last. Now one box per `ContestId::received_fields()`
    element, `SENT`/`RCVD` split rather than one copied into the other, the sent
    report defaulting by mode (`599` on CW, `59` otherwise), and
    `ContestSession::sent_exchange` putting the serial and our own text on one
    line. EU VHF's **sent** exchange gained `Exchange::Grid` — it was sending
    `RST_SERIAL` while receiving a locator, so our own grid never went out.
  - **Scoring.** CQ WW zones typed into the exchange sat in `srx_string` with
    `cq_zone` empty, so a hand-typed session counted no multipliers;
    `cq_zone_of` reads either. ARRL DX was **inverted** — `dxcc` won
    unconditionally, so DX states never counted. `Multiplier::DxccState` became
    `DxccOrState` and the rule is now *which one, by where the station is*:
    W/VE counts the entity, everyone else the state. `is_w_ve` answers from the
    cty file (`entity::resolve_callsign(...).flag`), not a prefix list — the
    rule is about location, so `K1ABC/7` is W/VE and a list would miss `KL`,
    `KP`, `VO`, `CY`.
  - **`wpx_prefix`** got three shapes wrong: a leading digit (`2E0`, `4X4`)
    came out `"2"`; a worked area (`W1AW/7`) came out `W1` not `W7`; and no
    digit gave no area instead of `0`. `/MM`/`/AM` fall through to the home
    prefix.
  - **Cabrillo:** ARRL DX does not split by mode, so `ARRL-DX-RTTY` (a contest
    that does not exist) is gone; `CATEGORY-MODE` was hardcoded `MIXED` and now
    reports what the log is; **FM has its own v3 code** and was falling through
    to `PH`; `DIGU` and the other data modes were too, because none of them
    spells "FT" — `cabrillo_mode` is a table over the modes we actually log;
    `CREATED-BY` names the program and version (passed in, so `sdroxide-types`
    does not depend on `sdroxide-version`); and a QSO dropped for having no
    frequency is counted in a comment rather than lost in silence.
  - **Dupes** ran over the whole logbook, so last month's QSO lit DUPE before
    the contest started; now over the session's own rows. **Serials** restarted
    at `001` after STOP/START or a restart; `ContestId::seed_serial` seeds from
    the highest serial already tagged for that contest.
- **11 m gets no amateur FT8 layout** (`5d8d2a7b`). `digi_contest_for` mapped
  the serial contest to `ContestMode::RttyRoundup`, whose calling message is
  literally `CQ RU <call>` — on the citizens' band that is longer than a Type-4
  call can carry (the whole identifier is capped at 11 characters), so the CQ
  did not resolve. The window now leaves the engine alone on 11 m, **and**
  `QsoMachine::contest` refuses any amateur layout there whatever
  `DigiConfig::contest` holds — the second half covers a setting persisted from
  another band or left set by an older build.
- **STOP restores the digi layout** (`5d8d2a7b`). Only START sent a
  `SetDigiContest`, so the contest's calling message stayed in force after the
  session ended. The window remembers what the engine held and puts it back.
  `None` from `digi_contest_for` means "send nothing" while `ContestMode::None`
  means "clear it" — the distinction is what keeps a session with no FT8 layout
  from clearing the operator's own setting.
- **A hand-typed contact reaches the logbook** (`cb7f0933`). `LogQso` is
  fire-and-forget — the engine sends WSJT-X/N1MM and keeps no logbook — so the
  contest window's typed contacts reached nobody: not `qso_log`, not the
  session, not the score, not the export. It now does the whole pairing the
  LOGBOOK window's own entry does. **Any new logging path must copy that, not
  the command alone.** The regression test builds a real app in an isolated
  config dir and fails on the old code with `left: 0, right: 1`.
- **Withdrawn upstream (#603, closed 2026-10-04), fork-only.** Upstream has **no
  contest logger at all** — the feature only ever existed in that PR. Reopening
  it means resubmitting the whole logger plus every fix above, CB-stripped
  (`Text` not `CbActivity`, no 11 m gate), reconstructed on `upstream/main` at
  v174. The operator's call was to leave it fork-only for now: it is
  self-sufficient for single-operator logging, the maintainer's queue is long,
  and upstream's missing logger is his gap rather than a bug of ours. Revisit
  when the fork's copy has bench hours on it.
- **The manual has an entry** (`docs/USER_MANUAL.md` §10.8, rewritten
  2026-10-04; in-app help is `include_str!` of the same file, so there is no
  second copy to drift).

### The 3D window in the multi-radio shell (2026-09-26)

The solar-system view is a **native child viewport**, and the multi-radio shell
draws only the **visible tab** — so a hidden tab's window stopped being emitted
and eframe destroyed it, to be **remapped** when the tab returned. That is the
whole of a bug class, and each fix was a round:

- **A remapped window is a new window to a tiling compositor** (niri, sway), so
  its size and place were the compositor's, not the operator's. `Solar3d::
  keep_alive` (called from `drain_events` for a hidden tab) keeps the *same*
  window mapped instead, which is the only thing that holds on a tiling WM.
- **Geometry is per screen, not per radio.** Size and position live in
  `UiSettings::solar3d_window` (`sdroxide_types::Solar3dWindow`), seeded into the
  builder only on the frame the viewport is (re)built (`cumulative_pass_nr_for
  (vid) == 0`), so a live resize is never fought.
- **On Wayland `viewport().inner_rect` is `None`** — it is built from winit's
  `inner_position`, which a Wayland client is not given — so the capture reads
  `content_rect()` (the toolkit's `inner_size`) instead, and `outer_rect` for the
  position, which is also `None` on Wayland. X11/Windows/macOS report both.
- **One window, owned by the visible radio that has it open.** `solar3d_owner
  (ctx)` is claimed every frame a shown radio's 3D is open (not just on the open
  edge — claiming on the edge let the previous owner and the returning tab each
  hold a window), and only the owner keeps its window alive when hidden. A radio
  left with `open` set from an earlier visit emits nothing while it is the shown
  tab only if it is not the owner; showing it again takes the one window back,
  so N radios cannot leave N scenes rendering.

The honest smell: the window is owned by a per-radio app, keyed by a per-radio
salt, but created and destroyed on the shell's say-so. The real fix is a
shell-owned window manager with **stable** viewport ids and an explicit owner,
emitted every frame regardless of focus; until that is done, `keep_alive` plus
the owner cap is the patch. All geometry handling is a `ROADMAP.md` item if the
window set ever grows past one.

## DMR: scoped to the codec, and **decided against** (2026-10-11 — read before replying to anyone asking again)

**The verdict, and it is the operator's: we stay quiet on DMR. Nothing is built,
nothing is promised, and the next person who asks gets the answer below.** The
reason is **a patent, not a licence** — and the distinction is the whole finding,
because "we looked into it" has to mean what was actually checked.

### What was asked, and by whom — three times, two people

- dig647 (Wolfgang), fork **issue #28** *"Kanalabstand"* (2026-10-09), then
  *"und vielleicht noch DMR"*. Kevin answered both halves in the thread: the
  channel spacings went up as **PR #32 (merged 2026-10-10)**, and DMR was
  **"notiert, aber nicht geplant"** — noted, not planned, *"der schwierige Teil
  ist der Sprach-Codec (AMBE+2), das ist ein größeres Projekt"*.
- Wolfgang's **last word on #28** (2026-10-10 04:26) is a push on exactly that
  objection: *"bezüglich dmr gibts ja mbe codec du kannst sicherlich was damit
  anfangen…"* — there is the MBE codec, surely you can do something with it.
- dig647 again in fork discussion **#15**, and **upstream #607** (Itsisme, DMR /
  YSF / D-Star). So: three requests, two of them pressing the codec point.

**What our earlier note said was wrong**, and this is the correction worth
keeping: it recorded that DMR's hard part is the **AMBE+2 voice codec**. The
codec turns out to be the *easy* half — vendored C, a licence we already
satisfy. The hard half is the air interface, and nobody had looked at that.

### The codec: no licence wall, and that is genuinely good news

**Wolfgang is right that the codec is a solved, take-it-away problem.**

- The original **mbelib is ISC-licensed**; **`mbelib-neo`**
  (`arancormonk/mbelib-neo`, a fork of `lwvmobile/mbelib`) is
  **GPL-2.0-or-later** and is the one to use — it has a modern CMake build with
  installable headers and pkg-config, and it already covers **AMBE+2 (DMR, NXDN,
  P25 Phase 2, dPMR)**.
- **We are GPL-3.0-or-later**, so GPL-2.0-or-later is compatible and ISC is
  cleaner still. **There is no licence obstacle of any kind** — contrast
  RadioScript, which has none because it has no licence *at all*
  (`ON RESEARCH, and NOT OURS TO BUILD`, above). This one is the opposite case,
  and it is worth knowing that not every "closed codec" is a closed door.

So the build is a `vendor/` and a `-sys` binding, the same shape as `rade_c`.

### The patent, which is why the answer is no

The clean pure-Rust crate that exists for this family,
**`swiftraccoon/mbelib-rs`**, says in its own README why it does **not** do
DMR:

> Does not support AMBE+, AMBE+2, IMBE, or the AMBE+2 "half-rate" variant
> used by DMR / YSF / NXDN (**which remains covered by US Patent 8,359,197
> until 2028-05-20**).

**That asymmetry is the finding.** There is a clean, no-unsafe, pure-Rust decoder
for **D-STAR** (AMBE 3600×2400) and none for **DMR**, and the reason is the
patent, not the engineering. It **expires 2028-05-20**, about nineteen months
out.

**Two things that are not ours to decide, recorded so the question is closed and
not re-derived:** we are not a patent office and this was read off a crate's
README, not a claim chart — **if anyone ever wants to revisit this, it is a
lawyer's question, not an agent's.** And the operator's own ruling
(2026-10-11) is the operative one, and it is unconditional: **we stay quiet on
this one, we are not messing with patents in any way.**

### Why the ecosystem having it does not change the answer (checked 2026-10-14)

The operator's fair challenge: *"there is a ton of software out there that offers
it."* True — and looking at **how** they offer it is what settles it, because almost
every one of them avoids the exact position this fork is in.

- **DSDcc / SDRangel (f4exb)** is the honest engineering answer, and it is in their
  own README: *"While DSDcc is intended to be patent-free, `mbelib` that it uses
  describes functions that may be covered by one or more U.S. patents owned by DVSI
  Inc... **Compiling or using `mbelib` may infringe on patents rights in your
  jurisdiction and/or require licensing.** It is unknown if DVSI will sell licenses
  for software that uses `mbelib`."* **And it ships without it** — `mbelib` is a
  compile-time opt-in (`-DUSE_MBELIB=ON`), so the *distributed binary* carries no
  codec and can feed a **ThumbDV dongle** instead. The project ships the frames; the
  user opts into the patent.
- **The dongle route** is DVSI's own licensed silicon (ThumbDV / AMBE-3000, via
  `serialDV`). Clean by construction, because you are buying the licence.
- **mbelib and DSD's authors are anonymous** and identify by cryptographic public
  key — Bruce Perens' *"AMBE Exposed!"* says why in as many words: *"because those
  folks do not wish to tangle with DVSI and its attorneys."* Perens also dates the
  patent — **8,359,197B2, filed 2003-04-01, granted 2013-01-22, extended, expiring
  ~22 May 2028** — and concludes: *"The only problem is patents."*
- **DSDPlus is closed and paid.** That is the other way out.

**So the pattern is: ship no codec, use licensed hardware, or stay anonymous.** This
fork is a public project with public releases and a named maintainer, which is
precisely the position all three are avoiding. The popularity of DMR software is
evidence about tolerance, not about permission.

**What would technically be clean, if it were ever wanted:** the **air interface is
patent-free** and is the bulk of the work — C4FM demodulation, the DMR data link,
call signalling — ordinary DSP of the same shape as the VDL2 and HFDL lanes. A
receive-only "frames and talkgroups, no voice" mode, with the codec left to a dongle
or a build-time opt-in, is the DSDcc posture and is defensible. It is also a poor
feature for this fork's users by itself, since the point of DMR to a listener is the
voice. **And the priority rule has not moved**: DMR is VHF/UHF commercial two-way
with no 11 m or SWL audience, and this fork takes the CB/listening choice when the
two compete.

**Still the operator's call, and still not reopened by an agent.** The above is the
ground the next person who asks deserves rather than a re-assertion.

### How much work it would have been, for the record

Worth keeping because "it is a big project" was true of the wrong layer. The
vocoder was never the work. DMR's air interface is **C4FM** —
continuous-phase 4-level FSK, 1200 baud, in a **12.5 kHz** channel — and what a
receiver actually needs is three layers, only the last of which is the codec:

1. a **C4FM demodulator**: symbol timing recovery, differential mapping,
   de-whitening, and the embedded signalling bits that ride *in* the symbol
   stream itself;
2. the **DMR data link**: burst/frame/superframe sync, the four data types, call
   signalling (source and destination IDs, talkgroup), encryption indicator,
   SMS;
3. **voice framing**: 72 AMBE+2 bits per 20 ms, its FEC, the embedded-signalling
   bits swapped out, handed to the vendor library for PCM.

Layers 1 and 2 are the bulk, and they are ordinary DSP — the same shape as the
**VDL2 and HFDL** lanes already in the tree. **DAB is the closer precedent**,
because it also has to turn a wideband lane into audio on the speaker. So
**receive-only DMR voice, promiscuous mode, is a sitting job and not a project**;
transmit is a second and separate decision. **The codec was never the wall.**

### Hardware: none, which is worth saying out loud

No front-end problem at all. 12.5 kHz out of a wideband lane, and the machinery
that carries one already exists (ADS-B, AIS, VDL2, DAB); the RSP1 on the bench
reaches 2 m and 70 cm easily. A cheap FM set will **not** do it — this is not
analogue FM voice. The only real constraint is sample rate for 70 cm (a wider
band than 2 m), and that is the existing wideband-lane plan, already solved.

### Priority, which would have been the second no

Even without the patent, our standing rule settles it: when a choice is between
a ham workflow and a CB or listening one, **this fork takes the CB/listening
one**. DMR is VHF/UHF commercial two-way radio. It has nothing to do with 11 m
or with SWL, and it has no audience here beyond two people who have asked.

**Do not reopen this without the operator saying so.** Kevin's answer in #28 —
noted, not planned, the codec is a big project — was right in its conclusion and
wrong only in its reason; the reason is the patent, and it does not expire until
May 2028.

## Explore later

- **NR2 (WDSP's Ephraim-Malah denoiser)** — **landed upstream on the 2026-09-20
  merge** (`0d03b507`, plus #515's review commits) as the fifth `NrEngine`, so
  the fork carries it now and this watch item is closed. Two things that were
  open when it was queued are settled: upstream **did** bump `PROTO_VERSION`
  for the appended `NrLevel` variants (their 159, folded into the fork's
  register), and the **454 KiB `nr2_tables.bin`** does **not** reach the
  browser — `sdroxide-ui` depends on `sdroxide-types`, not `-dsp`/`-radio`, and
  those are native-only under `cfg(not(target_arch = "wasm32"))`, so
  `cargo tree -p sdroxide-ui --target wasm32-unknown-unknown` carries neither.
  The five engines are RNN, DeepFilter, SpecBleach, **NR2** and Spectral.

## Build and test

- `cargo build --release` — the full binary (needs the vendored submodules,
  `vendor/xng` among them now; see the README's Building section).
- **A sandboxed build of the radio engine needs two hosts on the network.**
  `sdroxide-rade` builds opus through `vendor/rade_c/cmake/BuildOpus.cmake`'s
  CMake `ExternalProject`, which fetches the opus source from
  **`github.com/xiph/opus`** and, during opus's own build, its DNN model data
  (LPCNet/DRED) from **`media.xiph.org`**. A cloud/CI sandbox that allows one
  and not the other stalls with a fetch error inside the nested opus build and
  no mention of RADE — reported by a tester's Claude cloud session that could
  not compile the radio engine because `media.xiph.org` was blocked. Allow both,
  or build that crate where they are reachable. Nothing else in the tree
  fetches at build time.

  **In a Claude Code cloud session, allowing `github.com` is not enough.** The
  session's proxy serves GitHub *git* reads (clone/fetch) but refuses the opus
  **archive** URL with a 403 ("GitHub access to this repository is not enabled
  for this session"). The way through, with `vendor/` untouched: fetch the
  pinned commit by git, zip it, and point `OPUS_URL` at the zip through a
  CMake toolchain file that `cmake-rs` reads from a **target-specific**
  variable (plain `CMAKE_TOOLCHAIN_FILE` is not picked up):

  ```sh
  S=/path/to/scratch; C=940d4e5af64351ca8ba8390df3f555484c567fbb
  git init -q $S/opus && git -C $S/opus fetch -q --depth 1 https://github.com/xiph/opus $C
  git -C $S/opus archive --format=zip --prefix=opus-$C/ -o $S/opus.zip FETCH_HEAD
  echo "set(OPUS_URL \"file://$S/opus.zip\" CACHE STRING \"\")" > $S/opus-local.cmake
  export CMAKE_TOOLCHAIN_FILE_x86_64_unknown_linux_gnu=$S/opus-local.cmake
  ```

  Delete `target/*/build/sdroxide-rade-*/out` first if a failed configure is
  cached. `soapysdr-sys` also needs `libsoapysdr-dev` from apt. Verified
  2026-10-08: `sdroxide-rade` builds and the `sdroxide-radio` dial tests run.
- `cargo test --release --workspace` — everything. The `sdroxide` bin's
  `icomnet_source` tests flake now and then when the whole workspace runs at
  once and pass when that binary is run alone; re-run
  `cargo test --release --bin sdroxide` before chasing a failure there.
  Likewise `sdroxide-pluto`'s `iiod_loopback` (a scheduling flake under the
  parallel workspace run, passes alone).
- **After a merge, test the packages you touched rather than the whole
  workspace**, e.g. `cargo test -p sdroxide-types -p sdroxide-proto -p
  sdroxide-ui -p sdroxide-radio -p sdroxide-dsp -p sdroxide-hfdl`. A merge
  rarely reaches the audio/USB crates, and the whole-workspace run spends most
  of its time in the two flakes above. Reserve `--workspace` for cutting a
  release. Do not skip the merge-time run: the invariants it holds are
  `help::anchor_links_resolve_to_headings` (a manual cross-reference broken by
  a heading choice), `mode_discriminants_are_stable` / `nr_discriminants_are_
  stable` (a variant inserted rather than appended), the PROTO register, and
  the mode/band tables — every one of which has caught a careless merge edit,
  and none of which a reviewer would have found by eye.
- **The radio crate's test set is the expensive one — prefer a single test
  binary.** `cargo test -p sdroxide-radio` builds and links **72** integration
  test binaries, each carrying the whole engine (RADE, ADS-B, DAB, the vendored
  C), so it is minutes of **linking** even when nothing changed — that is the
  answer to "why is the test run so long when it compiles fine". Chasing one
  thing, name its binary: `cargo test -p sdroxide-radio --test cw_panadapter`,
  not the whole package. `cargo check --all-targets` only type-checks (seconds);
  `cargo test` codegens and links, and that is what actually costs the time.
- `cargo check --release --target wasm32-unknown-unknown -p sdroxide-ui` — the
  browser client, which shares the same UI code.
- `cargo test -p sdroxide-digi --release -- --ignored --nocapture sensitivity`
  — the FT8/FT4 receive-sensitivity sweep. It measures the floor of *our* chain
  (the 12 kHz path, i16 scaling, the search window), not mfsk-core's intrinsic
  one: the decoder is the same engine WSJT-X and WSJT-CB run, so a difference
  against them can only be the plumbing this measures. Reports SNR in the
  2500 Hz reference bandwidth every FT8 figure is quoted in; the floor lands
  near −24 dB (a decode's own reported −21 dB). Slow and a judgement rather
  than an assertion, hence `#[ignore]`d. The other flakes under a full
  workspace run but pass alone: `sdroxide-deepcw`, and the
  `sdroxide-tci`/`icomnet_source` ones above.

  **`skim_window` is load-sensitive, not red — and the note that called it red
  was itself the error.** Re-measured 2026-10-06:

  - Unloaded, this machine: **passes in ~9 s against a 45 s deadline, 12 of 13
    runs.** The thirteenth hit the deadline.
  - Under load (twelve busy loops on a 16-core box): **3 of 3 runs fail**, every
    one at the full 45 s.

  And it is the *same code*. `crates/sdroxide-radio/tests/skim_window.rs` is
  **byte-identical** between `v1.9.18_brown` and now, and the skimmer's source
  is unchanged too — so the tag it was called "genuinely red" on behaves exactly
  as this one does. **The earlier note saying it "flakes under a parallel run and
  passes alone" was right, and the correction that replaced it was wrong.** That
  correction cost this session the hour it was written to save.

  **Why it flakes, and it is not the decoder.** The skimmer takes IQ over a
  **bounded channel and drops what it cannot keep up with** — the test's own
  comment says so. Carrier energy survives dropped IQ; character decode does
  not. So under CPU pressure the station is still spotted and the callsign never
  arrives, which is the exact shape the failure reports: on a failing run,
  **~110 spots, every one at 14.0599–14.0600 MHz, and not one with a callsign.**

  **So window-following works.** Those spots can only exist if the window moved
  onto the view and stayed there, which is what the test exists to check. What
  fails is the claim the test attaches to the *text decode*, and that belongs to
  the skimmer's own tests rather than to a window-placement one — coupling the
  two is why this fails in any parallel workspace run.

  **No fix is recorded here, deliberately.** More headroom for the worker, or a
  larger channel, does nothing under sustained saturation, and lowering the
  test's 400 kHz span would weaken the premise it was chosen for. The defensible
  change is to split the assertion — a spot at the right frequency as the window
  test, the callsign as a separate strict one — and weakening an assertion is not
  something to do on the way past. **Read a failure here as "the machine was
  busy", not as the skimmer being broken.**

## The bench

An **SDRplay RSP1** is attached to this machine and reachable through the
SDRplay API service (`sdrplay_apiService`, running as a **system** unit — it
is not `sdrplay_api` and `systemctl status sdrplay_api` says "could not be
found", which reads like a missing dongle and is not). Gain range 20–59 dB,
0.01–2000 MHz, one RX channel. It samples real RF — a 5 s capture at 10 MHz
with max gain showed the WWV/WWVH cluster around 10.000 MHz at ~30 dB over a
~1 dB noise floor — so it is usable for decoder bench work.

> **Do not trust `SoapySDRUtil --find` on this box.** It reported *"No devices
> found!"* on 2026-10-11 while **both** radios opened in the program without
> trouble. `Backend::SdrPlay` and the `SoapySDRUtil` sdrplay driver do not go
> through the same path, so the utility saying nothing says nothing about the
> radio. It cost a wrong call on the bench before the RSP1 was reported
> missing — the instrument was the first suspect and should have been the
> first place to look.

Two things worth keeping:

- **Capturing headlessly.** There is no timed-capture flag; `--record-iq`
  records the GUI's stream. A throwaway SoapySDR capture tool does the job:
  open `driver=sdrplay`, set rate/frequency/manual gain, `rx_stream` →
  `read` → write interleaved CF32 (little-endian f32 pairs), which `--file`
  reads back. It needs `soapysdr` as a direct dependency, which the root crate
  does not carry, so it lives outside the tree (a scratch crate that
  path-depends on `vendor/soapysdr`).
- **`tools/rsp1-capture` builds in place now** (2026-10-11). An empty
  `[workspace]` table in its `Cargo.toml` stops cargo walking up to the root
  manifest and refusing the tool, and its `soapysdr` path-dep is relative
  rather than absolute, so the copy-to-scratch dance is over. It is still its
  own workspace, so `--workspace` at the root never builds it and a mistake
  in it cannot fail a release.
- **`rspcap` silently ignored the sample rate you gave it.** On 2026-10-11 it
  printed *"invalid sample rate. Sample rate unchanged."*, kept the radio's own
  rate, and then wrote `rate × seconds` samples regardless — so every
  frequency in the file was wrong while the file size looked exactly right. It
  worked at 2.048 Msps and failed at 48 kHz. **Fixed**: it reads
  `sample_rate()` back, captures at what the radio is *actually* doing,
  reports asked-vs-got and the ratio the frequency axis would have been wrong
  by, and warns on a mistune the same way. **Not yet run** — the RSP1 was held
  open by a running sdroxide when it was built, so it is verified to compile
  and nothing more.
- **Some channels are simply dead.** 2187.5 kHz (MF DSC) was flat noise over a
  3-minute watch here, and 8414.5 kHz had no burst in a 20 s capture; 40 m was
  busy at the same time. A missing signal is not a broken decoder — check a
  known carrier (WWV at 10 MHz) before blaming the DSP.
- **Use it to prove recording behaviour rather than writing "not tested".**
  The operator's standing note (2026-09-28): if a change turns on something that
  needs real RF to show, reach for the RSP1 rather than settling for a unit test
  and a caveat. **Transmitting is authorised**: the operator is on a
  **licence-free frequency**, and we are free to test — **keep transmissions
  short**. That is permission to key up, not permission to guess a dial: a rig
  with no CAT link has to be tuned by hand, so ask which channel and confirm it
  before anything goes out, and never sit on the air.
  It was live at this writing (`SoapySDRUtil --find` sees
  `sdrplay Dev0 RSP1 0000000001`, so the `sdrplay_api` service is up) and there
  is **no Pluto on the network** — `pluto.local` does not resolve and
  `192.168.2.1` does not answer, which is also why `sdroxide-pluto`'s
  `iiod_loopback` is a local mock and flakes under a parallel run rather than
  needing hardware.
  - What it settles: the **recording silence auto-split** end to end. The gate
    reconstructs the engine's squelch from the published meter, so the thing a
    real receiver proves and a synthetic number cannot is that `passband_dbfs`
    against a real signal crosses the threshold the way the code assumes. Pick
    a **keyed** signal — a CW beacon, say — and the whole chain is observable.
    **Better, because it is fully under our control: the loopback below.**
  - **The loopback test, which needs no new code.** A **CRT SS9900v** (11 m CB)
    is on the bench, registered as radio 1 and reachable **through VOX and its
    USB audio, with no CAT link** — it is a `UsbAudio` radio, so the program
    both receives the rig's audio and sends audio to its input, and VOX keys on
    what arrives. That makes the rig a *transmitter under our control*, which is
    the test vector the gate needs: the RSP1 receives on the same band in the
    same program while the rig keys, so the squelch follows real RF with timing
    we choose. The way to key it is the **CW keyer panel's KEY** with `cw_keying
    = Sound card (MCW)` —
    the keyer's sidetone is transmitted as audio to the rig, VOX picks it up, and
    the paddle (**CH55x `1209:c550`**, also on the bench) gives exact on/off
    edges. This route is **confirmed on air** (2026-09-27, iambic and straight).
    So: key 3 s, release about 2–3 s so the gate orders its stop, key again. One
    stamped file per keying, and the second transmission lands while the first
    file is still closing — which is exactly the `RecGate.stop_asked` path that
    was dropping the transmission ending the silence run. No software, a paddle
    and a stopwatch. **The operator tunes the rig and arms the row** — no CAT
    link and no scripting, so both need a hand; what an agent can do is check
    the signal path and read the results afterwards.
  - **The numbers, so it is not re-derived.** The hold is the **2 s** chips —
    the shortest, and the one that leaves the least room for the race. The gap
    between the two transmissions should be **2–3 s**: long enough that the gate
    reaches its `Some(since) if now - since >= 2 s` arm and orders the stop, and
    short enough that the recorder is still closing when the second one starts,
    which is the whole point. The gate's start is re-asked for up to
    `REC_START_TIMEOUT_S` (3 s), so a gap much under 2 s would be swallowed as
    one transmission and prove nothing. The rig's carrier alone is enough — the
    test is about the **squelch crossing**, not about intelligible audio, so the
    keyer's sidetone need not be decoded. **Check the meter first**: with the
    rig keyed, `passband_dbfs` must sit clear of the set `squelch_db`, or the
    gate never opens a file and the run tells you nothing about the gate. Then
    **look for the files** in the config dir's `recordings/` (`config_dir()` is
    the `-brown` one), named with the existing UTC/frequency/mode stamps: one
    per keying, and the second one *existing at all* is the assertion — under
    the old logic it was dropped and no file appeared.
  - **What is actually plugged in (checked 2026-09-28, and the two are always
    connected).** `radios.json` registers two, and they are the two devices
    this section is about. `Scope::None` is the top-level `radio.json` and
    `Scope::Some(id)` is `radio-{id}/` (`sdroxide-config`'s `RadioScope::dir`),
    so:
    - **Radio 0, unnamed → the top-level `radio.json`, `backend = SdrPlay`**, so
      the RSP1 is the first radio by default and needs no configuration at all.
    - **Radio 1, named `CRT SS9900v` → `radio-1/`, and it is
      `backend = UsbAudio`**, with `radio_audio_out` = `Generic AB13X USB Audio,
      USB Audio [Audio · 001f:0b21]`, tuned to **27.265 MHz**, mode FT8.
    `radio-2`…`radio-6` are stale leftovers from earlier experiments
    (`next_id: 7`); the `SpyServer`, `Cat`/TrUsdx, `KiwiSdr` and `AtsMini`
    configs in them are not live hardware.
  - **The CRT's own audio drops out, and it is the radio's fault, not ours
    (operator, 2026-10-18).** On the SS9900v the waterfall is streaked where the
    RSP1's is smooth, **the sound drops at the same time as a black line in the
    waterfall**, and FT8 decodes nothing there while the RSP1 on the same
    machine decodes 300 in a session. The drops are **the rig** — the operator
    traced it to the radio itself. Do not go looking for a buffer underrun in
    the engine: a demod-audio rig's slot is only as whole as the audio the rig
    sends, so a rig gating its own audio is a station fault with a decode
    symptom. `cat.ptt`/`mode_control`/`cw_keying` on `radio-1` are inert — the
    `UsbAudio` path reads none of `radio.cat` (it keys by VOX off the audio it
    plays), so a `'Cat'` in those fields is a leftover, not a broken link.
  - **A wireless mouse radiates into the SS9900v (operator, 2026-10-18).** Moving
    it swings the S-meter hard; the trackpad does not. It is the mouse's own
    transmitter, not the USB link — worth knowing before reading an S-meter on
    that rig, and before blaming the app for a level that moves on its own.
  - **The rig's USB card has two independent levels, and only one is in the
    app (2026-10-18).** `amixer -c 2` on the AB13X: `Mic` is the *capture*
    switch and `PCM` is the *playback* volume, and they are the receive and
    transmit halves of the same chip. `PCM` had been left at **2% (−44 dB)**, so
    CQ transmitted, the panel said so, and the rig never keyed — VOX had nothing
    to hear. Nothing in Settings → Radio can show or fix that, which is what
    made it look like an app fault. **The symptom to remember: the program says
    it is transmitting, the meters move, and the radio stays silent — check
    `amixer -c <card> sget 'PCM'` before touching anything in the program.**
    ALSA restores this level at login, so it survives a reboot.
  - **The FT8 CB CQ fix is confirmed on the air — by loopback, and then by a
    real station (2026-10-18).** `ac9183fe` (`.also_accept` put back on the
    streaming decode request) was proven offline first: pull the hook out and
    the guard loses **exactly** the four CQ forms while the three report forms
    still arrive. It was then proven end to end the same evening, with no
    propagation needed — **the RSP1 on 27.265 receiving the CRT's own CQ** — and
    a real station on the band was copied in the same session, so it is not only
    the bench talking to itself. That loopback is the technique to reach for
    when a decode fix cannot wait for the band: two radios in one program, one
    keying, one decoding, no ionosphere in the path. Worth keeping, because the
    alternative is an evening lost to a dead band.
  - **This corrects an earlier note here, which was wrong twice.** The rig is
    **not** unusable as a radio: `Backend::UsbAudio` is
    "USB audio radio (sound card)" — the same audio machinery as a demod-audio
    CAT rig, *minus the serial* — so the SS9900v opens, tunes and meters like any
    other radio, and carries audio **out** to the rig's input for VOX to key.
    And the `Backend` enum is much longer than an earlier reading of it
    suggested (`Auto`, `Soapy`, `Cat`, `Hpsdr`, `Tci`, `RtlSdr`, `Rx888`,
    `SmartSdr`, `Pluto`, `SdrPlay`, `None`, `AirspyHf`, `IcomNet`, `RtlTcp`,
    `Lime`, `HydraSdr`, `Fobos`, `UsbAudio`, and network sources
    `SpyServer`/`KiwiSdr`) — it was truncated, and the conclusion drawn from the
    truncation ("no second source, so a hidden-tab test needs a dongle") was
    wrong. **Two real radios are registered, so the hidden-tab test does not
    need anything bought:** arm the gate on one, show the other's tab, and the
    hidden one must keep splitting files.
  - **The squelch is the thing that will silently make the test prove nothing.**
    `squelch_db` is `-150.0` in **both** saved sessions, and that is
    `SQUELCH_OPEN_DB` — "squelch fully open (slider minimum)", the default, and
    `passband_dbfs >= squelch_db` is then true for the noise floor. The gate
    would see unbroken signal and never close a file; worse, the Auto-record row
    treats it as no squelch at all and **does not offer the chips**
    (`squelch_open` is `squelch_db <= SQUELCH_OPEN_DB + 0.5`). So **tighten the
    squelch above the noise floor first**, on whichever radio carries the gate,
    and watch the meter with the rig keyed: the carrier has to sit clear of the
    threshold, or the run tells you nothing about the gate.
  - **What still cannot be settled: a minimised window.** That is a windowing
    fact, not an RF one, and no bench gear proves it. Two real radios do settle
    the *hidden tab*, which is the part the fix addresses. The honest claim for
    the OS-hidden case stays "the gate asks for its own frames, and
    `recording_stop_at`, auto mode and the reconnect countdown still do not run
    on a window that is not being drawn".

## The subtract short-buffer fix: our first patch was wrong, and the symptom it left is worse than the panic (2026-10-05)

`vendor/mfsk-core` carries a patch. It has now been rewritten, and the reason
is worth more than the patch: **the first version silenced a panic by turning
it into a silent partial subtract, and the fork shipped that for a day.**

### What the bug is

Upstream **issue [#567]** (not a PR request): `engine::dsp::subtract`'s
`apply_at_offset` sized the FFT from the buffer (`nfft = audio.len()`), where
`subtractft8.f90` (v3.2.0-rc1, lines 10-11) fixes `NFFT = NMAX = 15*12000` —
always at least `NFRAME`, so `camp`/`cfilt` can hold the whole frame and a
short `dd` is simply zero-filled. On a slot buffer shorter than the frame,
`cfilt` is shorter than `nframe` and two lines index past its end.

**It is not the `6245c34` branch-hoist**, which our first report blamed. That
commit hoisted the per-iteration `j >= 0 && j < audio.len()` check into the
two clamps; the clamps are equivalent for `audio[j]` but were **never a bound on
`cfilt[i]`**. The end-correction block is byte-identical at `6245c34^`. There
was no bound to lose.

### Two corrections to our own report, both from the maintainer, both verified

- **`dt` need not be negative.** With `endcorrection = true` — the FT8
  wrapper's own setting (`ft8/subtract.rs:55`) — a short buffer panics at
  **every** `dt`. Measured on `main` at `148a9e85`, a 130 284-sample buffer:
  `dt` −2.0 panics at `subtract.rs:735` (camp build, index 130 284 = `nfft`)
  with end correction *either* way; `dt` −0.5 / 0 / +1 / +2.5 panic at `:752`
  (end correction, index 151 679 = `nframe-1`). Our report's off-air story
  ("the mis-aligned slot's `dt` is negative") understated the trigger by
  exactly the case that matters.
- **The root cause is the FFT length, not the loop bounds.** Clamping `i_hi`
  by the buffer stops the panic and **leaves the last `|signed_start|`
  samples with no subtraction at all**, and an FFT shorter than the frame
  wraps the LPF around its own ends.

### What our shipped patch actually cost — the number to remember

Measured with jl1nie's own `subtract_short_buffer.rs`, against the whole-slot
result on the same samples:

| | late-attach tail | whole overlap | full slot (reference) |
|---|---|---|---|
| our clamp (`db8b8fa`) | **0.0 dB** | −4.9 dB | −36.5 dB |
| `nfft` fix | **−36.5 dB** | −30.0 dB | −36.5 dB |

**0.0 dB is not "degraded", it is absent** — 2.6 s of signal not subtracted at
all. Since SIC *is* the recall mechanism for FT8 `Deep` (the default), the
pass did its ~1.1 s of work and returned nothing, while the panel still showed
decodes from the other passes. **A loud panic became a quiet wrong answer,
which is the harder failure to notice and the worse one to ship.**

The general lesson, and it is the same shape as §10's: **a fix that makes the
symptom stop is not a fix until the *value* is right too.** Clamping bounds
made the panic go away and left the subtraction silently partial. The test that
settled it was not "does not panic" but "does the whole overlap reach what a
full slot reaches" — a question about the result, not about the absence of a
crash.

### Where the fix lives, and why there are two of them

- **Upstream [PR #574](https://github.com/jl1nie/mfsk-core/pull/574)**, branch
  `fix/subtract-short-buffer`, one commit on current `main` (`148a9e85`,
  version **0.13.0**). `nfft = audio.len().max(nframe)`; the existing clamps
  and the end correction are then correct as written. Also threads `nframe`
  into `residual_band_power` (`sqf`) so the `−90/0/+90` trial search scores on
  the grid the subtract used — flagged separately in the PR body in case he
  wants the diff kept to one line.
- **The vendored copy** is branch **`fix/subtract-short-buffer-0.11`**
  (`cb340709`), the same fix rebased onto **`d243359a`** (version 0.11.0).
  **It cannot be the 0.13.0 commit**: 0.13.0 removed `DecodeRequest` /
  `SniperRequest` / `MultiPeriodRequest` as public API, and this fork's decode
  calls are built on them. Re-vendoring at 0.13.0 is a migration project, not
  a submodule bump — do not "just bump it". The pin in the test is re-recorded
  for this base (`1_301_204_944_408_214_188`, verified unchanged by the fix)
  because 0.11.0 synthesises via `ft8::wave_gen::tones_to_i16` where 0.13.0
  uses the generic `engine::tx::synthesize_i16` — a different waveform, so a
  different hash. Each base is pinned against its own unfixed output, which is
  the property that matters.
- **When #574 lands**, drop the `[patch.crates-io]` entry and return to the
  plain 0.11 dependency. Do not bump to 0.13 as part of that.

### Verified

His three tests, on both bases: `short_buffers_do_not_panic` and
`late_attach_subtracts_the_whole_overlap` fail on the unfixed base (panic /
0.0 dB) and pass with the fix; `full_slot_output_is_pinned` passes on both,
which is its whole point. Tier A+B on the 0.11 base: **103 test binaries, 0
failures**. On the 0.13 base the same suite introduces no new failures —
`decode_snapshot` failed there, but that failure was **a real upstream bug,
reported by us and now fixed** (see the `decode_snapshot` entry below); it was
never "a fixture mismatch on this box". SIC-specific gates green
on the 0.11 base: `qso3_full_parity_meets_wsjtx_golden_floor`, the three
`sic_early_*` tests, `ft4_subtract_pipeline`. Fork side:
`cargo check --workspace --all-targets` silent, `cargo test -p sdroxide-digi
--release` green (513 in the main binary).

### The `decode_snapshot` failure was an upstream bug, and it is fixed

The one test that failed on the 0.13 base was **not** a quirk of this box, and
an earlier note here said so wrongly ("pre-existing fixture mismatch on this
box, not ours"). It was a genuine portability bug in mfsk-core's own test, found
because our box is one of the machines it breaks on.

**What it was.** `decode_snapshot`'s fixtures pin `f32::to_bits()` for `freq_hz`,
`dt_sec`, `snr_db` and `sync_score` and compare with `==`, so the last bit of a
float becomes a property of the *compiler's* codegen and the *platform's* libm
rather than of the decoder. On this box (CachyOS, glibc 2.44, Ryzen 7 7435HS)
seven tests fail: `ft4_request_shapes`, `ft8_request_shapes`,
`iq_receiver_rows`, `q65_request_shapes`, `via_decoder::{ft4,ft8,q65}`. Same
messages, same order, every integer field identical. Re-measured in strict mode,
the deviation is **four rows across four fixtures** — wider than the original
report claimed, which had diffed `ft8_default` alone and called it "two `snr_db`
rows":

| fixture | row | column | fixture → now |
|---|---|---|---|
| `ft8_default` | 4 | `snr_db` | −9.148531 → −9.148533 (2 ULP) |
| `ft4_default` | 5 | **`freq_hz`** | 1909.7092 → 1909.7094 |
| `q65_30a_averaged` | 0 | `snr_db` | −20.01653 → −20.016531 (1 ULP) |
| `iq_direct_ft8` | 14 | `snr_db` | −0.09790039 → −0.097904205 |

All four sit far inside `Tol` (the worst is `freq_hz` 1.9e-4 Hz against a
5e-3 Hz limit, ~26× margin), so this changes nothing about the fix — but it does
mean **"freq_hz is always identical here" was an over-read of one fixture**, and
the note is worth having right before anyone compares a machine against it.

**Why it was worth filing rather than ignoring.** The obvious read is "our
toolchain, our problem", and that read is wrong twice: by jl1nie's measurement
1.98.1 and 1.99.0 give byte-identical output on both the Zen 2 and the M5, and
the SNR path calls `f32::log10`/`powf`, which are **platform libm**, not
repository code — Apple against glibc differ on 5.9 % of `log10` inputs. The same
reasoning as the Olivia polarity entry applies — the answer was in the artifact
only a second machine could supply.

**Reported as [#579](https://github.com/jl1nie/mfsk-core/issues/579), fixed by
jl1nie in [#581](https://github.com/jl1nie/mfsk-core/pull/581)** (`f6323dca`,
merged 2026-10-05, into 0.13.1). **He wrote the fix himself** after we offered
to send data, because he wanted the limits to come from his own measurements on
several architectures — which is the right call and worth remembering as the
shape of a good response: take the credit for the diagnosis, keep the work. Rows
are compared within `Tol` (`freq_hz` 5e-3 Hz, `dt_sec` 1e-5 s, `snr_db` 0.1 dB,
`sync_score` 1e-3 relative, `hard_errors` ±2, each 2.5-5× the largest measured
gap); messages, row count, order and `pass` stay exact;
`MFSK_SNAPSHOT_STRICT=1` restores bit-exactness and is deliberately **not** set
in CI, because "a runner image that updates its glibc" is exactly the failure.

**Verified here (2026-10-05):** `main` at `f6323dca` gives **15 passed, 0
failed**, `MFSK_SNAPSHOT_REPORT=1` prints nothing (every column inside `Tol`),
and `MFSK_SNAPSHOT_STRICT=1` fails the same seven tests with the same 2-ULP
`snr_db` row — so the tolerance is what fixed it, not a coincidence. Our
`rust-toolchain.toml` pin is **not** honoured on this box (Arch's rustc, no
rustup), so that run used 1.99.0, which is a second independent machine behind
the "toolchain is not the variable" claim. Probe hashes and `ldd --version`
(glibc **2.44**, against their 2.35 reference) posted to #579 at his request.

**For the fork, the practical consequence: our vendored 0.11 copy is
unaffected** — `vendor/mfsk-core/tests/decode_snapshot.rs` does not exist on
that base at all, so the test cannot be failing there — and a future re-vendor
to 0.13+ should *not* treat `decode_snapshot` as a known-bad local failure. FT8's dispatched slot is 15.0 s = 180 000
samples (`mode.rs:848`) against `nframe` = 79 × 1920 = 151 680, so a slot has
to be **> 0.7 s short** to reach this at all, and `check_slot_arrived_whole`
only *warns* at 0.95 (`controller.rs:772`) without suppressing the dispatch —
which is how a short buffer reaches the decoder at all. FT4's wrapper passes
`endcorrection: false` and its frame (53 760) is well inside its 90 000-sample
slot. **Not bench-tested on air** — the fix is a DSP invariant restored from the
Fortran, with the evidence above, and the 0.7-s-short slot is not something to
go looking for on 11 m.

### One thing noticed and deliberately not touched

`ft4::subtract`'s doc claims the LPF path "falls back to a no-op when audio is
shorter than the FT4 frame". That guard is in `subtract_tones`; the function
actually calls `subtract_tones_lpf`, which has no such guard. Said in the PR
body as a follow-up rather than bundled.


## Pull requests to this fork: clean and mergeable as opened (the maintainer's standing order, 2026-10-08)

Kevin (kevin2008-01) opens pull requests here from Claude sessions. The
maintainer's words: make each PR **clean and mergeable**, because that is what
saves his compile time. So a PR is opened only when it can be merged as it
stands, without him building, fixing or re-running anything:

1. **Up to date with `main`, no conflict.** Bring `main` in with a **merge
   commit** — that is fine, and there is no linear-history requirement: no
   rebase, no force-push. He merges with a merge commit too.
2. **It compiles, silently.** `cargo check --workspace --all-targets` with no
   warnings, and `cargo check --release --target wasm32-unknown-unknown -p
   sdroxide-ui` when any browser-reachable code is touched.
3. **The touched crates' tests pass**, and a fix carries a test **verified to
   fail on the unfixed code**.
4. **`rustfmt --check` is clean** on every touched file.
5. **One complete thing per PR** — not one commit. Work in as many commits as
   you like and tidy them before opening.
6. **If a check cannot be run, do not open the PR.** Say what blocked it and
   what is needed. (#22 went up as a draft with its engine test unrun; that is
   the case this rule exists to stop.) A sandbox build of the radio engine needs
   both `github.com/xiph/opus` and `media.xiph.org` allowed — see the build note.

**The split, agreed with the maintainer:** Kevin's PRs take the **browser / web
server** side, where he is the heaviest user; the maintainer takes the **radio**
side. A change that reaches into the radio engine is his unless he asks.

**Auto-approve and merge Kevin's PRs (the operator's standing order,
2026-10-09).** Kevin's PRs (kevin2008-01, opened from Claude sessions) are
**approved and merged without waiting** when they are **sensible** and **merge
cleanly**. Do not leave a good one sitting for the maintainer to press the
button on.

- **Sensible** = it does what its body says, touches what it claims, and carries
  no red flag that its own body has not already declared. A declared wire change,
  a new dependency or a reach into the radio engine is not disqualifying by
  itself — an *undeclared* one is.
- **Merges cleanly** = `MERGEABLE` and `mergeStateStatus` nothing but the
  approval gate (`gh pr view <n> --json mergeable,mergeStateStatus`). A conflict
  is not clean; say so on the PR rather than fixing it for him.
- Merge with a **merge commit** (`gh pr merge <n> --merge --admin`). `--admin`
  is needed only because branch protection wants an approval the maintainer's
  agent is standing in for.
- **Not sensible, or not clean?** Leave it and say why on the PR. The point is
  to stop good work idling, not to lower the bar.

## The settings trace — a tool, and why it exists (2026-10-16)

**`SDROXIDE_SETTINGS_TRACE=1 sdroxide`** draws a ruled scale down the settings
page's left margin, numbered every 25 points, and every skinned widget labels
itself with its own y (`chk`, `sld`, `rdo`). Two rows drawn on top of each other
then read as **two identical numbers** in a screenshot.

It exists because **four settings pages were reported with rows overlapping and
no render here reproduces it** — three widths, two display scales, measuring ink
as well as geometry, and three separate theories each died by measurement. So the
next report of this class starts from a number rather than another guess. Off
unless the variable is set, read once at startup.

**The harness trap it exposed, which is the part worth keeping.** My first
"no overlaps" readings meant nothing, three times over: the Settings probe draws
a **stub** unless it has a `radio_cfg` **and** `caps` **and** a controller that
returns a config — the window re-reads `radio_cfg` from the controller when it
opens, which overwrote what I had set. A probe that cannot reach the code under
test reports success. It is the same shape as the sound-device bug the same day.

**And a colour guard caught a theme draft**, which is worth knowing about:
`every_dark_theme_keeps_its_accents_readable` refuses any accent under 4.5:1 on
its panel, because these colours are used as *text*. SDRuno's own dim cyan is
about 2.7:1 — fine for a lamp, not for a caption.

## House rules

- **Release pages are checked, not requested.** `release.yml` fails a release
  whose notes run past **60 lines**, because the short-notes rule was written down,
  read, and broken anyway — the page carried a **185-line preamble** the workflow
  pasted above every changelog entry, so every release was long no matter how short
  its entry was, and the operator had to ask for the same thing week after week.
  The preamble is a few lines now and the guard is what keeps it there. **When a
  prose rule has been given more than twice, make it a check.**
- **Say it short. Always — release notes, front page, and anything a user reads.**
  The operator's standing order, and it has been broken three times by an agent
  that read the rule further up this file and wrote prose anyway. So it is here,
  where every session reads it, and it is not negotiable:
  - **Release notes: one line per change, then stop.** `### Fixed` / `### Added`
    / `### Changed` / `### Needs feedback`, a bold lead and a single clause after it.
    No second sentence, no "why it happened" paragraph, no sub-bullets. The notes
    sit **above the download assets** — a wall of prose buries the thing the
    visitor came for. The *why* lives in the commit message, which is where a
    reader who wants it will look.
  - **The README front page is an elevator ride.** A short paragraph that says
    what this is, why *this* one, and who it is for — then stop. Not a feature
    tour, not a history, not a table of contents; the manual is the table of
    contents.
  - **The hierarchy the operator uses for the pitch itself, and for anything
    else worth selling**: a **long story** for when there is time for a
    presentation, a **paragraph** lifted from it that still carries the core for
    an elevator ride, and a **one-line sentence** taken from that paragraph for
    when there is only a line. Write the long story first and cut down, never the
    other way — which is how prose gets written when the short form is asked for
    directly.
- **Write comments and replies the way a person would — the operator asked for
  this twice, and the second time was in a meeting with Kevin (2026-10-14), where
  both of them said they liked having an agent do the work.** So it is here rather
  than in a thread: **answer to the point, do not over-explain, and go easy on
  headings and bold.** A reply to a user is a message from a person, not a
  document — no `##`, no bulleted essays, no `**bold**` on every other phrase.
  Polite, personal, and short. This is the same failure as the release notes: the
  rule was written down somewhere and still got broken, so it lives where a
  session starts. Markdown is for the *repository* — commit messages, the manual,
  this file. A reply in an issue, a discussion or a chat is prose.
- **Ask a reporter one thing, not three.** The operator, 2026-10-17: *"don't
  overask Kevin please.. you have a tendency to keep asking like you did now"* —
  after a reply that carried three questions, one of which (whether the sound went
  to a Bluetooth device) he had already ruled out. **A report is usually enough to
  work from**, and the questions are the cheapest part to cut: ask the one that
  decides the fix, and leave the others until that one is answered. Same failure
  as the release notes — a rule about being short, broken by adding.
- **No paid service, no API key, no account. Ever.** The operator, 2026-10-17,
  on map tiles: *"that goes against everything this software stands for."* This
  program is given away free and has to work with the network unplugged, so the
  answer to "where do the data come from" is always **a dataset we may ship** —
  public domain or a free licence, committed as an asset like `assets/earth/`,
  regenerated by a committed script, fetched at build or run time by nobody.
  The line was drawn over map tiles (Google and Mapbox want a key and a card;
  OpenStreetMap's own tileserver asks apps not to hammer it) and it holds for
  everything else: no weather API, no callsign service, no geocoder. Where a
  paid or keyed source is the only one, **the feature does not ship**, and that
  is the answer rather than a caveat.
  **And the program must work offline**, which is the same principle seen from
  the other side: a "hot" online resource that a feature *needs* is a failure
  even when it is free. Everything that matters — receive, transmit, the
  decoders, the maps, the log — is local, and the things that do reach out
  (spots, TLEs, space weather, the schedule, callbook, log uploads, the update
  check) are extras that are off until asked for and degrade quietly. New work
  that only works connected is in the wrong fork. The user-facing statement is
  the manual's **Offline by default** table under §2.1 and the README's Running
  section; keep both true when adding a feature that fetches anything.
- **A contributor's PR that is sensible and merges cleanly is approved and
  merged without waiting.** In particular **Kevin's PRs (kevin2008-01, from
  Claude sessions) are auto-merged** — see "Pull requests to this fork" for what
  "sensible" and "merges cleanly" mean and the merge command. Do not leave a good
  one sitting.
- **A measurement that disagrees with a report is the first suspect, not the
  last.** Before writing a fix for a reported fault, put it through four questions
  in this order, and stop at the first "no":
  1. **Is it visible?** The reporter's words, taken literally. "The outline
     extends beyond the edge" is a paint past the window; "the layout is wrong" is
     a guess about cause. Ask for the screenshot rather than inferring a mechanism.
  2. **Does the instrument measure the thing reported?** An oracle that counts
     geometry counts ink nobody can see — a transparent rect overhung at every size
     and was reported as a border that failed to close. Transparent is not drawn;
     ask what is drawn.
  3. **Does anything already in the tree have a number, and does it disagree?** The
     thing that knows the arithmetic is the instrument; the thing that merely looks
     wrong is the symptom. `angled_frame` knew the width each panel was offered
     *and* the rectangle it took, and the question had been "which row is too wide"
     when the answer was "nothing inside is too wide — the background is".
  4. **Is something else fighting the fix?** A fix that is correct and changes
     nothing means a *second* cause is in play, and the way to find it is the
     one-click experiment that isolates the other combatant, not a third
     implementation attempt.
- **A passing test is evidence about the test, not about the program.** Every
  version of this has shipped green: nine tests on a QSL card while three typed
  fields were missing from the rendered image; a sweep whose 6 pt tolerance existed
  only to clear the fault it was supposed to catch; a bisect whose first two steps
  were clean and whose third would have been too. When a fault survives a test,
  **the test and the tolerance are suspects before the code is.** And a tolerance
  that was added to accommodate a known fault is not a passing test — it is the
  fault, written down, and it should be the next thing removed.
- **For anything drawn rather than computed, render it and look at it.** Numbers
  prove where the ink is; only looking proves it is *there*. A test can assert a
  buffer's size, a colour, a width and still pass on a panel with nothing in it.
- Keep changes CB- and listener-first: when a choice is between a ham workflow
  and a CB or listening one, this fork takes the CB/listening one. CB is a
  two-way service, so "CB-first" means using the band in full — transmit and
  the digital modes included — not a receive-only reading of it.
- **Assume a beginner, and never leave them guessing why nothing happened.**
  The fork's listeners include people who will not know what an option does or
  why a number looks wrong, and the freedom to explore is the point — so the
  target is not to *remove* options but to make each one impossible to
  misread. A control that can silently do nothing, or a displayed value that
  disagrees with what the operator just chose, is a bug in this fork even when
  the underlying behaviour is correct. Make the state say itself: name *which*
  thing is off (`DECODING` / `DECODING OFF`, not `RUNNING` / `OFF`), explain a
  deliberate offset where the two numbers are (`carrier 4610.0 · dial 4608.1
  kHz (USB −1.9k)`), and put the fix next to the symptom (the amber "press
  LISTEN above" line). The worked examples, all from issue reports, are the
  HFDL off-state and dial-vs-channel fixes and the WEFAX carrier note (all
  2026-09-21/22); the general form of the last is scoped in `ROADMAP.md` under
  Phase 4. "Simple UI" *hides* advanced chips and SWL mode *hides transmit*;
  neither is a substitute for this — error-proofing is what lets a beginner
  explore in either.
- Do not touch the vendored subtrees (`vendor/`) except to update a submodule.
- Native-only crates (`-drm`, `-nrsc5`, `-faad2`, the USB drivers, …) must never
  become dependencies of a wasm-targeted crate.
- Search with `rg -n`, never `rg -rn`: `-r` is ripgrep's replace flag and
  rewrites what it prints.
- **Formatting: no repo-wide `cargo fmt`, but the tree is nearly clean, and the
  reason is smaller than it used to be.** Measured 2026-09-29: of 982 tracked
  `.rs` files, **26 are not rustfmt-clean, 50 hunks between them** — all 26
  `fork-modified`, i.e. files upstream also edits. Every *fork-only* file is
  clean; they were swept that day (13 files, 42 hunks, one style-only commit)
  precisely because they can never conflict with an upstream merge.
  `vendor/` (7 files, 58 hunks) is never touched. So the practical rule is
  **format what you touch, and leave the 26 alone**: upstream keeps editing them
  unformatted, so a hunk we fix now is a hunk that can conflict at the next
  merge. In a merge, take his formatting where he reformatted a line and left
  the surrounding house style alone — which is what the 2026-09-28 merge did for
  `audio`, `qso` and the `digi_has_log` ordering.
- **`rustfmt.toml` sets `use_small_heuristics = "Max"`, and it is only found
  when the file is inside the repo.** This cost a real wrong answer once:
  running `rustfmt` on a scratch copy in `/tmp` silently falls back to stock
  defaults and reports a file that is actually clean as having ~50 diffs, and
  the `‑w`/direction of `--check`'s `-`/`+` is then read backwards. Measure
  in place, or pass `--config-path`. To re-measure the whole tree, run
  `rustfmt --edition 2024 <file>` on each tracked file **with the path still
  under the repo root** (a scratch file in `target/` works) and count the
  differing opcodes. A one-liner is what `use_small_heuristics = "Max"` keeps
  a short `if`/`else`, struct literal or call argument on; the vertical chains
  and the split labels the old note complained about are what it *rejects*.

- After resolving a merge, `git add` every edit the resolution produced and
  compile the **committed** tree, not just the working one. A merge went up
  non-compiling because three resolution edits were left unstaged while the
  local build, which saw them, was green.
- Commit messages: a short imperative subject, then the why. Say what was *not*
  tested when it could not be tested here.
- Dependabot's two tract advisories (`tract-onnx`, `tract-nnef`, both reached
  through `deep_filter`) are dismissed as **not used**: only the model embedded
  in the binary is ever parsed, never an operator- or network-supplied one.
  There is no `cargo-audit`/`cargo-deny` config; if one is added, those two
  GHSA ids go in its ignore list with that note, or the same reasoning goes
  upstream where the dependency is shared.
- **`rustls` 0.23.43 → 0.23.45 (`0ebd2082`, 2026-10-06), and it was a real one.**
  `GHSA-2mjx-qc3c-rqvc` — TLS 1.3 handshake messages accepted across encryption
  level boundaries, vulnerable `>= 0.23.13, < 0.23.45`. Reached through
  **ewebsock** (the `wss://` remote client) and **ureq** via `sdroxide-config`,
  so it was a protocol-boundary bug on a link we do not control, not a
  not-used dismissal like the tract pair. **The lesson is the one worth
  keeping:** the alert showed up in the *push output*, which is the only place
  it is ever visible — nothing in the tree mentions it, and all three of these
  had gone unread. `gh api repos/<owner>/<repo>/dependabot/alerts` lists them; a
  lockfile-only bump is the fix, and the watcher is not installed.

## The screen-settings store: the cause is found, and it is FIXED (2026-10-02, last)

**It is one thing, and it only shows on a server with a password.** Kevin
(discussion #4, Roy / F6KIM) compared the two files and found the tell:
`clientsettings.json` had a populated **`default`** block with his values and
**`profiles: {}` completely empty** — no named profile was ever written.

**I first diagnosed this as a race and was wrong.** The claim was that the
server decided the profile at connect before `Auth` had been processed, so
`login` was empty and `default` was offered. `handshake` completes
`auth::challenge` — which captures the username — *before* `run_session` starts
(`session.rs:62`→`:73`), and with credentials the order is hello →
`AuthRequired` → `Auth` → `HelloAck`, so the session and every offer start with
`login` already known. **Do not repeat the race story.** It was posted to Kevin
and retracted there.

**The actual cause.** `ClientSettingsStore::for_profile`
(`sdroxide-config/src/lib.rs:1789`) *falls back* to the station `default` when
the named profile is empty, **and reports the fallback** — `(None, settings)`:

1. Connect as `1-sebastien`: `for_profile(Some("1-sebastien"))` finds nothing,
   falls back to `default`, and tells the client `profile: None` — *"you are on
   `default`"*.
2. Save: the client sends back the only profile it knows (`frame.rs:2257`) —
   `None` — and `store.set(None, …)` writes the **shared default**.

**That loop cannot terminate**: a profile is only created by a save, and a save
can only ever reach `default`. Hence `profiles` stayed empty. The severity is
that on a password server the operator's screen was stored as the **station's
shared** look, so the next client to sign in inherited it. Invisible without a
password, because there `default` is the right bucket.

**The fix, on both the screen and the bindings write.** The server keys the
store on the **authenticated identity** and ignores the profile in the message
(`profile: _`): `let key = (!login.is_empty()).then(|| login.to_string())`. One
line each in `session.rs`. This also closes the hardening point — a client could
previously write *another* profile's settings by naming it. **No wire change**:
the variant is untouched and the reply already reported the login-derived
profile, so the client now learns its own name from the save's echo.

**Pinned by `a_signed_in_clients_screen_lands_in_its_own_profile`**
(`sdroxide-server/tests/session.rs`), which **fails on the old code** with
`left: None, right: Some("f6kim")` and passes after. It drives a real WebSocket
with real credentials, deliberately sends the `None` a real client sends, and
asserts the settings come back under the signed-in name. Two traps in writing
it: `spawn_server(port, Some(credentials))` is what makes `login` non-empty (no
credentials ⇒ no profile is ever in play), and a read loop must be **bounded** —
the server streams state continuously, so `loop { … _ => continue }` never
reaches `recv_msg`'s own 15 s timeout and hangs instead of failing.

**Not verified by a human.** Server-side and test-proven, but no browser was
driven. Asked Kevin to save, restart both ends, and confirm `profiles` now holds
his name.

Also acknowledged, and **not** part of this fix: an existing profile cannot be
**edited** — you must create a new one to save a change. Kevin is right that it
is wrong; do not bundle it silently. Also open: whether the client should adopt
`client_settings_from` from the reply's `from` (harmless now the server is
authoritative, but the client still believes it is on `default`).


## Classic UI — the SDR++-style left column (2026-10-17)

**What the name means, because it will come up.** *Classic UI* is the arrangement
where every control sits in one column down the **left**, each group folding
away, and the picture takes the rest — SDR++'s and SDR++Brown's layout. The
operator's brief: *"most SWL use SDR++ (SDRPP) or SDRPP BROWN. They are very
familiar with that layout… all controls on the left (collapsable bar)"*, and he
named it. It is `UiSettings::classic_ui`, off by default, on Settings → UI beside
**Layout**, local `config.toml` and **not** `ClientScreen`.

**Why it is on `main` rather than parked**: it answers the complaint this fork
keeps hearing. Both r/ShortwavePlus readers who called the program *"too
complicated, controls all over the place"* are SDR++ users — SDR++Brown's own
docs say *"In Brown, expand the left **Source** panel"* — so the fix is the
arrangement they already know, not fewer features.

**The manner is SDR++'s; the controls are ours — and the first version got this
wrong in a way that is worth keeping.** That version drew the *strip's modules*
inside the column (`vfo_rit_module`, `rx_filter_module`, `tx_condensed` and the
rest) on the rule that a control should have one implementation. The rule is right
for the docked band column and the detached windows, which *are* the same widgets
in another place; it is wrong here, because those modules are boxes built for a
strip — each with its own margins and a *width*, chosen to make a row of boxes come
out even. In a resizable column they fight it: the panel's width and the content's
width disagree, so a drag springs back and the operator sees a fixed-width box that
does not belong to the column. His words, and they are the specification: *"it's a
complete new section… I would replace it with a complete new container containing
all controls nicely categorized, adaptive and hide the original boxes"*.

So every row takes the width it is given. A row is a caption in a left gutter and
controls filling the rest (`classic_row`), a chip run spreads and wraps
(`classic_chips`), and a rail is fitted to what is left of its line
(`chrome::slider_readout`). **Nothing has a width of its own**, which is what makes
the panel's edge draggable at all. Eight sections, SDR++'s order: Source, Band &
mode, Receiver, Tuning, Transmit, Recorder, Display, Windows & tools — the fold
order SDR++ uses top to bottom.

**Behaviour still lives in one place, and that is the half of the rule that
stands.** Where a chip carries a rule — the mode chips' band-grey, BIN's
sub-receiver gate, the transmit refusals, the passband presets, the
noise-reduction engines — the column calls the same function the strip and the RX
menu call: `band_menu_body`, `rx_chip`, `filter_controls`, `nr_controls`,
`rec_controls`, `system_chips_top`/`_bottom`, `display_view_chips`/
`display_tool_chips`, `vfo_util_chips`, `vfo_offset_row`, `tx_*`. So there is one
implementation of every *control* and two layouts. When the column is in use the
strip draws **only the readout and the S-meter** (`desktop_strip`'s early
branch) — the two things that belong beside the picture.

**One decision, asked once.** `classic_surface(tier, width, collapsed)` returns
`None` / `Rail` / `Column(min, max)`, and both the frame (which draws it) and the
strip (which folds its boxes only where a surface is actually drawn) ask it. Two
copies of that decision is how the pair drifts, and it did: the strip folded while
the column was left out for want of room (592 pt = 232 for the column + 360 for
the picture), leaving the bands, the VFO and the tools in neither place.

**The handle works, and the width has to be a *number*.** egui's note on
`resizable` — the ui must use the space it was given — is necessary and **not
sufficient**, and the first attempt at it was wrong in a way worth keeping: the
column's scroll area was made `ScrollArea::both` to stop a wide row dragging the
panel out, and **a horizontal scroll area makes `available_width` infinite**, so
every row stretched to infinity and was clipped at the viewport. The operator saw
it before I did: *"the contents of the bar doesn't wrap"*, with all seven rows
ending at exactly the same pixel (356 of a 366 pt column). Vertical only, and the
column's own contents are pinned to the width they are handed. The ceiling is 640,
not 420: a drag that stops reads as a drag that does not work, and his first report
was exactly that.

**The chip that was 33 points outside the column: `add_enabled_ui` stops a row
wrapping.** This file already said so where the mode grid needs it
(`chip_enabled_tinted` "rather than a chip inside `add_enabled_ui`: the latter wraps
every chip in a child scope, which stops the row it is in from wrapping") — and
four chips in the RX body still did it: **BIN, MONO, the auto-record hold and I/Q
WAV**. BIN is the one that measured, 33 points past the edge of a 376 pt column,
and it took a bisect by section, a shape-kind dump (the crossing was a `Shape::Mesh`
— a gradient box, not the chip I was looking for) and a word dump to name it.
`chip_enabled` / `chip_accent_enabled` reserve the width first and grey the chip
inside that reservation, so the wrapping row can see it. **Grep `add_enabled_ui`
next to `chip(` before drawing a chip in a wrapping row.**

**The fold lives on the column, and folding leaves a rail.** `◀` in the column's
heading folds it away; the rail keeps **22 points** and carries the `▶` that brings
it back, so the fold and the unfold are the same control in the same place. The
rail was the operator's idea and it is the better one (*"leave 15 pixels width so
the unfold button stays visible"*): it is what lets the fold live on the column at
all, since a column that vanishes leaves no way back. Three things about it are
load-bearing. It has **its own panel id** — a rail that wrote 22 points into the
column's stored width would bring the column back 22 points wide — and
`unchecking_classic_ui_gives_the_strip_back` asserts the column comes back at its
own width, not the rail's. **Nothing of any of it is drawn when Classic UI is
off**: the strip is simply the strip, which is what the odd chip in the middle of
the readout was there to work around. And the rail is drawn wherever the tier is
not a phone, even on a window with no room for the column: it is 22 points, so
there is always room for the way back.

**And the mode was in the column twice** — its own `Mode` fold above the `Bands`
one, which is `band_menu_body` and already carries the mode matrix
(*"mode is in the sidebar and in the bands menu in the sidebar"*). The `Mode`
section is gone; the band panel is the one panel with the modes in it, exactly as
it is in the docked column and the detached band window.

**The guard is `the_column_fits_the_width_it_is_given`**: the column is drawn at
each width it may be dragged to and every shape's ink is measured against the
width it was handed. **It sweeps every 24 points rather than sampling** — the
defect it was written for lived in a band about thirty points wide (465 to 495)
that three spot widths stepped straight over, and a second one hid at 400. It
sweeps the *new* container now, which is the point of it: it is what would have
caught the module-reuse mistake in the first place. Beside it,
`the_surface_is_the_columns_own_room` pins where a surface is drawn at all, and
`unchecking_classic_ui_gives_the_strip_back` drives the frames the operator's
reports were about — the boxes leave with the setting, the picture's left edge
returns, the rail is a rail, and every section is drawn (on a window tall enough
to hold them, because in a 900-point window the later ones are below the fold).

**Two traps around the setting itself, both found by using it.** The strip's
layout override folds the *strip* into fewer rows, and the classic strip has no
rows to fold — left to the override, a `Small` setting would draw the compact
strip's chips beside a column carrying the same controls, so Classic UI is drawn
by the desktop strip at every tier but the phone. And the classic strip is mostly
bare ground, where the strip's **double-click layout switch** was registered and
*persisted*: an invisible target that changes the arrangement for good. Neither is
reachable now, and `unchecking_classic_ui_gives_the_strip_back` drives the frames
the operator's report was about — the boxes leave with the setting, the fold chip
is drawn only with the column, and the picture's left edge goes back to where it
was (8 pt, against 280 with the column).

**The column is a panel, and it is drawn like one.** `chrome::angled_frame` gives
it the same cut-corner outline and fill the waterfall and the operating panel
have — the operator's *"I just miss the border around it"*, and the right reading
of it: without the border the column is a region of the window rather than a panel
in it. The panel itself is only space now (a 4-point margin, no fill); the frame
is the look. The folded rail gets a line on its right edge for the same reason.

**The readout is centred over the picture.** It sat at the left edge, above a
column it has nothing to do with. The S-meter is pinned right first, then the
readout is centred in what is left of the row — and since the column has already
taken its width off that row, "centred" is the middle of the waterfall. (This is
the operator's one *visual* preference in the whole arrangement: *"I would love to
have the frequency centered so it's above the waterfall"*.)

**The sections are the operator's own order, dragged into place.** Wanted first
for the Recorder, and wanted as a category order nobody else can guess. Every
heading is a bar in an accent wash (`header_fill` — the 0.14 the notice bars use)
so where a section starts is visible without reading; the heading is the drag
handle, and dropping it on another section swaps them. The order is
`UiSettings::classic_sections`, a comma-separated list of section keys read
through `ClassicSection::order`. **It is a `String` on purpose**: a list of enum
names would make `config.toml` unparseable the day a section is added or removed,
and `Settings::load` answers an unparseable config by **quarantining the whole
file and returning defaults** — an arrangement would then cost a theme, a font
size and a layout. Read as text, an unknown name costs the arrangement and
nothing else: `order` keeps what he placed, drops what this build does not know,
and appends anything missing in the shipped order.
`a_section_order_survives_what_it_does_not_know` pins all four properties, and
`unchecking_classic_ui_gives_the_strip_back` asserts the *drawn* column follows
the stored order rather than the shipped one.

`UiSettings` is **no longer `Copy`** (it carries a list now). That cost one
`clone()` in `sdroxide-config`, a few `&self` on its accessors, and nothing else
— worth knowing, because "it is `Copy`" is the reason a `Vec` field looks
impossible at first.

**Three egui defaults that cost this column a round each**, all found on the
operator's screen and all worth knowing before the next arrangement:

- **A section's applicability is asked *before* its heading.** `Transmit` was
  gated inside its body, so SWL mode drew the heading with nothing under it — *"in
  SWL transmit has nothing to do in that menu"*. A control that does not apply
  takes its heading with it, and the question is asked of the app once so the
  column and the strip's own transmit box cannot disagree.
- **`Frame`'s content layout defaults to `top_down(Center)`**, and **`add_sized`
  centres what it is given**. Together they put every caption in the middle of its
  own gutter and every section name in the middle of its bar, and made a row
  wider than the column hang out of *both* edges at once — which reads as missing
  padding and is neither. `classic_column` pins `top_down(Align::Min)` and
  captions go through `gutter_label`; `the_column_fits_the_width_it_is_given`
  now asserts a heading and a control are in the left gutter, and that nothing
  hangs out of the left edge either.
- **`RichText::strong()` is a colour, not a weight.** A heading that says
  `strong()` is exactly as thin as the text beside it, which is why the operator
  asked twice for bold; the section headings name the family the theme installs
  for `TextStyle::Heading` (`chakra-bold`) explicitly. And **a heading bar should
  be the fold's control**: a click on it opens or closes the section (the fold's
  own arrow is a small target on a bar the width of the column), which egui tells
  apart from a drag by the distance travelled, so the same widget still moves the
  section. The click has to be applied to the fold *before* its body is drawn or
  the section answers a frame late.
- **egui clips a too-long *right-aligned* text at its left.** The Source rows
  right-aligned their values, so at 250 points `SDRplay RSP1 (serial 0000000001)
  @ 3.000 Msps` read `RSP1 (serial 0000000001) @ 3.000 Msps` — the wrong end
  lost. Long values wrap under their caption and elide at the end now, with the
  whole value on the hover.

**A bar that is not a strip has no boxes.** `chrome::module_plain_h` is
`module_bare_h` without its fill and border, and `smeter::show` takes whether to
draw its own outline — the Classic UI bar's readout and meter use them. The
operator asked for the accent background and the border on the frequency block to
go and the meter's border with them: a boxed module in the middle of an open bar
reads as a leftover from the arrangement it replaced. The **scroll bar floats**
for the same reason — one that allocates takes its width off every row it appears
beside, which he saw as *"it takes a lot of realestate and the padding is non
existent with the chips"*: one complaint, not two, and the fix is one setting
(`spacing.scroll.floating`) plus a content margin.

**The shipped order leads with Windows & tools, then Receiver** — the operator's
(*"default the Windows and Tools to the top and the Radio 2nd or 3rd"*), open by
default with the band panel. It is `ClassicSection::ALL`: what a *fresh* column
opens in, and the order anything missing from a stored list is appended in. **A
list he has arranged himself is never re-ordered by it** — which also means a
change to the default reaches an operator who already has an arrangement only if
they clear `classic_sections` from `config.toml`.

**It opens at 300 points**, and changing that needed the panel's id bumped:
egui remembers a panel's width, so a new default is hidden behind whatever the
last drag left there. The **section order** is not egui memory — it is
`UiSettings::classic_sections` in `config.toml`, written on every drop, so it
survives a restart.

**Two more egui facts this column paid for**, both worth knowing before the next
one is built:

- **`Ui::dnd_drop_zone` overrides the frame it is given** — its own doc says "the
  color is ignored" — so a `Frame::new()` around a drop zone does not mean "no
  box": every section came out filled and bordered (*"the sections have their own
  border too, ditch them, they look warped in the layout"*). A drop target has to
  be drawn by hand: the section's rect is kept in egui's temp store,
  `DragAndDrop::has_payload_of_type` plus `pointer_interact_pos` decide whether it
  is under the cursor, the heading bar brightens, and `DragAndDrop::take_payload`
  on release is the drop. The rect is the *previous* frame's — a section can only
  be known to be under the pointer once it has been drawn, and one frame behind on
  a drag is invisible.
- **The order panels are shown in decides what "centred" means.** A panel takes
  its space off what is left, so the strip has to be shown *after* the classic
  column for the bar to span the picture rather than the window — which is the
  only way "centre the readout" can mean "over the waterfall". Shown after the
  strip, the readout is centred in the window and looks right in a screenshot,
  which is why this took two attempts: *"can you make the frequency center above
  the waterfall?"* The column also takes the window's full height, so its heading
  is level with the bar beside it.

**The Settings menu's page names, and the one rule that fixes them** — reported
three times (*"the accent in the settings menu column is wrong, active text is
unreadable"*, then *"the active chip is unreadable in every theme"*), and there
was **already a comment in that code from an earlier attempt at the same fault**,
which is the tell: it is not one colour, it is the *resolution*.

**`RichText::strong()` is not a weight.** It selects
`Visuals::strong_text_color()`, which is `widgets.active.fg_stroke.color` — and
the accent is what every palette here sets that to. So a `strong()` page name is
drawn in the accent, and the *current* page is that accent on a selection fill of
the same accent. The rule that ends the class: **name both colours explicitly**.
egui checks an explicit colour before `strong`, before the widget's state and
before the theme (`WidgetText::get_text_color`), so the name cannot come out as
its own fill whatever a theme or a state does; the weight comes from the bold
*face* (`chakra-bold`) instead. Two tests hold what is measurable:
`every_palette_has_an_ink_for_its_selection` (no palette fills its selection with
the colour it writes on it) and `an_explicit_label_colour_is_not_overridden`.

**Three causes were measured and all three were wrong** before that, recorded so
nobody repeats the search: egui's selection ink (a scratch test draws the selected
label in `ink_on_cyan`, readable, under every chrome style), `theme::scroll_palette`
(**the plausible one, and the trap to warn about** — it sets
`widgets.*.fg_stroke.color`, which egui uses for the scroll *handle* and is also
every widget's text ink, but `ThemedScroll::show_themed` **restores the normal
visuals before the content draws**, so it cannot reach a label; do not "fix" it
there, it is the scrollbar's colour), and the theme's `visuals` in general.
**The invisible-name half was never reproduced headlessly** — in every theme and
chrome style, with and without `strong()`, those labels draw — so the detached
Settings window (a child viewport the harness does not draw) is where to look if
it is still missing.

**Left to do**: a skinned section header rather than egui's default, SDR++'s
vertical Zoom/Max/Min rails beside the waterfall, and splitting the Receiver
section — it currently carries the passband presets and the noise-reduction
engines as sub-bodies, which are their own subjects and want their own folds. It has been looked at on the operator's own screen
across four screenshots (2026-10-17) — which is how the missing sections, the
stuck handle, the unwrapped rows, the chip past the edge, the misplaced fold and
the doubled mode were all found — and nothing has been rendered here: there is no
probe that draws the column to an image, so every reading is arithmetic and every
look is the operator's.

## WEFAX: the rephase rule was cutting pages inside their own phasing signal (2026-10-18)

Found while working fork issue **#91** ("WEEFAX PB", **F4NRM** — Kevin), which
reported a picture that stopped after 322 lines. The decoder is **unchanged
between 2.0.12 and 2.0.13** (diffed, decoder + controller + station table), so
the report had nothing to bisect; the way in was to point the decoder at *public*
recordings instead. `crates/sdroxide-dsp/examples/wefax_probe.rs` (committed)
decodes a WAV and prints the decoder's own account — start tone, phasing skew,
lines, how each page ended — writing **one PGM per page**, because the page
*count* is the diagnostic a screenshot cannot give.

**The fault, measured.** `Sigidwiki`'s *WFax_IntroNORTHWOOD* (a start tone, its
phasing signal, six seconds of chart) came out as **four** pages of 8–11 lines,
cut every seven seconds. A trace at both cut sites named the path: every cut was
`note_phasing_line`, every one of them *inside the phasing*. The lock is taken on
the transmission's **first** phasing pulse and the standard signal runs thirty
seconds after it, so the opening of every page is black lines with a white pulse —
exactly the shape the rephase rule looks for. `PHASING_SIGNAL_S` (30 s) now gates
it (`9bbc7a25`). The same recording is now **one page of 69 lines**.

**The gate is time, not content, and that was measured rather than assumed.** The
obvious test — wait until the strict shape test *rejects* a line (i.e. until chart
content has been drawn) — does not work: the strict test turns away some of the
phasing signal's **own** lines (the partial line the lock leaves at the head of the
page, and others a few seconds in), so the gate opens while the signal is still
running. The trace that showed it, on a fixture that is nothing but phasing:
`not-phasing line at y=0`, then `y=28…32`.

**Two details that cost a round each.** The run counter must keep being fed while
the guard is in force — `if auto_start && since >= GUARD && note_phasing_line(…)`
short-circuits, so the count is zero when the guard lifts and the cut then lands
eight lines late. And the two #276 fixtures modelled a page as ~7 s, *shorter than
the phasing signal a page has to outlive*; they now run past it, their subject
(finding the next transmission mid-picture, at an arbitrary line offset) unchanged.

**Guard:** `the_pages_own_phasing_signal_does_not_cut_the_page`, verified failing
without the guard with *"the page's own phasing signal ended it"*. Off-air gate:
`an_off_air_recording_decodes` (`SDROXIDE_WEFAX_SAMPLE`, `_LPM`, `_IOC`), the same
shape the other modes carry.

**Not proven, and the honest state of #91.** Nothing was received live here and the
recording is a public sample. **Kevin's picture is 322 lines and cleanly phased,
which is not what this fault produces** (fragments of 8–11 lines), so it is *not*
claimed as his bug — it is a real fault found on the way to it. He asked for no
reply until his next instruction ("Jarvis. Kevin (F4NRM) make correction. wait new
instruction."), so nothing was posted; the probe is what settles his report once a
recording exists.

### Kevin fixed the other half of the same symptom, in parallel (PR #101)

`b1add00c` — **"WEFAX: a start tone mid-picture no longer restarts the page"** —
came in *while this work was in flight*, was merged as **#101**, and is in
**v2.0.13.3 / v2.0.13.4**. It is the same symptom from the other path, and the two
are complementary rather than overlapping:

- **His:** a chart's own periodic detail (dotted graticule, hatching) alternates at
  the start tone's rate, so the **start-tone** path cut healthy pictures; a start
  tone mid-picture is now ignored, and the next transmission is found by its
  phasing signal — which makes the rephase rule the *only* mid-picture page-break.
  Plus `auto_begun` / `AUTO_PHASE_TIMEOUT_S` (20 s): an automatic start that is
  never phased yields no picture instead of one drawn on no phase at all.
- **Ours:** the **rephase** rule firing on the page's own phasing signal.

**Verified, and this is the reason both are needed:** with his change and without
this guard, the *same* public recording still comes out as **four pages of 8–11
lines**. His fix removes the start-tone path; nothing in it touches the rephase
one. The merge was clean, and his new test's fixture needed the same extension as
the two #276 ones (a page of ~7 s cannot outlive the phasing signal the guard
protects) — `d485e3c8`.

### WEFAX under a standing carrier, on a real recording (2026-10-20)

F4NRM's own off-air recording (48 kHz mono, 26 min) of a Pinneberg chart: **a
carrier at 2328 Hz on the channel the whole time** — 28 Hz above white, so it
reads as a blank page — fading under the signal and back over it. The decoder
never started on it, or started on chart detail and ended the page at 695 s.

**What the recording actually is, and it is not what it was described as.** One
chart, **phasing at 39.5 s to a clean stop tone at 1118–1122.7 s**, black after.
The "end at 13 min 34" was a stretch of the chart's own bottom frame; the long
"carrier-only" gaps inside it are fades, up to 28 s. The start tone is **not
recoverable** (it sits under the carrier) and there is **no audio tone at 300,
450 or 675 Hz anywhere**, so the audio-tone detector of 2026-10-18 cannot start
anything here. Nothing in the tree had a real recording with a carrier before.

**Four faults, each pinned by a piece of the recording** (`crates/sdroxide-dsp/tests/
wefax_real_recording.rs`, fixtures in `tests/testdata/wefax/`, under a megabyte):

1. **No start without a tone.** Idle now cuts lines at the line rate on a clock
   of its own and tests them for the phasing shape, round the circle so the
   starting point does not matter; three seconds with the pulse in one place
   begin a page (`idle_phasing`). The pulse in this recording is **a quarter of
   the line wide**, scattered by noise, and its edge wanders by a tenth of a line
   from line to line — so the lock is taken on a **stack of lines**
   (`PhasingStack`), where it stands out cleanly, with a vote that nearly every
   line carries it. The picture starts at the pulse's **centre plus half of the
   standard 5 %** — the falling edge for a standard pulse, as before, and within
   2 % of right for this one (the chart's frame is symmetric about it).
2. **Noise ended the page.** Discriminator noise crosses mid-grey 450 times a
   second, the stop tone's rate, for seven windows — and a count of crossings
   cannot tell it from the tone. A stop (and a start) alternation must now hold
   a share of the window's energy at its own pitch (`LevelSpectrum`,
   `PERIODIC_SHARE`); the real stop tone in the same file passes it. The first
   fixture for this was stored at 6 kHz and **did not reproduce the fault** —
   band-limiting the audio removes the noise crossings — so that one piece is
   kept at 12 kHz.
3. **A start tone announced a page on its own**, wiping the picture on screen for
   nothing. It arms now and is announced when the phasing locks, which is also
   where a *consistent* run is required (the old lock averaged any five lines
   that looked a little like phasing).
4. **Black and white were fixed at 1500/2300 Hz.** They are re-measured from the
   phasing signal (`calibrate`, within ±200 Hz) — checked by shifting the whole
   recording ±100 Hz and through a ±80 Hz drift: the same single 2153-line chart.

Also: the rest of the phasing signal at the head of an idle-started page is left
out of the picture; nothing begins a page for 15 s after one ends (a chart's own
closing bars look like phasing); a *flat* line of any brightness counts as blank,
so a carrier that has drifted below white still ends an automatic page; the
audio-tone detector judges purity against the low audio only, since a carrier
dominates the total.

**The replay needed no code, only proof.** Playback already stands the radio down,
plays the file through the audio path and feeds every decoder; a file with no
stamp in its name is played in whatever mode the operator picks. What was missing
was a test that a fax recording comes out as a picture while it plays and that the
live source is not read meanwhile: `crates/sdroxide-radio/tests/playback_wefax.rs`
(13 s, real-time paced). A file reaches it by being put in the recordings folder.
**No wire change, no `PROTO_VERSION` bump** (214 on `main`).

**Lessons worth keeping.** A description of a recording is a claim: the 814 s end
came from listening, the truth from a timeline of the carrier's share of the
spectrum. A mutation check found one of the new unit tests passing with its own
fix removed (noise at 900 symbols a second is 225 crossings per window, not 450) —
**remove the fix and watch it fail, for every test, every time.**
