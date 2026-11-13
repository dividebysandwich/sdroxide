//! Multi-radio shell: one window, one radio per tab — or several side by
//! side.
//!
//! Each tab owns a complete [`SdroxideApp`] — controller, view state, panels,
//! decoders' UI — so radios are isolated by construction rather than by a
//! field-by-field split of the app struct. The shell draws the radio strip,
//! hands the rest of the window to the radios on screen, and gives every
//! hidden tab a chance to drain its engine's events each frame: a background
//! radio keeps decoding, logging, recording and reconnecting; it just isn't
//! drawn.
//!
//! The screen itself is a row of *panes* ([`MultiApp::panes`]). One pane is
//! the ordinary window; each pane has its own row of tabs
//! ([`MultiApp::rows`]), and the columns chip at the end of a row opens a
//! further pane for a radio taken off its row, splitting the main area into
//! equal columns. "Radio ▼" puts a radio that is on no row onto a pane's
//! row. Exactly one visible radio holds the
//! keyboard/MIDI focus; each [`SdroxideApp`] gates its input handling on it,
//! and clicking a pane moves it there.
//!
//! What keeps the radios from stepping on each other lives mostly *below*
//! the UI: each engine has its own config scope (`sdroxide_config::Store`),
//! the strip's mute chip is the engine's own per-receiver mute (the same
//! [`sdroxide_types::Command::SetMute`] state the MUTE button shows, so the
//! two always agree and the engine remembers it), a shared `TxGate` keys one
//! transmitter at a time, and a shared `StoreSync` keeps the station-wide
//! stores (memories, band stacks, digi config) converged across engines. Up
//! here the shell only has to salt the persisted view state per tab (and,
//! through `layout::set_radio_salt`, the fixed egui ids) and gate the
//! announcer and the window title to the focused one.

use crate::i18n::tr;
use eframe::egui::{self, RichText};

use crate::app::{Clip, ClipAct, RadioChip, RadioTabRequest, SdroxideApp};
use sdroxide_types::RadioController;

/// One radio handed to [`MultiApp::new`] by the frontend.
pub struct RadioTab {
    pub id: u32,
    pub name: String,
    /// Whether this radio is switched on. A radio that is off arrives with no
    /// interface open behind it (the frontend gave it a stand-in) and its tab
    /// shows the switch in the off position.
    ///
    /// True for everything that is not one of this station's own radios: a
    /// connection to somebody else's station is switched on and off at that
    /// station, and there is no roster here that could say otherwise.
    pub enabled: bool,
    pub ctrl: Box<dyn RadioController>,
}

/// Builds the engine + controller for a radio created at runtime from the
/// "+" chip. Lives in the binary — only it knows how to open backends.
pub type RadioFactory = Box<dyn FnMut() -> Result<RadioTab, String>>;

/// Dials another station for the Remote settings tab: `(url, id, ctx)` in,
/// a connected tab out. Also in the binary, for the same reason — the
/// connection needs this machine's sound devices hung off it, and only the
/// frontend can open those. The context is what the socket wakes when a
/// message arrives.
pub type RemoteFactory = Box<dyn FnMut(&str, u32, &egui::Context) -> Result<RadioTab, String>>;

/// Ids for tabs that hold somebody else's station. The station's own radios
/// are numbered from the roster on disk, from 0 up; these are not in it — a
/// connection is not a radio this machine owns — so they are numbered from a
/// place the roster will never reach rather than allocated from it. The id
/// keys the tab's view state and its waterfall history, and a collision would
/// hand a connection the layout and the history of one of this machine's own
/// radios.
const REMOTE_TAB_ID_BASE: u32 = 1 << 31;

struct Tab {
    id: u32,
    name: String,
    app: SdroxideApp,
    /// The radio that has borrowed this one's receiver as its panadapter, if
    /// one has. Such a radio has no front end of its own — the borrower opened
    /// it — so it is kept out of the strip: a tab that can only ever say "my
    /// receiver is on that other tab" is a tab worth not having.
    ///
    /// It stays in the roster the settings dialog draws, which is how the
    /// operator reaches its interface settings and detaches it again. Refreshed
    /// each frame from the configuration its engine reports, so undoing the
    /// pairing brings the tab straight back.
    attached_to: Option<u32>,
    /// Whether this radio is switched on ([`RadioTab::enabled`]). The roster on
    /// disk is what the interface factory reads; this is the shell's copy of
    /// the same answer, kept because the strip draws it every frame.
    ///
    /// For one of this machine's own radios the switch below is the only thing
    /// that ever changes it. For a radio at a station the roster on disk is
    /// *that* machine's, and the switch may be thrown by another client or at
    /// the station itself — so this follows what the station announces, once a
    /// frame ([`MultiApp::refresh_power`]).
    enabled: bool,
    /// What to call this tab while nobody has named it and its radio has no
    /// interface to be named after: the address its connection was opened at,
    /// as the frontend's dialler writes it. `None` for one of this machine's
    /// own radios, which fall back to their number instead.
    ///
    /// Kept apart from `name` because it is not a name anybody gave: a radio
    /// added at a station has no interface yet, and putting the dialler's
    /// stand-in in `name` would make it the operator's name for the radio —
    /// still there, and now wrong, once they had chosen an interface for it.
    fallback: Option<String>,
    /// Somebody else's station, reached over the network. It has no entry in
    /// this machine's radio roster, so closing, renaming or switching it must
    /// not go looking for one — each of those goes to the station instead. In
    /// the browser there is no roster at all and every tab is one of these.
    remote: bool,
}

/// What the operator asked a strip to do, resolved after the frame — the
/// strips draw while the tabs are borrowed for display, so clicks are
/// collected as values first (the same arrangement as [`RadioTabRequest`]).
enum StripAction {
    /// A name-chip click: show this radio on this pane.
    Show {
        pane: usize,
        id: u32,
    },
    /// "Pick radio ▼": put this radio on this pane's row and show it here,
    /// taking it off the row it was on.
    Pick {
        pane: usize,
        id: u32,
    },
    /// The "−" on a tab: take this radio off this pane's row. It keeps
    /// running; off is the rings.
    Unpick {
        pane: usize,
        id: u32,
    },
    /// The "+" at the end of a row: open another pane.
    AddPane,
    Mute {
        id: u32,
        muted: bool,
    },
    /// The ON/OFF switch: open this radio's interface, or let it go.
    Power {
        id: u32,
        on: bool,
    },
    /// "New radio…" at the foot of the pick menu: the strip's former "+".
    Add,
    /// A click on the name of the radio a pane shows, where that radio is a
    /// public receiver: Public SDRs opens on the map, at that receiver.
    ShowOnMap(u32),
}

pub struct MultiApp {
    tabs: Vec<Tab>,
    focused: usize,
    /// The radios on screen, left to right, by tab id. One entry is the
    /// ordinary single-radio window; more than one splits the main area into
    /// that many equal columns. Kept duplicate-free — the same radio twice
    /// would be two views fighting over one engine's spectrum stream — never
    /// empty, and always holding the focused tab ([`Self::sanitize_panes`]).
    panes: Vec<u32>,
    /// The radios picked onto each pane's row of tabs, by tab id, one row per
    /// entry of [`Self::panes`] (`rows[k]` belongs to `panes[k]`). Drawn in
    /// the order of [`Self::tabs`], so the operator's arrangement (#224)
    /// holds within every row.
    ///
    /// A radio is on at most one row, and the one a pane shows is always on
    /// that pane's row. A radio on no row is not gone: it keeps running, as
    /// every radio behind the one on screen does, and "Pick radio ▼" brings it
    /// back. On is the rings, never where a radio is shown.
    // Why: HDZ 2026-11-04, Panel Exchange (#771) retracted: one radio per
    //      strip hid the multi-radio model ("die angeschalteten Geräte dudeln
    //      irgendwo rum, auch wenn sie nicht sichtbar sind") and left the link
    //      switch in two shapes. "Pick Radio, dann kann an einem Panel oben eine
    //      Reihe Radios gewählt werden … jedes Radio braucht dann einen Minus-
    //      Knopf … und das Plus am Ende macht wie bisher ein zweites Panel auf";
    //      on "−": "dann bleiben wir beim WT-Verhalten" (it keeps running).
    // Invariant: `rows.len() == panes.len()`; `panes[k]` is in `rows[k]`; no
    //      id on two rows ([`Self::sanitize_panes`]).
    rows: Vec<Vec<u32>>,
    /// Each pane's row of tabs as tall as it drew itself last frame, before
    /// any padding: what the split view evens the rows out to, so that the
    /// panes below them start on one line. Measured a frame late, which is
    /// what keeps a row that wraps from moving under the pointer.
    // Why: HDZ 2026-11-04 (two panes, five tabs wrapping to two lines on the
    //      left, one tab on the right): "könnte man wenigstens die roten
    //      Rahmen ausrichten aneinander".
    strip_heights: Vec<f32>,
    /// The arrangement last written to the roster, so it is written again
    /// only when the operator changes it.
    #[cfg(not(target_arch = "wasm32"))]
    saved_layout: Option<sdroxide_config::PaneLayout>,
    factory: Option<RadioFactory>,
    /// How a station somewhere else is dialled — Settings → Remote. Present
    /// in every native session, including one that is itself a remote client:
    /// a screen with no radio of its own is exactly the one most likely to be
    /// pointed at a server.
    remote: Option<RemoteFactory>,
    /// Kept for tabs created at runtime, which are built long after the
    /// [`eframe::CreationContext`] is gone.
    wgpu: Option<eframe::egui_wgpu::RenderState>,
    /// The station a radio has just been asked for, by station key, while the
    /// station is still creating it. What it does is put the radio in front of
    /// the operator when it arrives instead of quietly behind the tab they are
    /// on: they asked for it, and the next thing they want is its Radio
    /// settings page. Cleared as soon as one arrives — see
    /// [`MultiApp::open_peer_radios`].
    pending_add: Option<String>,
    /// A configuration to hand the radio `pending_add` is waiting for, from
    /// "Public SDRs". Separate from `pending_add` because the plain "+" has
    /// none and must still open the settings page.
    pending_preset: Option<Box<sdroxide_types::RadioConfig>>,
    /// Addresses of a station's further radios that have already been opened
    /// beside the one that was dialled. Kept for the whole session and never
    /// cleared on close: a tab the operator shut is one they did not want, and
    /// the offer arrives again on every reconnect. See
    /// [`MultiApp::open_peer_radios`].
    peers_opened: std::collections::HashSet<String>,
    /// The station radio: the first one the frontend booted, which is the first
    /// entry of `radios.json`.
    ///
    /// Held rather than read off `tabs[0]`, because the strip's order is the
    /// operator's to arrange (issue #224) and this radio's standing is not. It
    /// holds the station's shared services and the legacy configuration paths,
    /// takes the command line's overrides, and is the one radio that cannot be
    /// closed — none of which may move to another radio because a chip was
    /// dragged. `None` only where every tab is somebody else's station, which
    /// is every browser client.
    station_radio: Option<u32>,
    /// Whether the window has been checked against the screen it opened on.
    ///
    /// Once, on the first frame that knows both sizes. A window is only ever
    /// brought *in* — see [`crate::layout::fit_inner_size`] — and doing it
    /// every frame would fight an operator dragging their window bigger than
    /// the display on purpose.
    #[cfg(not(target_arch = "wasm32"))]
    fitted: bool,
    /// The clipboard (📋): what it holds. It stays filled across radio
    /// switches until a right click empties it or the last other radio closes.
    clip: Option<Clip>,
}

