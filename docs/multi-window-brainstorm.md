# Splitting radios into separate windows — brainstorm

> **Status: brainstorm, not a plan.** Written down so the idea and the reasons
> behind it survive; nothing here is committed to. If it ever becomes work, it
> starts with a real design note and a decision.

## The idea

Today the multi-radio shell shows several radios inside one window: the radio
strip across the top, and *panes* — equal columns, one per chosen radio, each
with its own copy of the strip. The question that prompted this: could each
radio instead live in its **own OS window**, so they can be put on different
monitors?

## What already exists

- **Panes.** The `⊞` toggle on a radio gives it a pane; the main area splits
  into equal columns. Two radios, one window spanning both monitors, is already
  two-radios-on-two-monitors.
- **Each tab already owns a complete `SdroxideApp`** — its own controller,
  view state and panels (`crates/sdroxide-ui/src/multi.rs`). The per-radio UI
  state is therefore *already* separated; a separate window is "only" a
  rendering question.
- **Multi-viewport is already in use.** The 3D solar view opens natively as a
  child viewport of the main window (`crates/sdroxide-ui/src/lib.rs`), so
  eframe/egui multi-window is enabled and known to work on at least this
  platform.

## What separate windows would add

- **Per-monitor memory.** Each radio reopens where it was left, at its own size
  and maximised state. A single spanned window cannot do this.
- **Unequal screens.** Radio A maximised on a big monitor, radio B windowed on a
  small one. Panes force equal columns inside one window.
- **Per-monitor DPI/scale.** A window spanning two monitors with different
  scaling has one scale factor; two windows each get theirs. This is the
  strongest technical argument for it.
- **Per-window OS behaviour.** Separate taskbar entries, alt-tab, always-on-top,
  snapping, and closing/iconifying one radio without the others.
- **Different layouts.** A wide panadapter on the large screen, controls on the
  small one, rather than two equal half-screens.

## What it would cost, and why it may not be worth it

- **Real work.** egui's deferred viewports render in closures that may run on
  other threads, while the panes are drawn today in one `&mut self` loop. The
  per-radio `SdroxideApp` would have to be taken out of the `Vec<Tab>` (or put
  behind a lock) to render in its own viewport. That restructuring is the bulk
  of it.
- **Shared station services.** The T/R switch, MIDI, the remote factory and the
  engine threads are station- or shell-level, and the wgpu render state is
  shared; all of that must stay coherent across windows.
- **Focus and shortcuts.** "Which radio has the keyboard" is currently the
  shell's per-pane bookkeeping. Across OS windows it becomes window-manager
  territory, and global shortcuts need a rule.
- **Window sprawl.** Many taskbar entries and windows are worse for some users
  than tabs; it is a preference, not a strict improvement.
- **Platforms and maintenance.** Multi-viewport behaviour differs between
  Wayland, Windows and macOS, and it is one more thing to keep working across
  eframe updates.
- **The panes already cover the common case at zero cost.** Same-size monitors
  and simple side-by-side are done.

## Sketch (if it were built)

- Keep `MultiApp` as the station and the `Vec<Tab>` of `SdroxideApp`s, but render
  each *pane* through `ctx.show_viewport_deferred`/an immediate viewport keyed
  by the radio id, instead of drawing the columns inline.
- Hoist the per-radio `SdroxideApp` into an `Option` that the viewport closure
  takes and returns, so the deferred closure never aliases the shell.
- Decide where the **radio strip** lives: a small main window (a "control room")
  with each radio as a child window, or a strip per window as the panes have
  now.
- Keep the station-level services on `MultiApp` and reach them from the
  viewports the way the 3D view already reaches shared state.

## Open questions

- Strip: one control window, or one per radio window?
- What happens to a radio whose window is closed — does the tab come back in a
  main window, or does the radio stop?
- Do global keyboard shortcuts follow the focused window, or stay on a main one?
- Is a maximised-pane-per-monitor using today's panes "good enough" for the
  people actually asking for two monitors?

## Decision

**Defer.** Use the panes for now. Revisit only if a concrete need shows up that
panes cannot meet — most likely different DPI/scaling across monitors, or
per-window always-on-top.