impl MultiApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        radios: Vec<RadioTab>,
        factory: Option<RadioFactory>,
        remote: Option<RemoteFactory>,
    ) -> Self {
        let shared_log = radios.len() > 1;
        let can_add = factory.is_some();
        let tabs: Vec<Tab> = radios
            .into_iter()
            .enumerate()
            .map(|(i, r)| {
                let remote = r.ctrl.engine_is_remote();
                let mut app = SdroxideApp::new_tab(
                    &cc.egui_ctx,
                    cc.storage,
                    cc.wgpu_render_state.clone(),
                    r.ctrl,
                    r.id,
                    i == 0,
                );
                app.set_focused_flag(i == 0);
                app.set_shared_log(shared_log);
                app.set_can_add_radio(can_add);
                Tab {
                    id: r.id,
                    name: r.name,
                    app,
                    remote,
                    fallback: None,
                    attached_to: None,
                    enabled: r.enabled,
                }
            })
            .collect();
        assert!(!tabs.is_empty(), "MultiApp needs at least one radio");
        // Read before the strip is rearranged, off the boot order: see
        // [`MultiApp::station_radio`].
        let mut tabs = tabs;
        let station_radio = tabs.iter().find(|t| !t.remote).map(|t| t.id);
        #[cfg(not(target_arch = "wasm32"))]
        {
            // The operator's own order, if they have arranged one. Stable, so
            // the connections — which are in no roster and so in no saved order
            // — keep the order they were opened in, after the local radios.
            let order = sdroxide_config::load_radios().display_order();
            tabs.sort_by_key(|t| order.iter().position(|id| *id == t.id).unwrap_or(usize::MAX));
        }
        // The leftmost chip is the one the session opens on, whichever radio
        // the operator has put there — `new_tab` above seeded the flag from the
        // boot order, which is no longer the order on screen.
        for (i, t) in tabs.iter_mut().enumerate() {
            t.app.set_focused_flag(i == 0);
        }
        // The panes and rows the last session closed with. Without any — a
        // first start, a station from before they were kept — every radio is
        // on the one pane's row: a session opens looking exactly as the single
        // strip always did.
        let ids: Vec<u32> = tabs.iter().map(|t| t.id).collect();
        #[cfg(not(target_arch = "wasm32"))]
        let saved = sdroxide_config::load_radios().layout;
        #[cfg(not(target_arch = "wasm32"))]
        let (panes, rows, focus) = restore_layout(&ids, &saved);
        #[cfg(target_arch = "wasm32")]
        let (panes, rows, focus) = (vec![ids[0]], vec![ids.clone()], None::<u32>);
        let focused = focus.and_then(|id| ids.iter().position(|t| *t == id)).unwrap_or(0);
        for (i, t) in tabs.iter_mut().enumerate() {
            t.app.set_focused_flag(i == focused);
        }
        MultiApp {
            tabs,
            focused,
            panes,
            rows,
            #[cfg(not(target_arch = "wasm32"))]
            saved_layout: Some(saved),
            strip_heights: Vec::new(),
            factory,
            remote,
            wgpu: cc.wgpu_render_state.clone(),
            pending_add: None,
            pending_preset: None,
            peers_opened: std::collections::HashSet::new(),
            station_radio,
            #[cfg(not(target_arch = "wasm32"))]
            fitted: false,
            clip: None,
        }
    }

    /// Put the tabs in `ids` order and remember it (issue #224).
    ///
    /// Tolerant by design: an id that is not here is skipped and a tab the list
    /// does not name keeps its place at the end, because the strip that sent
    /// this drew a frame ago and a radio may have arrived or gone since. The
    /// sort is stable, so those tabs stay in the order they were in.
    ///
    /// Only this machine's own radios are written to the roster: a connection
    /// has no entry there to order.
    fn reorder_tabs(&mut self, ids: &[u32]) {
        let focused_id = self.tabs[self.focused].id;
        self.tabs.sort_by_key(|t| ids.iter().position(|id| *id == t.id).unwrap_or(usize::MAX));
        self.focused = self.tabs.iter().position(|t| t.id == focused_id).unwrap_or(0);
        #[cfg(not(target_arch = "wasm32"))]
        {
            let local: Vec<u32> = self.tabs.iter().filter(|t| !t.remote).map(|t| t.id).collect();
            if let Err(e) = sdroxide_config::reorder_radios(&local) {
                eprintln!("sdroxide: recording the radio order: {e}");
            }
        }
    }

    /// The main window's strip is a switcher, so it is only drawn once there
    /// is something to switch between — with one radio the window looks
    /// exactly as it always has, and radios are managed from Settings → Radio
    /// (which is also where the second one gets added, and the only place a
    /// radio can be deleted from).
    ///
    /// A radio lent to another as its panadapter does not count: a station of
    /// one transceiver and one borrowed receiver is a one-radio station, and
    /// should look like one.
    fn strip_wanted(&self) -> bool {
        self.tabs.iter().filter(|t| t.attached_to.is_none()).count() > 1
    }

    /// Re-read which radios have been lent out as panadapter receivers.
    ///
    /// Once per frame from what each engine reports, rather than from the files
    /// on disk: the answer has to be right for a radio on another machine too,
    /// and it has to follow an Apply the moment the engine has acted on it.
    /// A radio never counts as lent to itself, and one that is on screen is
    /// left there until the operator moves off it — a tab vanishing under the
    /// pointer mid-click is worse than one frame of a stale strip.
    fn refresh_attachments(&mut self) {
        // Each lent radio names its borrower the way its *station* numbers
        // radios, so the borrower is looked up by (station, station id) rather
        // than by tab id: a pairing is a relationship inside one station, and a
        // screen may hold several stations' radios at once — including two
        // whose rosters both have a radio 2. What comes out is a *tab* id,
        // which is what the strip and the reopen request speak.
        let by_station: std::collections::HashMap<(String, u32), u32> = self
            .tabs
            .iter()
            .map(|t| ((t.app.station_key(), t.app.station_radio_id()), t.id))
            .collect();
        let attached: Vec<Option<u32>> = self
            .tabs
            .iter()
            .map(|t| {
                let owner = t.app.lent_to_radio()?;
                by_station.get(&(t.app.station_key(), owner)).copied()
            })
            .collect();
        for (tab, owner) in self.tabs.iter_mut().zip(attached) {
            tab.attached_to = owner;
        }
    }

    /// Re-read the switch on every radio that is at a station rather than on
    /// this machine.
    ///
    /// A radio here is switched by the button below and nothing else, so its
    /// `enabled` is the shell's own. One at a station is switched *there* — by
    /// this client, by another one on it, or at the station itself — so what is
    /// shown is what the station last announced, never what this screen thinks
    /// it asked for. A station that holds no switch for it says so by not
    /// answering, and the radio stays as it arrived: on.
    fn refresh_power(&mut self) {
        for tab in self.tabs.iter_mut().filter(|t| t.remote) {
            if let Some(on) = tab.app.station_power() {
                tab.enabled = on;
            }
        }
    }

    /// Whether this tab's radio has a switch on this screen. One of this
    /// machine's own always does — its roster is here. One at a station has one
    /// only where the station said it would take the request; the rest are
    /// stations too old to have been asked, or hosts that wired none of it up,
    /// and a button there would do nothing.
    fn switchable(t: &Tab) -> bool {
        !t.remote || t.app.station_power().is_some()
    }

    /// The roster as published to the visible tabs, for the settings dialog's
    /// copy of the strip.
    fn roster(&self) -> Vec<RadioChip> {
        // The radio each station will not part with: its first. For this
        // machine that is [`MultiApp::station_radio`] — the radio that holds
        // the shared services and the legacy configuration, which is where the
        // frontend booted it and not wherever its chip has since been dragged
        // to; for a station at the far end it is the first radio in the roster
        // it announced, which is what its `/ws` means.
        let first_local = self.station_radio;
        self.tabs
            .iter()
            .enumerate()
            .map(|(i, t)| RadioChip {
                id: t.id,
                station: t.app.station_key(),
                station_id: t.app.station_radio_id(),
                name: t.name.clone(),
                default_name: Self::default_name(t),
                tx_on: t.app.tab_tx_on(),
                error: t.app.tab_error(),
                muted: t.app.tab_muted(),
                focused: i == self.focused,
                attached_to: t.attached_to,
                enabled: t.enabled,
                // A radio of this machine's own, or one at a station that
                // takes the request — see [`MultiApp::switchable`].
                switchable: Self::switchable(t),
                // Whether the roster this radio is in takes edits from here.
                // For one of this machine's own that is whether the shell can
                // build a radio at all (the browser cannot); for one at the far
                // end of a connection it is what that station said.
                roster_editable: if t.remote {
                    t.app.station_roster_editable()
                } else {
                    self.factory.is_some()
                },
                first_of_station: if t.remote {
                    t.app.peer_first_radio() == Some(t.app.station_radio_id())
                } else {
                    first_local == Some(t.id)
                },
            })
            .collect()
    }

    /// What an unrenamed tab is called: the interface its radio runs, or a
    /// neutral "Radio N" while it has none to be named after. The number is
    /// the id the engine's own messages use ("radio 2 is on the air"), so the
    /// two always agree.
    fn default_name(t: &Tab) -> String {
        if let Some(name) = t.app.interface_name() {
            return name;
        }
        // A radio with no interface yet — one just added at a station, or one
        // whose device is not there. A connection says where it is instead,
        // which is what tells two stations' unconfigured radios apart.
        if let Some(fallback) = &t.fallback {
            return fallback.clone();
        }
        // Numbered the way the station that holds the radio numbers it. A
        // connection's tab id is this screen's own bookkeeping — allocated from
        // `REMOTE_TAB_ID_BASE` so it cannot collide with the roster — and would
        // read as "Radio 2147483649".
        let n = if t.remote { t.app.station_radio_id() } else { t.id };
        tr!("multi-radio", n = n + 1)
    }

    /// The name a tab chip shows: the operator's, else the derived default.
    fn display_name(t: &Tab) -> String {
        if t.name.is_empty() { Self::default_name(t) } else { t.name.clone() }
    }

    /// Re-establish the pane invariants after anything changed the roster or
    /// the panes: every pane shows an existing radio, no radio twice, at
    /// least one pane, and the focused tab is on screen.
    fn sanitize_panes(&mut self, ctx: &egui::Context) {
        self.focused = self.focused.min(self.tabs.len() - 1);
        // The rows go with their panes: first drop ids that no longer exist
        // and any id already on an earlier row, then let a pane whose radio has
        // gone show the next one left on its row, and only then drop a pane
        // with nothing left to show — together with its row.
        self.rows.resize_with(self.panes.len(), Vec::new);
        let mut seen: Vec<u32> = Vec::new();
        for row in &mut self.rows {
            row.retain(|&id| {
                let keep = !seen.contains(&id) && self.tabs.iter().any(|t| t.id == id);
                if keep {
                    seen.push(id);
                }
                keep
            });
        }
        let mut shown: Vec<u32> = Vec::new();
        let mut keep: Vec<bool> = Vec::with_capacity(self.panes.len());
        for k in 0..self.panes.len() {
            let id = self.panes[k];
            let alive = self.tabs.iter().any(|t| t.id == id) && !shown.contains(&id);
            let next = if alive {
                Some(id)
            } else {
                self.rows[k].iter().copied().find(|r| !shown.contains(r) && !self.panes.contains(r))
            };
            match next {
                Some(id) => {
                    self.panes[k] = id;
                    shown.push(id);
                    // What a pane shows is on its row, and on no other.
                    for row in &mut self.rows {
                        row.retain(|&r| r != id);
                    }
                    self.rows[k].push(id);
                    keep.push(true);
                }
                None => keep.push(false),
            }
        }
        let mut at = keep.iter();
        self.panes.retain(|_| *at.next().unwrap_or(&false));
        let mut at = keep.iter();
        self.rows.retain(|_| *at.next().unwrap_or(&false));
        if self.panes.is_empty() {
            let id = self.tabs[self.focused].id;
            self.panes.push(id);
            self.rows.push(vec![id]);
        }
        if !self.panes.contains(&self.tabs[self.focused].id) {
            let first = self.panes[0];
            if let Some(i) = self.tabs.iter().position(|t| t.id == first) {
                self.focus_tab(i, ctx);
            }
        }
    }

    /// Write the panes, rows and focus to the roster when they have changed,
    /// so the next session opens on them. Only a station's own radios count:
    /// a screen showing nothing but somebody else's has no roster of its own
    /// to write.
    #[cfg(not(target_arch = "wasm32"))]
    fn remember_layout(&mut self) {
        let known: Vec<u32> = self.tabs.iter().filter(|t| !t.remote).map(|t| t.id).collect();
        if known.is_empty() {
            return;
        }
        let layout = sdroxide_config::PaneLayout {
            panes: self.panes.clone(),
            rows: self.rows.clone(),
            focused: self.tabs.get(self.focused).map(|t| t.id),
            known,
        };
        if self.saved_layout.as_ref() == Some(&layout) {
            return;
        }
        if let Err(e) = sdroxide_config::save_pane_layout(&layout) {
            eprintln!("sdroxide: recording the panes: {e}");
        }
        self.saved_layout = Some(layout);
    }

    /// The pane the focused radio sits on — where "switch to radio X"
    /// requests land while the view is split.
    fn active_pane(&self) -> usize {
        let fid = self.tabs[self.focused].id;
        self.panes.iter().position(|&p| p == fid).unwrap_or(0)
    }

    /// Put radio `id` on pane `pane` and hand it the keyboard. A radio that
    /// is already on screen only takes focus — the strip disables its name
    /// chip everywhere else, so the same radio is never shown twice.
    fn show_in_pane(&mut self, pane: usize, id: u32, ctx: &egui::Context) {
        let Some(i) = self.tabs.iter().position(|t| t.id == id) else { return };
        if !self.panes.contains(&id) && pane < self.panes.len() {
            self.panes[pane] = id;
            self.put_on_row(pane, id);
        }
        self.focus_tab(i, ctx);
    }

    /// Put radio `id` on pane `pane`'s row, and on no other.
    fn put_on_row(&mut self, pane: usize, id: u32) {
        for row in &mut self.rows {
            row.retain(|&r| r != id);
        }
        if let Some(row) = self.rows.get_mut(pane) {
            row.push(id);
        }
    }

    /// "Pick radio ▼": radio `id` onto pane `pane`'s row, shown there and
    /// handed the keyboard. A radio another pane is showing leaves that pane,
    /// which shows the next radio on its row, or closes when its row is empty.
    fn pick(&mut self, pane: usize, id: u32, ctx: &egui::Context) {
        if pane >= self.panes.len() || !self.tabs.iter().any(|t| t.id == id) {
            return;
        }
        if let Some(k) = self.panes.iter().position(|&p| p == id)
            && k != pane
        {
            self.panes[k] = u32::MAX;
        }
        self.put_on_row(pane, id);
        self.panes[pane] = id;
        // `u32::MAX` is no tab: the next line finds the pane it was put on
        // the next radio on its row, or closes it.
        self.sanitize_panes(ctx);
        if let Some(i) = self.tabs.iter().position(|t| t.id == id) {
            self.focus_tab(i, ctx);
        }
    }

    /// The "−" on a tab: radio `id` off pane `pane`'s row. It keeps running —
    /// the same as the wildtype, where a radio behind the one on screen goes
    /// on receiving — and "Pick radio ▼" brings it back. Where it was the one
    /// shown, the pane shows the next radio on its row, or closes; the last
    /// tab of the last pane offers no "−".
    fn unpick(&mut self, pane: usize, id: u32, ctx: &egui::Context) {
        let Some(row) = self.rows.get_mut(pane) else { return };
        row.retain(|&r| r != id);
        if self.panes[pane] == id {
            match self.rows[pane].first().copied() {
                Some(next) => self.panes[pane] = next,
                None if self.panes.len() > 1 => {
                    self.panes.remove(pane);
                    self.rows.remove(pane);
                }
                // Refused by the strip, which draws no "−" there; put back.
                None => self.rows[pane].push(id),
            }
        }
        self.sanitize_panes(ctx);
    }

    /// The columns chip at the end of a row: another pane, showing the first
    /// radio that is on no row. Nothing when every radio is placed (the strip
    /// does not draw the chip then).
    fn add_pane(&mut self, ctx: &egui::Context) {
        let Some(id) = self.next_unshown() else { return };
        self.panes.push(id);
        self.rows.push(Vec::new());
        self.put_on_row(self.panes.len() - 1, id);
        if let Some(i) = self.tabs.iter().position(|t| t.id == id) {
            self.focus_tab(i, ctx);
        }
    }

    /// The first radio, in strip order, that is on no row: one the operator
    /// took off a row with "−". Never one lent out as a panadapter, which is
    /// reached through Settings → Radio.
    fn next_unshown(&self) -> Option<u32> {
        self.loose().first().copied()
    }

    /// The radios on no row, in strip order: what "Pick radio ▼" and the
    /// columns chip have to offer. While it is empty every radio is placed,
    /// and neither is drawn.
    // Why: HDZ 2026-11-04: "wenn alle Radios platziert sind, dann wird Popdown
    //      und Spalten-Eumel ausgeblendet, nur wenn ich irgendwo ein Radio vom
    //      Panel löse, kann ich eine neue Spalte oder einen neuen Selektor
    //      machen".
    fn loose(&self) -> Vec<u32> {
        self.tabs
            .iter()
            .filter(|t| t.attached_to.is_none())
            .filter(|t| !self.rows.iter().any(|r| r.contains(&t.id)))
            .map(|t| t.id)
            .collect()
    }

    /// Act on the radio-management requests the visible tabs' settings
    /// dialogs queued this frame.
    fn handle_requests(&mut self, reqs: Vec<RadioTabRequest>, ctx: &egui::Context) {
        for req in reqs {
            match req {
                RadioTabRequest::Focus(id) => {
                    if let Some(i) = self.tabs.iter().position(|t| t.id == id)
                        && i != self.focused
                    {
                        // The dialog follows the switch: the operator asked to
                        // work on that radio's settings, not to leave a stale
                        // dialog behind on this one. In a split the target
                        // takes over the active pane rather than adding one.
                        self.tabs[self.focused].app.close_settings();
                        self.show_in_pane(self.active_pane(), id, ctx);
                        self.tabs[i].app.open_radio_settings();
                    }
                }
                // Added exactly as before, then named: the radio that was not
                // here a moment ago is the one this request made.
                // Why: HDZ 2026-11-04: "dabei wäre … der Sendername aus dem
                //      Katalog zu übernehmen". Naming after the fact keeps
                //      `add_radio`/`add_tab` and the strip's own "+" as they
                //      are, so this works with the original tab strip and with
                //      Panel Exchange alike ("es sollte ein Fallback-Verhalten
                //      auf die Ausgangsversion geben").
                // Pitfall: a radio added at a station is not here yet — it
                //          arrives when the station announces it, and is named
                //          there — so only this machine's radios get the name.
                RadioTabRequest::Add { station, preset, name } => {
                    let before: Vec<u32> = self.tabs.iter().map(|t| t.id).collect();
                    let here = station.is_empty();
                    self.add_radio(&station, preset, ctx);
                    if let Some(name) = name.filter(|n| here && !n.is_empty()) {
                        let new = self.tabs.iter().map(|t| t.id).find(|id| !before.contains(id));
                        if let Some(id) = new {
                            self.rename_tab(id, &name);
                        }
                    }
                }
                #[cfg(not(target_arch = "wasm32"))]
                RadioTabRequest::Connect { url, name } => self.connect_tab(&url, name, ctx),
                RadioTabRequest::Close(id) => {
                    if let Some(i) = self.tabs.iter().position(|t| t.id == id) {
                        self.close_tab(i, ctx);
                    }
                }
                RadioTabRequest::Mute { id, muted } => {
                    if let Some(t) = self.tabs.iter_mut().find(|t| t.id == id) {
                        t.app.set_tab_muted(muted);
                    }
                }
                RadioTabRequest::Power { id, on } => self.set_power(id, on),
                RadioTabRequest::Reopen(id) => {
                    if let Some(t) = self.tabs.iter_mut().find(|t| t.id == id) {
                        t.app.reopen_source();
                    }
                }
                RadioTabRequest::Rename { id, name } => self.rename_tab(id, &name),
                RadioTabRequest::RemoveFromStation(id) => self.remove_at_station(id),
                RadioTabRequest::Reorder(ids) => self.reorder_tabs(&ids),
                // Normally taken straight from the radio that asked, in
                // `take_requests`; one that arrives here belongs to the
                // focused radio.
                RadioTabRequest::Clip(act) => self.clip_act(self.focused, act),
            }
        }
    }

    /// Collect radio `ti`'s requests into `reqs`, acting on its clipboard
    /// clicks at once: they need to know which radio was clicked, and the
    /// focus may sit on another pane.
    fn take_requests(&mut self, ti: usize, reqs: &mut Vec<RadioTabRequest>) {
        for r in self.tabs[ti].app.take_radio_tab_requests() {
            match r {
                RadioTabRequest::Clip(act) => self.clip_act(ti, act),
                other => reqs.push(other),
            }
        }
    }

    /// One clipboard click on radio `ti`: copy fills the clipboard from it,
    /// paste lays the clipboard on it (the clipboard stays filled), clear
    /// empties it.
    fn clip_act(&mut self, ti: usize, act: ClipAct) {
        let Some(tab) = self.tabs.get_mut(ti) else {
            return;
        };
        match act {
            ClipAct::Copy(c) => self.clip = Some(c),
            ClipAct::Clear => self.clip = None,
            ClipAct::Paste => {
                if let Some(c) = self.clip {
                    tab.app.apply_clip(c);
                }
            }
        }
    }

    /// Give radio `id` the name `name`, here and where the radio is kept.
    ///
    /// In: the tab id and the new name. Effect: the tab shows it at once; a
    /// radio of this machine has it written to the roster on disk, a radio at
    /// a station has it sent to that station's roster where the station takes
    /// edits. Fails: a roster that cannot be written is reported on stderr and
    /// the name lasts for this session.
    // Why: one routine for the RENAME request and for a radio added from
    //      "Public SDRs" with its directory name (`RadioTabRequest::Add`), so the second is
    //      stored exactly like the first.
    fn rename_tab(&mut self, id: u32, name: &str) {
        let Some(t) = self.tabs.iter_mut().find(|t| t.id == id) else { return };
        t.name = name.to_string();
        // A radio at the far end of a connection is named in the roster where
        // it lives, so the name goes there — otherwise it would last exactly as
        // long as this connection, and nobody else on that station would ever
        // see it. A station that does not take roster edits keeps the name
        // here and nowhere else, which is what it has always done.
        if t.remote {
            if t.app.station_roster_editable() {
                let station_id = t.app.station_radio_id();
                t.app.rename_station_radio(station_id, name);
            }
            return;
        }
        // In the browser there is no roster to record it in at all: every tab
        // there is somebody else's radio.
        #[cfg(not(target_arch = "wasm32"))]
        if let Err(e) = sdroxide_config::rename_radio(id, name) {
            eprintln!("sdroxide: renaming radio {id}: {e}");
        }
    }

    /// What the strip asked for, applied once the tabs are no longer
    /// borrowed for drawing.
    fn apply_strip_actions(&mut self, actions: Vec<StripAction>, ctx: &egui::Context) {
        for act in actions {
            match act {
                StripAction::Show { pane, id } => self.show_in_pane(pane, id, ctx),
                StripAction::Pick { pane, id } => self.pick(pane, id, ctx),
                StripAction::Unpick { pane, id } => self.unpick(pane, id, ctx),
                StripAction::AddPane => self.add_pane(ctx),
                StripAction::Mute { id, muted } => {
                    // To the engine, not latched here: the chip and the MUTE
                    // button both read the engine's answer back, so they
                    // cannot drift apart — and the engine remembers it.
                    if let Some(t) = self.tabs.iter_mut().find(|t| t.id == id) {
                        t.app.set_tab_muted(muted);
                    }
                }
                StripAction::Power { id, on } => self.set_power(id, on),
                StripAction::ShowOnMap(id) => {
                    if let Some(t) = self.tabs.iter_mut().find(|t| t.id == id) {
                        t.app.show_self_on_public_map();
                    }
                }
                // The main window's strip only ever offers this machine's own
                // roster — it is drawn on every pane and a menu there would be
                // in the way. Adding a radio at a station is one screen away,
                // in Settings → Radio, where the two rosters are already side
                // by side.
                StripAction::Add => self.add_radio("", None, ctx),
            }
        }
    }

    /// Switch a radio on or off: record it in the roster, then have the engine
    /// rebuild its front end, which is where the answer takes effect — the
    /// interface factory reads the roster and either opens the radio or hands
    /// back the stand-in that holds nothing open.
    ///
    /// For a radio at a station both of those happen *there*, and this only
    /// asks. Same switch, same result; the roster it is written in is the one
    /// the radio's own machine keeps.
    ///
    /// The tab, its engine and its whole configuration stay exactly where they
    /// are either way. This is not closing a radio; it is putting it down.
    fn set_power(&mut self, id: u32, on: bool) {
        let Some(tab) = self.tabs.iter_mut().find(|t| t.id == id) else { return };
        if tab.enabled == on {
            return;
        }
        // Somebody else's radio has no entry in this machine's roster: the
        // request goes over that tab's own connection, named the way the
        // station numbers its radios. Nothing is changed here — the station
        // announces the switch's new position to everyone on it, this client
        // included, and `refresh_power` picks it up next frame.
        if tab.remote {
            let station_id = tab.app.station_radio_id();
            tab.app.switch_station_radio(station_id, on);
            return;
        }
        tab.enabled = on;
        #[cfg(not(target_arch = "wasm32"))]
        if let Err(e) = sdroxide_config::set_radio_enabled(id, on) {
            eprintln!("sdroxide: switching radio {id} {}: {e}", if on { "on" } else { "off" });
        }
        tab.app.reopen_source();
        // The switch reaches the receiver this radio borrows as its panadapter,
        // where it borrows one. That receiver goes back to being a radio of its
        // own while the borrower is off and hands its device over again when the
        // borrower comes on, and neither move happens by itself: an engine with
        // a working front end has no reason to rebuild it. So it is asked to,
        // and finds out from the roster which of the two it is doing.
        let (station, borrowed) = (tab.app.station_key(), tab.app.pan_source_radio());
        if let Some(rx) = borrowed
            && let Some(i) = self.tabs.iter().position(|t| {
                !t.remote && t.app.station_key() == station && t.app.station_radio_id() == rx
            })
        {
            self.tabs[i].app.reopen_source();
        }
    }

    /// One pane's row: the radios picked onto it, each a tab with its own
    /// controls (the rings, mute, "−"), then "Pick radio ▼" and the "+" that
    /// opens another pane. `pane` is the column the row sits on (0 in the
    /// single view). Closing a radio is deliberately absent — that lives in
    /// Settings → Radio, behind a dialog, not one stray click away on the main
    /// window; "−" only takes a radio off this row.
    // Why: HDZ 2026-11-04 (see [`MultiApp::rows`]); the link switch on a tab
    //      wears the rings the frequency box's switch wears, so one switch
    //      has one shape wherever it stands ("zwei verschiedene Knöpfe machen
    //      das gleiche … einmal das Ringe-Symbol und einmal der andere Knopf").
    // Invariant: a tab is drawn for exactly the radios on `rows[pane]`, in
    //      the order of `tabs`.
    fn strip_row(&self, ui: &mut egui::Ui, pane: usize, actions: &mut Vec<StripAction>) {
        let row: &[u32] = self.rows.get(pane).map_or(&[], Vec::as_slice);
        let last_tab_standing = self.panes.len() == 1 && row.len() == 1;
        crate::chrome::tab_bar(ui, |ui, bar| {
            for (i, tab) in self.tabs.iter().enumerate() {
                if !row.contains(&tab.id) {
                    continue;
                }
                let id = tab.id;
                let here = self.panes.get(pane) == Some(&id);
                // In a split the accent marks the pane holding the keyboard.
                let accent = if here && self.panes.len() > 1 && i == self.focused {
                    crate::theme::PINK()
                } else {
                    crate::theme::CYAN()
                };
                // A real tab, not a chip: this row decides which radio the
                // page below belongs to, which is the one thing a row of
                // buttons cannot say. Its own controls ride inside it the way
                // a close box rides in a browser's.
                let body = bar.tab_body_accent(ui, here, accent, |ui| {
                    let mut label = RichText::new(Self::display_name(tab)).size(12.5);
                    if here {
                        label = label.strong().color(crate::theme::TEXT_STRONG());
                    }
                    // A radio nobody has switched on is a radio the row is
                    // only carrying so that it can be switched on: it says its
                    // name and no more.
                    if !tab.enabled {
                        label = label.weak();
                    }
                    // The name of the radio this pane shows is a link to the
                    // map, where that radio is a public receiver: underlined
                    // while the pointer is on it, as a link reads. Anywhere
                    // else a click on the tab is what it always was.
                    // Why: HDZ 2026-11-04: "wenn du linken kannst, kannst du den
                    //      Link auch ohne Symbol an den Namen heften"; only on
                    //      the tab already shown, because a click on any other
                    //      tab shows that radio.
                    if here && tab.app.on_public_sdr(ui.input(|i| i.time)) {
                        let name = Self::tab_name(ui, label, egui::Sense::click());
                        let name = if name.hovered() {
                            ui.painter().hline(
                                name.rect.x_range(),
                                name.rect.bottom() - 1.0,
                                egui::Stroke::new(1.0, crate::theme::CYAN()),
                            );
                            name.on_hover_cursor(egui::CursorIcon::PointingHand)
                        } else {
                            name
                        };
                        if name.on_hover_text(tr!("multi-show-on-map")).clicked() {
                            actions.push(StripAction::ShowOnMap(id));
                        }
                    } else {
                        Self::tab_name(ui, label, egui::Sense::hover());
                    }
                    // On the air: the one thing worth seeing from any tab.
                    if tab.app.tab_tx_on() {
                        ui.label(
                            RichText::new(tr!("multi-tx")).size(11.0).color(crate::theme::ALERT()),
                        );
                    } else if tab.app.tab_error() && tab.enabled {
                        ui.label(RichText::new("⚠").size(11.0).color(crate::theme::ALERT()));
                    }
                    // The switch, as the rings: lit while the link is open
                    // (issue #253), and the same request the frequency box's
                    // rings send, so the two can never disagree.
                    if Self::switchable(tab) && tab.attached_to.is_none() {
                        let h = crate::chrome::chip_height(ui, Some(11.0));
                        let power =
                            crate::chrome::chip_link(ui, tab.enabled, egui::vec2(1.8 * h, h));
                        let tip = if tab.enabled {
                            crate::chrome::LINK_CLOSE_TIP
                        } else {
                            crate::chrome::LINK_OPEN_TIP
                        };
                        if power.on_hover_text(tip).clicked() {
                            actions.push(StripAction::Power { id, on: !tab.enabled });
                        }
                    }
                    // No mute on a radio that is off: there is nothing coming
                    // out of it to silence. Whether it *was* muted is kept, and
                    // the button comes back with it when the radio does.
                    if tab.enabled {
                        let muted = tab.app.tab_muted();
                        let mute = crate::chrome::chip(
                            ui,
                            muted,
                            RichText::new(if muted { "🔇" } else { "🔊" }).size(11.0),
                        );
                        if mute.on_hover_text(tr!("multi-mute-radio-s-audio")).clicked() {
                            actions.push(StripAction::Mute { id, muted: !muted });
                        }
                    }
                    // Off this row. Not offered on the last tab of the last
                    // pane: a window shows a radio, always.
                    if !last_tab_standing {
                        let minus = crate::chrome::chip(ui, false, RichText::new("−").size(11.0));
                        if minus.on_hover_text(tr!("multi-take-radio-off-row")).clicked() {
                            actions.push(StripAction::Unpick { pane, id });
                        }
                    }
                });
                // The tab itself switches the pane — anywhere on it that is not
                // one of its own buttons, which keep their clicks.
                let body = if here {
                    body.response
                } else {
                    body.response.on_hover_text(tr!("multi-show-radio-here"))
                };
                if body.clicked() && !here {
                    actions.push(StripAction::Show { pane, id });
                }
            }
            bar.end_tabs(ui);
            // Only while a radio is loose — on no row — is there anything to
            // pick or to open a pane for; with every radio placed the row ends
            // at its last tab. A new radio is then added in Settings → Radio.
            if self.loose().is_empty() {
                return;
            }
            self.pick_menu(ui, pane, actions);
            // Another pane, with the first loose radio. Drawn as columns, not
            // "+": "+" adds a radio everywhere else.
            let h = crate::chrome::chip_height(ui, Some(13.0));
            if crate::chrome::chip_columns(ui, egui::vec2(1.6 * h, h))
                .on_hover_text(tr!("multi-open-pane"))
                .clicked()
            {
                actions.push(StripAction::AddPane);
            }
        });
    }

    /// A tab's name, centred in the height of the chips beside it.
    ///
    /// In: the name as it is to be drawn, and what it should sense. Out: the
    /// name's response. The name gets a box as tall as a chip and sits in its
    /// middle, so it stands at the same height in every tab.
    // Why: HDZ 2026-11-04 (strip in two rows): "der Name des Funkgeräts ist
    //      oft mittenzentriert in der Höhe, aber nicht immer konsistent". The
    //      first tab of each row had its name at the top: egui centres an item
    //      in the height the row has reached so far, and for a row's first
    //      item that is still the line height, before the taller rings chip
    //      beside it has been laid out.
    fn tab_name(ui: &mut egui::Ui, name: RichText, sense: egui::Sense) -> egui::Response {
        let h = crate::chrome::chip_height(ui, Some(11.0));
        let galley = egui::WidgetText::from(name.clone()).into_galley(
            ui,
            Some(egui::TextWrapMode::Extend),
            f32::INFINITY,
            egui::TextStyle::Button,
        );
        let size = egui::vec2(galley.size().x, h.max(galley.size().y));
        ui.allocate_ui_with_layout(size, egui::Layout::left_to_right(egui::Align::Center), |ui| {
            ui.add(egui::Label::new(name).sense(sense))
        })
        .inner
    }

    /// "Pick radio ▼": the radios on no row, each marked with the rings while
    /// its link is open and dimmed while it is off — the same marks the tabs
    /// wear. Drawn only while there is one ([`MultiApp::loose`]). Picking puts
    /// it on this row and shows it here. At the foot, "New radio…", which is
    /// what the single strip's "+" used to do.
    fn pick_menu(&self, ui: &mut egui::Ui, pane: usize, actions: &mut Vec<StripAction>) {
        let btn = crate::chrome::chip(ui, false, RichText::new(tr!("multi-pick-radio")).size(12.0))
            .on_hover_text(tr!("multi-pick-radio-tip"));
        crate::chrome::menu_popup(ui, &btn, |ui| {
            let loose = self.loose();
            for tab in self.tabs.iter().filter(|t| loose.contains(&t.id)) {
                let mut label = RichText::new(Self::display_name(tab));
                if !tab.enabled {
                    label = label.weak();
                }
                let picked = ui
                    .horizontal(|ui| {
                        // The rings, painted: the symbol is not in the text
                        // font (see [`crate::chrome::chip_link`]).
                        let (rect, _) =
                            ui.allocate_exact_size(egui::vec2(18.0, 12.0), egui::Sense::hover());
                        if tab.enabled && ui.is_rect_visible(rect) {
                            let r = rect.height() * 0.32;
                            let c = rect.center();
                            let stroke = egui::Stroke::new(1.4, crate::theme::CYAN());
                            for dx in [-0.55, 0.55] {
                                ui.painter().circle_stroke(
                                    egui::pos2(c.x + dx * r, c.y),
                                    r,
                                    stroke,
                                );
                            }
                        }
                        ui.add(egui::Button::new(label).frame(false)).clicked()
                    })
                    .inner;
                if picked {
                    actions.push(StripAction::Pick { pane, id: tab.id });
                    ui.close();
                }
            }
            if self.factory.is_some() {
                ui.separator();
                let tip = match self.tabs.iter().any(|t| t.remote) {
                    true => tr!("multi-add-radio-computer-settings-radio"),
                    false => tr!("multi-add-radio"),
                };
                if ui.button(tr!("multi-new-radio")).on_hover_text(tip).clicked() {
                    actions.push(StripAction::Add);
                    ui.close();
                }
            }
        });
    }

    fn focus_tab(&mut self, i: usize, ctx: &egui::Context) {
        if i == self.focused || i >= self.tabs.len() {
            return;
        }
        self.tabs[self.focused].app.set_focused(false, ctx);
        self.focused = i;
        self.tabs[i].app.set_focused(true, ctx);
        self.sync_audio();
    }

    /// The browser has one sound output for the whole page — a single worklet,
    /// fed by whichever tab pushes into it — so exactly one radio may be
    /// audible there, and it is the one the operator is on. A native client
    /// gives every radio a stream of its own and lets the sound system mix
    /// them, which is why this rule is the browser's alone. The operator's
    /// mute is not part of it: that lives in the engine, which zeroes the
    /// stream at the source.
    #[cfg(target_arch = "wasm32")]
    fn sync_audio(&mut self) {
        let focused = self.focused;
        for (i, tab) in self.tabs.iter_mut().enumerate() {
            tab.app.mute_tab(i != focused);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn sync_audio(&mut self) {}

    /// Put another radio in a roster: this machine's own (`station` empty), or
    /// that of a station this screen is connected to.
    ///
    /// The two are as different as plugging a dongle in here and plugging one
    /// in at the remote site, which is why the request says which. A radio
    /// added at a station is not built here at all — the station creates it,
    /// starts its engine and announces its roster again, and the new radio
    /// arrives as one more of that station's, through the same path the rest of
    /// them did.
    fn add_radio(
        &mut self,
        station: &str,
        preset: Option<Box<sdroxide_types::RadioConfig>>,
        ctx: &egui::Context,
    ) {
        if station.is_empty() {
            self.add_tab(preset, ctx);
            return;
        }
        let Some(t) = self.tabs.iter_mut().find(|t| t.remote && t.app.station_key() == station)
        else {
            return;
        };
        // Unnamed, exactly as the "+" chip creates one here: a radio is named
        // after whatever interface it ends up configured as until the operator
        // says otherwise.
        t.app.add_station_radio("");
        self.pending_add = Some(station.to_string());
        // Held until the station announces the radio and `open_peer_radios`
        // opens it: the radio does not exist yet, so there is nothing to
        // configure until then.
        self.pending_preset = preset;
        // Nothing else happens here. The station answers with its roster, and
        // `open_peer_radios` opens what is new in it — including, for a radio
        // this screen asked for, putting it in front of the operator.
        self.tabs[self.focused]
            .app
            .show_notice(tr!("multi-asked-station-another-radio").to_string());
    }

    /// Take a tab's radio out of the roster of the station it belongs to.
    ///
    /// Sent through that radio's own session, which the station then closes:
    /// the roster it announces on the way out no longer lists the radio, and
    /// the tab closes itself on that ([`MultiApp::open_peer_radios`]). Closing
    /// it here instead would be this screen deciding an answer that is the
    /// station's to give — and would drop the socket the request has to go out
    /// on.
    fn remove_at_station(&mut self, tab: u32) {
        let Some(t) = self.tabs.iter_mut().find(|t| t.id == tab) else { return };
        if !t.remote || !t.app.station_roster_editable() {
            return;
        }
        let station_id = t.app.station_radio_id();
        t.app.remove_station_radio(station_id);
    }

    fn add_tab(&mut self, preset: Option<Box<sdroxide_types::RadioConfig>>, ctx: &egui::Context) {
        let Some(factory) = self.factory.as_mut() else { return };
        match factory() {
            Ok(r) => {
                let i = self.install_tab(r, ctx);
                match preset {
                    // Already configured — it came from "Public SDRs", which
                    // knows the whole answer. Opening the settings page over it
                    // would be asking a question that has been answered.
                    Some(cfg) => self.tabs[i].app.apply_radio_config(*cfg),
                    // The new tab has no interface yet; the operator's next
                    // stop is Settings → Radio, so open it for them.
                    None => self.tabs[i].app.open_radio_settings(),
                }
            }
            // Into the focused tab's dismissable banner — the main window's
            // strip may not be on screen to carry a message.
            Err(e) => self.tabs[self.focused]
                .app
                .show_notice(tr!("multi-could-not-add-radio", error = e.to_string())),
        }
    }

    /// A free id for a tab that holds somebody else's radio. Never one from
    /// the roster: that station is not one of ours.
    fn next_remote_id(&self) -> u32 {
        self.tabs
            .iter()
            .map(|t| t.id)
            .filter(|id| *id >= REMOTE_TAB_ID_BASE)
            .max()
            .map_or(REMOTE_TAB_ID_BASE, |m| m.saturating_add(1))
    }

    /// Bring up the rest of a station's radios beside the one that was
    /// dialled, so connecting to a station gives the same tab strip as
    /// standing in it: a server serves every radio in its roster, and an
    /// operator who asked for the station meant the station.
    ///
    /// Each address is opened once. A tab the operator closes stays closed —
    /// the far end goes on offering it, and reopening it every frame would
    /// make it impossible to shut. Addresses already on screen are skipped
    /// too, which is what keeps two tabs of the same station from opening each
    /// other for ever.
    fn open_peer_radios(&mut self, ctx: &egui::Context) {
        // A tab that arrived without a name of its own takes the station's,
        // once the station has said what that is: the browser client dials the
        // page's own host, so there is no address anybody typed to name it
        // after. A tab that *was* named — dialled from Settings → Remote, or
        // renamed since — keeps what it has.
        for tab in &mut self.tabs {
            if tab.name.is_empty()
                && let Some(name) = tab.app.peer_name()
                && !name.trim().is_empty()
            {
                tab.name = name;
            }
        }
        // A radio the station has closed — by this client, by another one, or
        // at the station itself. The roster it announced on its way out no
        // longer lists this tab's radio, so the tab goes with it: the socket is
        // about to shut, and leaving it up would show a lost connection and
        // offer to redial an address that is now a 404.
        //
        // Never the last tab standing: a window with no radio in it has nothing
        // to draw, and a client dialled straight at one radio would be left
        // looking at nothing. That tab keeps its lost-connection banner, which
        // is at least the truth.
        while self.tabs.len() > 1
            && let Some(i) = self.tabs.iter().position(|t| t.remote && t.app.peer_removed())
        {
            let name = Self::display_name(&self.tabs[i]);
            self.close_tab(i, ctx);
            let focused = self.focused;
            self.tabs[focused].app.show_notice(tr!("multi-was-closed-station", value = name));
        }
        if self.remote.is_none() {
            return;
        }
        let open: std::collections::HashSet<String> =
            self.tabs.iter().filter_map(|t| t.app.peer_url()).collect();
        let wanted: Vec<sdroxide_types::PeerRadio> = self
            .tabs
            .iter()
            .flat_map(|t| t.app.peer_radios())
            .filter(|p| !open.contains(&p.url) && !self.peers_opened.contains(&p.url))
            .collect();

        for peer in wanted {
            // Marked before the attempt, not after: a station that cannot be
            // reached must not be retried once a frame for the rest of the
            // session.
            self.peers_opened.insert(peer.url.clone());
            let id = self.next_remote_id();
            // A radio this screen asked the station for, arriving. That one
            // goes in front of the operator on its Radio settings page — it has
            // no interface yet, and choosing one is what they pressed "+" to
            // do — while everything else keeps arriving quietly behind them.
            let asked_for =
                self.pending_add.as_deref() == Some(crate::login::station_key(&peer.url).as_str());
            let Some(remote) = self.remote.as_mut() else { return };
            match remote(&peer.url, id, ctx) {
                Ok(mut r) => {
                    // Named as the operator named it at the station — and left
                    // unnamed where they never did, so that the tab derives its
                    // own name from the radio it is now connected to. A radio
                    // added from away has no interface yet, so the name the
                    // station has for it is "No radio"; adopting that would
                    // still be its name after the operator had picked one.
                    //
                    // What the dialler called it becomes the fallback rather
                    // than the name, for the same reason: it says where the
                    // radio is, which is worth showing while it has nothing
                    // else, and is not something anybody typed.
                    let fallback = match peer.named && !peer.name.trim().is_empty() {
                        true => {
                            let dialled = std::mem::replace(&mut r.name, peer.name.clone());
                            (!dialled.trim().is_empty()).then_some(dialled)
                        }
                        false => {
                            let dialled = std::mem::take(&mut r.name);
                            (!dialled.trim().is_empty()).then_some(dialled)
                        }
                    };
                    // Into the strip, not in front of the operator: they asked
                    // for the station, and the radio they dialled is the one
                    // they are looking at. A radio *this* screen asked the
                    // station for is the exception — see `asked_for`.
                    let i = if asked_for {
                        self.pending_add = None;
                        let i = self.install_tab(r, ctx);
                        match self.pending_preset.take() {
                            Some(cfg) => self.tabs[i].app.apply_radio_config(*cfg),
                            None => self.tabs[i].app.open_radio_settings(),
                        }
                        i
                    } else {
                        self.append_tab(r, ctx)
                    };
                    self.tabs[i].fallback = fallback;
                }
                Err(e) => self.tabs[self.focused].app.show_notice(tr!(
                    "multi-could-not-open",
                    peer = peer.name,
                    error = e.to_string()
                )),
            }
        }
    }

    /// Dial another sdroxide server and give it a tab of its own — Settings →
    /// Remote's CONNECT button, which is native-only (see
    /// [`RadioTabRequest::Connect`](crate::app::RadioTabRequest)).
    ///
    /// The verdict goes back to the tab that asked, not to the new one: a
    /// connection that never opened has no tab to report from, and the dialog
    /// the operator is looking at is where they are expecting an answer.
    #[cfg(not(target_arch = "wasm32"))]
    fn connect_tab(&mut self, url: &str, name: String, ctx: &egui::Context) {
        let origin = self.focused;
        if self.remote.is_none() {
            self.tabs[origin]
                .app
                .set_remote_status(Err(tr!("multi-client-cannot-open-connection")));
            return;
        }
        // The id is worked out before the factory is borrowed: both read
        // `self`, and only one of them may hold it.
        let id = self.next_remote_id();
        let Some(remote) = self.remote.as_mut() else { return };
        match remote(url, id, ctx) {
            Ok(mut r) => {
                // The address as the operator entered it wins over whatever
                // the factory called it — it is what they will recognise in
                // the strip.
                r.name = name;
                let label = r.name.clone();
                self.install_tab(r, ctx);
                // Not "connected": the socket opens in the background, and
                // whether the station answers is reported by the tab it now
                // has — with the sign-in screen, if it asks for one.
                self.tabs[origin]
                    .app
                    .set_remote_status(Ok(tr!("multi-has-tab-own-now", value = label)));
            }
            Err(e) => self.tabs[origin].app.set_remote_status(Err(format!("{url}: {e}"))),
        }
    }

    /// Append a freshly built radio to the strip without disturbing what is on
    /// screen. Returns its index.
    ///
    /// This is the whole of what a radio the *shell* opened needs — the rest
    /// of a station's roster arriving beside the one that was dialled. A radio
    /// the operator asked for goes through [`MultiApp::install_tab`], which
    /// also puts it in front of them.
    fn append_tab(&mut self, r: RadioTab, ctx: &egui::Context) -> usize {
        let remote = r.ctrl.engine_is_remote();
        let mut app = SdroxideApp::new_tab(
            ctx,
            // A brand-new radio has no saved view to restore, and the
            // station-wide settings are read from their real files on
            // native; storage is only a wasm concern, and wasm has no
            // factory.
            None,
            self.wgpu.clone(),
            r.ctrl,
            r.id,
            false,
        );
        app.set_focused_flag(false);
        app.set_can_add_radio(self.factory.is_some());
        self.tabs.push(Tab {
            id: r.id,
            name: r.name,
            app,
            remote,
            fallback: None,
            attached_to: None,
            enabled: r.enabled,
        });
        // On the row of the pane the operator is working in, so the radio they
        // asked for — or the next of a station's arriving — is one click away.
        let pane = self.active_pane();
        self.put_on_row(pane, r.id);
        for tab in &mut self.tabs {
            tab.app.set_shared_log(true);
        }
        // A radio that arrives behind the one on screen must not start talking
        // over it where there is only one output to talk through.
        self.sync_audio();
        self.tabs.len() - 1
    }

    /// Put a freshly built radio on screen: append it, take over the active
    /// pane, hand it the keyboard. Returns its index.
    fn install_tab(&mut self, r: RadioTab, ctx: &egui::Context) -> usize {
        let new_id = r.id;
        let i = self.append_tab(r, ctx);
        // The dialog follows: if the request came from inside Settings, that
        // dialog belongs on the new radio now (a no-op when it came from the
        // main window's strip).
        self.tabs[self.focused].app.close_settings();
        // In a split the new radio takes over the active pane rather than
        // opening yet another column unasked.
        let pane = self.active_pane();
        self.panes[pane] = new_id;
        self.focus_tab(i, ctx);
        i
    }

    /// Hand the station radio's network spots to this machine's other radios.
    /// See [`SdroxideApp::adopt_spot_feed`]. A connection to another station
    /// keeps its own: that station's radio runs its own feeds.
    fn share_spot_feed(&mut self) {
        let Some(station) = self.station_radio else { return };
        let Some(src) = self.tabs.iter().position(|t| t.id == station) else { return };
        let generation = self.tabs[src].app.spot_feed().0;
        let behind = |i: usize, t: &Tab| i != src && !t.remote && t.app.wants_spot_feed(generation);
        if !self.tabs.iter().enumerate().any(|(i, t)| behind(i, t)) {
            return;
        }
        let (spots, status) = {
            let (_, spots, status) = self.tabs[src].app.spot_feed();
            (spots.to_vec(), status.map(str::to_string))
        };
        for (i, tab) in self.tabs.iter_mut().enumerate() {
            if behind(i, tab) {
                tab.app.adopt_spot_feed(generation, &spots, status.as_deref());
            }
        }
    }

    fn close_tab(&mut self, i: usize, ctx: &egui::Context) {
        if i >= self.tabs.len() || self.tabs.len() == 1 {
            return;
        }
        // The station radio stays: it holds the shared network services and the
        // legacy configuration, and a window is a station. Named rather than
        // "the first tab", because the strip's order is the operator's to
        // arrange (issue #224) and this radio's standing is not.
        //
        // A *connection* is never that radio, whichever chip it sits under — a
        // client dialled straight at one of a station's other radios has one as
        // its first tab — and a connection whose radio has been closed at the
        // far end has to be able to go with it.
        if self.station_radio == Some(self.tabs[i].id) {
            return;
        }
        let mut tab = self.tabs.remove(i);
        tab.app.shutdown_ctrl();
        // Closing a connection hangs up and nothing more: it was never in this
        // machine's roster, and the station at the other end is not ours to
        // remove from anything. (The browser has no roster to remove from
        // either — see the rename above.)
        #[cfg(not(target_arch = "wasm32"))]
        if !tab.remote
            && let Err(e) = sdroxide_config::remove_radio(tab.id)
        {
            eprintln!("sdroxide: removing radio {} from the roster: {e}", tab.id);
        }
        crate::waterfall_gpu::retire(self.wgpu.as_ref(), u64::from(tab.id));
        // The removal shifted everything after `i` down by one.
        let was_focused = self.focused == i;
        if self.focused > i {
            self.focused -= 1;
        }
        if was_focused {
            self.focused = self.focused.min(self.tabs.len() - 1);
            self.tabs[self.focused].app.set_focused(true, ctx);
        }
        if self.tabs.len() == 1 {
            self.tabs[0].app.set_shared_log(false);
        }
        // Its pane goes with it (and the focus moves onto a visible radio).
        self.sanitize_panes(ctx);
    }
}

impl MultiApp {
    /// Bring a window that opened bigger than its screen back onto it.
    ///
    /// The size asked for at startup is in *points*, and a scaled display has
    /// fewer of them than its pixel count suggests: a 1920×1080 screen at 150%
    /// is 1280×720 points, and the 1280×800 window sdroxide asks for is then
    /// wider than the whole desktop and 80 points taller. What that looks like
    /// is the controls along the right-hand edge — the Setup gear at the end of
    /// the top bar among them — simply not being on the display (issue #234).
    ///
    /// Run once, on the first frame where the window manager has reported both
    /// the screen and the window, and it only ever makes the window smaller. An
    /// operator who has deliberately dragged a window past the edge of their
    /// display keeps it.
    #[cfg(not(target_arch = "wasm32"))]
    fn fit_window(&mut self, ctx: &egui::Context) {
        if self.fitted {
            return;
        }
        let (monitor, inner, outer) = ctx.input(|i| {
            let v = i.viewport();
            (v.monitor_size, v.inner_rect.map(|r| r.size()), v.outer_rect.map(|r| r.size()))
        });
        // Nothing is claimed until the window manager has answered. On a
        // desktop that never does, the check simply never runs — which is the
        // behaviour every release before this one had.
        let (Some(monitor), Some(inner), Some(outer)) = (monitor, inner, outer) else {
            return;
        };
        self.fitted = true;
        // The floor is the window's own minimum, as `gui_main` sets it: a
        // screen too small for that is one where something has to be cut off
        // either way.
        if let Some(want) =
            crate::layout::fit_inner_size(monitor, outer, inner, egui::vec2(800.0, 500.0))
        {
            tracing::info!(
                "window {}x{} does not fit a {}x{} point screen — bringing it in to {}x{}",
                inner.x.round(),
                inner.y.round(),
                monitor.x.round(),
                monitor.y.round(),
                want.x.round(),
                want.y.round(),
            );
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(want));
        }
    }
}

impl eframe::App for MultiApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        #[cfg(not(target_arch = "wasm32"))]
        self.fit_window(&ctx);
        let now = ctx.input(|i| i.time);
        self.sanitize_panes(&ctx);
        // Pane order → tab index; `sanitize_panes` guaranteed each exists.
        let pane_tabs: Vec<usize> =
            self.panes.iter().filter_map(|id| self.tabs.iter().position(|t| t.id == *id)).collect();
        // Hidden tabs first: their engines' unbounded event channels must not
        // back up, and their digital modes keep working in the background.
        // (The visible ones drain at the top of their own frame loop.)
        for (i, tab) in self.tabs.iter_mut().enumerate() {
            if !pane_tabs.contains(&i) {
                tab.app.drain_events(&ctx, now);
                // ...and let a hidden radio through a sign-in it can answer
                // by itself. A station asks each of its radios separately, so
                // without this the tabs behind the one on screen would each
                // wait at a challenge nobody is looking at.
                //
                // A tab waiting its turn — the station judges one sign-in at a
                // time — needs the frames to take it: nothing arrives on its
                // own socket while it waits, so without this it would sit at
                // the challenge until something else happened to redraw.
                if tab.app.poll_auth() {
                    crate::repaint::after_ms(&ctx, 120);
                }
                // A silence auto-split armed on this radio is a recording
                // decision, not a drawing one: it has to keep running while the
                // tab is behind another, or the files are split only for
                // whichever radio the operator happens to be looking at. Armed,
                // the gate asks for its own frames, so a monitor left on a
                // hidden tab still records a file per transmission.
                tab.app.poll_recording_gate(&ctx);
            }
        }
        self.share_spot_feed();
        // A link that has dropped is redialled here rather than on the error
        // screen, for the same reason: the screen belongs to one tab, and a
        // station's other radios have nobody looking at them to press anything.
        // Every tab is asked, the one on screen included — its own frame draws
        // the countdown but does not drive it.
        //
        // The clock is what releases these, so a window with nothing else
        // happening in it has to be woken to let them go.
        let mut waiting = false;
        for tab in &mut self.tabs {
            waiting |= tab.app.poll_reconnect();
        }
        if waiting {
            crate::repaint::after_ms(&ctx, 250);
        }
        // Publish the roster before the frame (the settings dialog draws it),
        // act on what the strips and the dialogs asked for after it. Which
        // radios have been lent out is settled first: both the strip and the
        // roster are drawn from it.
        self.refresh_attachments();
        self.refresh_power();
        let peers = self.tabs.len() > 1;
        if !peers {
            self.clip = None;
        }
        for t in &mut self.tabs {
            t.app.set_clip(peers, self.clip);
        }
        let roster = self.roster();
        let mut actions: Vec<StripAction> = Vec::new();
        let mut reqs: Vec<RadioTabRequest> = Vec::new();

        if pane_tabs.len() == 1 {
            if self.strip_wanted() {
                egui::Panel::top(egui::Id::new("radio-tab-strip"))
                    .frame(
                        // No bottom margin: the tabs' baseline is the top edge
                        // of the page below them, and a gap under it would
                        // leave the strip floating over the page instead.
                        egui::Frame::new()
                            .fill(crate::theme::BG_DEEP())
                            .inner_margin(egui::Margin { left: 8, right: 8, top: 3, bottom: 0 }),
                    )
                    .show(ui, |ui| self.strip_row(ui, 0, &mut actions));
            }
            let f = pane_tabs[0];
            self.tabs[f].app.set_radio_roster(roster);
            eframe::App::ui(&mut self.tabs[f].app, ui, frame);
            self.take_requests(f, &mut reqs);
        } else {
            // Split view: one equal column per pane, each under its own strip.
            let avail = ui.available_rect_before_wrap();
            let n = pane_tabs.len() as f32;
            let gap = 6.0;
            let col_w = ((avail.width() - gap * (n - 1.0)) / n).max(50.0);
            // Every row as tall as the tallest: the shorter ones get the
            // difference as room above their tabs, so the tabs still sit on
            // the page they switch.
            let tallest =
                self.strip_heights.iter().take(pane_tabs.len()).fold(0.0_f32, |a, &b| a.max(b));
            let mut heights: Vec<f32> = Vec::with_capacity(pane_tabs.len());
            for (k, &ti) in pane_tabs.iter().enumerate() {
                let left = avail.left() + k as f32 * (col_w + gap);
                let col = egui::Rect::from_min_max(
                    egui::pos2(left, avail.top()),
                    egui::pos2(left + col_w, avail.bottom()),
                );
                let id = self.tabs[ti].id;
                let mut pane_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(col)
                        .layout(egui::Layout::top_down(egui::Align::Min))
                        .id_salt(("radio-pane", id)),
                );
                pane_ui.shrink_clip_rect(col);
                // This pane's strip — switches what the pane shows.
                let pad =
                    (tallest - self.strip_heights.get(k).copied().unwrap_or(tallest)).max(0.0);
                let drawn = egui::Frame::new()
                    .fill(crate::theme::BG_DEEP())
                    .inner_margin(egui::Margin { left: 8, right: 8, top: 3, bottom: 0 })
                    .show(&mut pane_ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.add_space(pad);
                        self.strip_row(ui, k, &mut actions);
                    });
                heights.push(drawn.response.rect.height() - pad);
                self.tabs[ti].app.set_radio_roster(roster.clone());
                eframe::App::ui(&mut self.tabs[ti].app, &mut pane_ui, frame);
                self.take_requests(ti, &mut reqs);
                if k + 1 < pane_tabs.len() {
                    ui.painter().vline(
                        col.right() + gap * 0.5,
                        avail.y_range(),
                        egui::Stroke::new(1.0, crate::theme::PANEL()),
                    );
                }
            }
            if heights != self.strip_heights {
                // A row wrapped or unwrapped: even the rows out next frame.
                ctx.request_repaint();
                self.strip_heights = heights;
            }
            ui.allocate_rect(avail, egui::Sense::hover());
            // A press on a pane's background hands it the keyboard. Floating
            // windows live on layers above the background, so working in a
            // dialog that overhangs a neighbouring pane doesn't move focus.
            if ctx.input(|i| i.pointer.any_pressed())
                && let Some(pos) = ctx.input(|i| i.pointer.interact_pos())
                && ctx.layer_id_at(pos).is_none_or(|l| l.order == egui::Order::Background)
            {
                for (k, &ti) in pane_tabs.iter().enumerate() {
                    let left = avail.left() + k as f32 * (col_w + gap);
                    if ti != self.focused && pos.x >= left && pos.x < left + col_w + gap {
                        self.focus_tab(ti, &ctx);
                    }
                }
            }
        }
        // The undocked-module windows are the **shell's** to emit: once, for the
        // focused radio, every frame, whatever pane or tab is on screen. The app
        // says which windows it wants (`detached_wanted`); the shell emits those,
        // so a window that is wanted is always emitted — a tiling compositor
        // never sees it torn down and re-mapped when the radio or the mode
        // changes under it.
        #[cfg(not(target_arch = "wasm32"))]
        {
            let app = &mut self.tabs[self.focused].app;
            let want = app.detached_wanted();
            for module in sdroxide_types::DetachableModule::ALL {
                if want[module.index()] {
                    app.show_detached_module(module, &ctx, now);
                }
            }
        }
        self.apply_strip_actions(actions, &ctx);
        self.handle_requests(reqs, &ctx);
        self.open_peer_radios(&ctx);
        #[cfg(not(target_arch = "wasm32"))]
        self.remember_layout();
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        for tab in &mut self.tabs {
            eframe::App::save(&mut tab.app, storage);
        }
    }

    fn on_exit(&mut self) {
        // Joining every engine here is what lets each device close before
        // process teardown can race the C libraries' own exit handlers.
        for tab in &mut self.tabs {
            tab.app.shutdown_ctrl();
        }
    }
}

/// The panes, rows and focus a session opens with: the saved ones, held
/// against the radios there are now. A radio that has gone leaves its row; a
/// pane left with nothing to show goes; a radio the saved layout never saw
/// joins the first row. Nothing saved, or nothing left of it: one pane, every
/// radio on its row.
#[cfg(not(target_arch = "wasm32"))]
fn restore_layout(
    ids: &[u32],
    saved: &sdroxide_config::PaneLayout,
) -> (Vec<u32>, Vec<Vec<u32>>, Option<u32>) {
    let fresh = || (vec![ids[0]], vec![ids.to_vec()], None);
    let mut panes = Vec::new();
    let mut rows: Vec<Vec<u32>> = Vec::new();
    for (k, &shown) in saved.panes.iter().enumerate() {
        let row: Vec<u32> = saved
            .rows
            .get(k)
            .map_or_else(Vec::new, |r| r.iter().copied().filter(|id| ids.contains(id)).collect());
        if !ids.contains(&shown) && row.is_empty() {
            continue;
        }
        // A shown radio that has gone is left for `sanitize_panes`, which puts
        // the next radio on the row in its place.
        panes.push(shown);
        rows.push(row);
    }
    if panes.is_empty() {
        return fresh();
    }
    for &id in ids {
        if !saved.known.contains(&id)
            && !rows.iter().any(|r| r.contains(&id))
            && !panes.contains(&id)
        {
            rows[0].push(id);
        }
    }
    (panes, rows, saved.focused.filter(|id| ids.contains(id)))
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod layout_tests {
    use super::restore_layout;
    use sdroxide_config::PaneLayout;

    /// Two panes, a radio taken off with "−", the second pane focused: the
    /// next start opens on exactly that.
    #[test]
    fn a_saved_layout_comes_back() {
        let saved = PaneLayout {
            panes: vec![0, 2],
            rows: vec![vec![0, 1], vec![2]],
            focused: Some(2),
            known: vec![0, 1, 2, 3],
        };
        let (panes, rows, focus) = restore_layout(&[0, 1, 2, 3], &saved);
        assert_eq!(panes, vec![0, 2]);
        assert_eq!(rows, vec![vec![0, 1], vec![2]]);
        assert_eq!(focus, Some(2));
        assert!(!rows.iter().any(|r| r.contains(&3)), "radio 3 was taken off; it stays off");
    }

    /// Nothing saved is the strip as it always was: one pane, every radio.
    #[test]
    fn nothing_saved_is_one_pane_with_every_radio() {
        let (panes, rows, focus) = restore_layout(&[4, 0, 1], &PaneLayout::default());
        assert_eq!(panes, vec![4]);
        assert_eq!(rows, vec![vec![4, 0, 1]]);
        assert_eq!(focus, None);
    }

    /// A radio added since joins the first row; one closed since leaves its
    /// row, and a pane with nothing left goes with it.
    #[test]
    fn radios_added_and_closed_since() {
        let saved = PaneLayout {
            panes: vec![0, 5],
            rows: vec![vec![0], vec![5]],
            focused: Some(5),
            known: vec![0, 5],
        };
        let (panes, rows, focus) = restore_layout(&[0, 7], &saved);
        assert_eq!(panes, vec![0]);
        assert_eq!(rows, vec![vec![0, 7]]);
        assert_eq!(focus, None, "the focused radio is gone");
    }
}
