//! The sign-in screen a server that asks for a password puts up.
//!
//! Drawn *instead of* the radio rather than over it. There is nothing to draw
//! behind it — no capabilities, no state and no spectrum arrive until the
//! server accepts — and a screen full of empty meters with a dialog on top
//! would only invite the belief that the radio is there and unresponsive.
//!
//! Shared by both clients: the main window and, in the browser, the
//! solar-system tab, which is challenged by the same server for the same
//! reason.
//!
//! A station serves each of its radios on a connection of its own, and asks
//! each one for a password — so a client holding a whole station would face
//! one sign-in per radio. It does not: an answer a station has *accepted* is
//! kept for as long as the program runs ([`SESSION_LOGINS`]) and offered to
//! every other connection to that same station, so the operator signs in once
//! and the rest of the tabs let themselves in. Ticking "remember" is the
//! separate, longer-lived decision to keep it on this device between runs.
//!
//! Sign-ins go in one at a time ([`SIGNING_IN`]), because that is the only rate
//! a station will judge them at: it compares one answer at a time and tells
//! everybody else to come back. A station of four radios whose four tabs all
//! answered on the same frame would have three of them turned away — and a tab
//! that is not the one on screen never draws a card, so there is nobody there
//! to try again. The gate, and treating that "come back" as the non-verdict it
//! is ([`sdroxide_types::is_auth_busy`]), are what keep the rest of the
//! station's tabs from arriving at a challenge they cannot answer.
//!
//! *Every* answer takes the turn, the operator's own included
//! ([`LoginForm::manual`]). It used to be only the automatic ones, and the one
//! answer left outside the gate was the one somebody was watching: type a
//! password on the tab on screen while another tab of the same station lets
//! itself in, and the station compares one of the two and tells the other to
//! come back — which the card then drew in the refusal colour, under a password
//! that was perfectly good. That is issue #188's "it says the password is wrong
//! and then connects anyway".

use crate::i18n::tr;
use std::collections::BTreeMap;
use std::sync::{LazyLock, Mutex};

use eframe::egui::{self, RichText};
use sdroxide_types::{AuthPhase, RemoteAccess, is_auth_busy};

/// Sign-ins accepted this run, by station. Memory only, and never written
/// anywhere: what survives a restart is the "remember" store at the bottom of
/// this file, which is the operator's decision and not ours.
static SESSION_LOGINS: LazyLock<Mutex<BTreeMap<String, RemoteAccess>>> =
    LazyLock::new(|| Mutex::new(BTreeMap::new()));

/// Stations with an automatic sign-in with the server right now, and the
/// wall-clock second it went out at — the turn-taking gate described in the
/// module docs.
///
/// One entry at a time per station, so this screen never asks a station to
/// judge two answers at once. The timestamp is what stops a claim outliving
/// the connection that made it: a tab whose socket died mid-attempt is never
/// answered, so its claim is treated as abandoned after [`GATE_HOLD_S`] rather
/// than holding the rest of the station's tabs out for the rest of the run.
static SIGNING_IN: LazyLock<Mutex<BTreeMap<String, f64>>> =
    LazyLock::new(|| Mutex::new(BTreeMap::new()));

/// How long a claim on [`SIGNING_IN`] stands before the next tab takes its
/// turn anyway. Generously longer than a round trip and than the three-second
/// lockout a wrong answer costs, since the only thing on the other side of it
/// is a tab waiting a moment longer than it had to.
const GATE_HOLD_S: f64 = 15.0;

/// How many answers one connection offers without being asked before it gives
/// up and puts the card in front of the operator.
///
/// Only ever spent on answers that were never judged — a station that says
/// "come back" — so this is a backstop against a station that says nothing
/// else, not a retry budget for wrong passwords: those are spent on the first
/// try and never offered again.
const MAX_AUTO_ATTEMPTS: u8 = 8;

/// Which station an address belongs to: everything up to the endpoint, so the
/// several radios of one station (`/ws`, `/ws/1`, …) and its solar feed
/// (`/solar-ws`) come out the same, and a *different* station never does.
///
/// A password is answered to the station that asked for it and to nothing
/// else: two stations on one screen are two doors, however alike their
/// addresses look.
pub fn station_key(url: &str) -> String {
    let rest = url.trim_end_matches('/');
    match (rest.rfind("/ws"), rest.rfind("/solar-ws")) {
        (_, Some(i)) => rest[..i].to_string(),
        (Some(i), None) => rest[..i].to_string(),
        (None, None) => rest.to_string(),
    }
}

/// What the operator has typed, and what to do with it.
#[derive(Default)]
pub struct LoginForm {
    username: String,
    password: String,
    /// Whether to keep this sign-in on this device. Off by default: it writes
    /// a password to disk (or to the browser's local storage), which is the
    /// operator's call to make, not ours to make for them.
    remember: bool,
    /// Whether a stored sign-in has already been offered on this connection.
    /// Once only, so a stored password the server no longer accepts asks the
    /// operator instead of being posted back for ever.
    ///
    /// "This connection", not "this tab": a controller that reconnects keeps
    /// the form it had, so both this and the flag below are re-armed the moment
    /// a fresh challenge arrives ([`LoginForm::new_session`]). Without that a
    /// tab that had reconnected once could never let itself in again — and a
    /// tab nobody is looking at has no other way in.
    offered_stored: bool,
    /// Likewise for the sign-in another tab of this station made — offered
    /// once per connection, so a station that turns it down (the operator's
    /// access was withdrawn mid-session) asks this tab rather than looping.
    offered_session: bool,
    /// How many automatic answers this connection has spent, against
    /// [`MAX_AUTO_ATTEMPTS`]. Counted because the two flags above are put back
    /// whenever an answer comes back unjudged, and something has to stop that
    /// going round for ever.
    auto_attempts: u8,
    /// This form's claim on its station's [`SIGNING_IN`] gate, as the second
    /// it was made at, so it can give it up again when the server answers —
    /// and give up only its *own*, never the claim a later tab took over after
    /// this one was abandoned.
    holds_gate: Option<f64>,
    /// Whether the last look for an automatic answer was stopped by another
    /// tab of this station having the turn, rather than by there being nothing
    /// to offer — see [`LoginForm::waiting_for_turn`].
    waiting: bool,
    /// Which station this form belongs to, remembered from the last frame it
    /// was told: [`LoginForm::settle`] runs on the frame the answer is
    /// accepted, which is the frame that has to record it.
    station: String,
    /// Whether the attempt now being judged came out of storage, so a
    /// rejection can throw it away rather than leave a password behind that is
    /// known not to work.
    submitted_stored: bool,
    /// Whether the challenge was outstanding last frame, so a challenge
    /// arriving where there was none can be recognised as a *new* session on
    /// the same tab — which, after a reconnect, is the ordinary case.
    was_pending: bool,
    /// Whether the server was judging an answer of ours last frame, so the
    /// moment it accepts one can be noticed — that is when a sign-in is worth
    /// keeping, and not before.
    ///
    /// `Checking` rather than merely "pending": a connection that is dropped
    /// and redialled goes back to [`AuthPhase::Open`] with nothing having been
    /// accepted, and reading that as an acceptance would file the last thing
    /// typed — quite possibly the password the station had just refused — as
    /// this station's known-good sign-in, and write it to disk.
    was_checking: bool,
    /// What the operator typed and pressed SIGN IN on, waiting for this
    /// station's turn like every other answer.
    ///
    /// Held here rather than handed straight to the socket because a station
    /// judges one sign-in at a time: an answer that went out beside another
    /// tab's would be compared against nothing and come back
    /// [`sdroxide_types::AUTH_BUSY`], in front of the one person who would read
    /// that as a verdict. Kept across a "come back" — nothing was compared, so
    /// it is as good as it was — and dropped on a real verdict, which is the
    /// operator's to deal with.
    manual: Option<RemoteAccess>,
    /// Whether this sign-in is the operator's own answer to the card, rather
    /// than one the tab gave for itself.
    ///
    /// What it guards is the "remember" tick below, which is only an
    /// instruction when somebody was actually shown it: a tab that let itself
    /// in on another tab's sign-in never drew a card, so its `remember` is
    /// `false` because nobody was asked — not because anybody unticked
    /// anything. Acting on that is how a station of several radios erased the
    /// operator's remembered sign-in on every run, the second tab quietly
    /// forgetting what the first had just been let in with.
    answered_by_hand: bool,
    /// Whether the fields have been given the keyboard yet.
    focused: bool,
    /// The backdrop's state and the frame time it started at, on the screens
    /// that draw one. `None` on a phone, and until the first frame.
    sky: Option<(crate::login_globe::Sky, f64)>,
}

impl LoginForm {
    /// Notice the server's verdict. Call every frame, whatever the phase.
    ///
    /// The sign-in is written out on acceptance rather than on submission:
    /// storing what the operator typed before the server has agreed with it
    /// would leave a wrong password behind to be offered again next time.
    pub fn settle(&mut self, phase: &AuthPhase, station: &str) {
        if !station.is_empty() {
            self.station = station.to_string();
        }
        // The claim on the station's gate is held for exactly as long as an
        // answer is with the server, which is what `Checking` means. Anything
        // else — a verdict, an accepted sign-in, a socket that went away — is
        // the next tab's turn.
        if self.holds_gate.is_some() && !matches!(phase, AuthPhase::Checking) {
            self.release_gate();
        }
        // A station that was too busy to compare our answer did not refuse it:
        // it is as good as it was, and this connection has to be free to offer
        // it again. Nothing else here would — a tab that is not on screen never
        // draws a card, so "ask the operator" is not an answer for it.
        if let AuthPhase::Prompt(Some(why)) = phase {
            if is_auth_busy(why) {
                self.offered_stored = false;
                self.offered_session = false;
                self.submitted_stored = false;
            } else {
                // ...and a station that *did* compare it has answered the
                // operator, so their answer is spent. Posting it back would
                // loop the very password the station has just turned down.
                self.manual = None;
            }
        }
        if phase.is_pending() {
            // A challenge where a moment ago there was none: a new session on
            // the same tab, which after a reconnect is the ordinary case.
            if !self.was_pending {
                self.new_session();
            }
            self.was_pending = true;
            self.was_checking = matches!(phase, AuthPhase::Checking);
            return;
        }
        self.was_pending = false;
        if !std::mem::take(&mut self.was_checking) {
            return;
        }
        // Accepted. Every other tab of this station may use this answer for as
        // long as the program runs — including tabs that have not been asked
        // yet, which is most of them at the moment the first one gets in.
        if !self.station.is_empty() && !self.password.is_empty() {
            SESSION_LOGINS.lock().unwrap_or_else(|e| e.into_inner()).insert(
                self.station.clone(),
                RemoteAccess { username: self.username.clone(), password: self.password.clone() },
            );
        }
        if self.remember {
            store(&RemoteAccess {
                username: self.username.clone(),
                password: self.password.clone(),
            });
        } else if self.answered_by_hand {
            // Unticking it is an instruction, not an omission — but only from
            // somebody who was shown the box. See [`Self::answered_by_hand`].
            forget();
        }
        self.manual = None;
        self.password.clear();
        self.focused = false;
    }

    /// Put the once-per-connection state back, because there is a new
    /// connection: the controller reconnects in place and keeps this form.
    ///
    /// What is deliberately *not* reset is the station key, the "remember" tick
    /// and the answer the operator typed ([`Self::manual`]) — all three belong
    /// to the operator rather than to the socket, and a password that was in
    /// flight when the link dropped is still the right answer to the challenge
    /// the new one arrives at.
    fn new_session(&mut self) {
        self.offered_stored = false;
        self.offered_session = false;
        self.submitted_stored = false;
        self.answered_by_hand = false;
        self.auto_attempts = 0;
        self.waiting = false;
        self.focused = false;
    }

    /// Take this station's turn at signing in, if it is going.
    fn claim_gate(&mut self, now: f64) -> bool {
        if self.holds_gate.is_some() || self.station.is_empty() {
            return true;
        }
        let mut gate = SIGNING_IN.lock().unwrap_or_else(|e| e.into_inner());
        // Somebody else's attempt, and recent enough to still be alive. `now`
        // is a wall clock, so a step backwards is treated as a live claim
        // rather than as an ancient one.
        if let Some(&at) = gate.get(&self.station)
            && now >= at
            && now - at < GATE_HOLD_S
        {
            return false;
        }
        gate.insert(self.station.clone(), now);
        self.holds_gate = Some(now);
        true
    }

    /// Give the turn back — but only if it is still ours. A claim this form
    /// abandoned may since have been taken over by another tab, and pulling
    /// that one out would let a third go at the same time.
    fn release_gate(&mut self) {
        let Some(mine) = self.holds_gate.take() else { return };
        let mut gate = SIGNING_IN.lock().unwrap_or_else(|e| e.into_inner());
        if gate.get(&self.station) == Some(&mine) {
            gate.remove(&self.station);
        }
    }

    /// The turning globe behind the card.
    ///
    /// Tablets and desktops only. A phone is the one screen where the card is
    /// nearly the whole window, so there would be little of the globe to see —
    /// and the most likely device to be running on a battery, over a link that
    /// is already the reason this screen is up. There it stays flat.
    fn backdrop(
        &mut self,
        ui: &mut egui::Ui,
        tier: crate::layout::Tier,
        render_state: Option<&eframe::egui_wgpu::RenderState>,
    ) {
        if tier == crate::layout::Tier::Phone {
            self.sky = None;
            return;
        }
        let Some(rs) = render_state else { return };
        crate::login_globe::ensure(rs);

        let rect = ui.max_rect();
        if rect.width() < 1.0 || rect.height() < 1.0 {
            return;
        }
        let now = ui.input(|i| i.time);
        let sky = self.sky.get_or_insert_with(|| (crate::login_globe::Sky::default(), now));
        sky.0.step(now);
        // Faded up over the first second: the pipelines and the two maps are
        // built on the frame this is first asked for, so without it the screen
        // cuts from flat black to a lit globe.
        let alpha = (((now - sky.1) / 1.0) as f32).clamp(0.0, 1.0);

        ui.painter().add(crate::egui_wgpu::Callback::new_paint_callback(
            rect,
            crate::login_globe::callback(&sky.0, [rect.width(), rect.height()], alpha),
        ));
        // Nothing else drives this screen's repaints — a socket that is waiting
        // for a password sends nothing — so the globe has to ask for its own.
        crate::repaint::animate(ui.ctx());
    }

    /// The answer this connection has ready, if the station's turn is going:
    /// the one the operator typed ([`Self::manual`]), the sign-in another tab
    /// of this station has already had accepted, or the one kept on this
    /// device. `None` means either that the operator has to be asked, or that
    /// another tab of this station is mid-attempt and it is not this one's turn
    /// yet — [`LoginForm::waiting_for_turn`] tells the two apart.
    ///
    /// The operator's own answer comes first and is not held to
    /// [`answerable`]: they are answering a card that is showing them a
    /// refusal, which is exactly the phase an automatic answer must *not* keep
    /// posting into.
    ///
    /// Public because a tab that is not on screen never draws the sign-in and
    /// still has to be able to let itself in — see `SdroxideApp::poll_auth`.
    pub fn answer_without_asking(&mut self, phase: &AuthPhase) -> Option<RemoteAccess> {
        self.waiting = false;
        // Nothing goes out while the server is judging one, or where there is
        // no challenge to answer.
        let asked = matches!(phase, AuthPhase::Prompt(_));
        let by_hand = asked && self.manual.is_some();
        if self.auto_attempts >= MAX_AUTO_ATTEMPTS || (!by_hand && !answerable(phase)) {
            return None;
        }
        if !self.claim_gate(crate::time::now_unix_f64()) {
            // Another tab of this station is mid-attempt. Whether there was
            // anything to offer is not asked until the turn is ours, so this
            // only says "come back", which is what the flag is for.
            self.waiting = true;
            return None;
        }
        let answer = match self.manual.clone() {
            // Kept rather than taken: a station too busy to compare it has to
            // be offered it again, and `settle` is what drops it on a verdict.
            // The tick on the card speaks for the operator from here on, which
            // is what this is — see [`Self::answered_by_hand`].
            Some(a) if asked => {
                self.answered_by_hand = true;
                Some(a)
            }
            _ => self.session_answer(phase).or_else(|| self.stored_answer(phase)),
        };
        match answer {
            // Spent, and the turn is used: `settle` gives the gate back when
            // the server answers.
            Some(_) => self.auto_attempts += 1,
            // Nothing to offer, so nothing is waiting on the station — hand
            // the turn straight back rather than making the next tab sit out
            // the whole hold.
            None => self.release_gate(),
        }
        answer
    }

    /// Whether the last [`LoginForm::answer_without_asking`] was turned back
    /// only because another tab of the same station had the turn.
    ///
    /// The shell reads it to keep asking for frames: a tab that is not on
    /// screen has no other reason to be redrawn, and its turn comes round on
    /// somebody else's socket.
    pub fn waiting_for_turn(&self) -> bool {
        self.waiting
    }

    /// The answer another tab of this station has already had accepted, if
    /// there is one. Offered once per connection, and ahead of the stored
    /// sign-in: it is the one this station is known to be taking *now*.
    fn session_answer(&mut self, phase: &AuthPhase) -> Option<RemoteAccess> {
        if !answerable(phase) || self.offered_session {
            return None;
        }
        let known =
            SESSION_LOGINS.lock().unwrap_or_else(|e| e.into_inner()).get(&self.station).cloned()?;
        self.offered_session = true;
        self.username = known.username.clone();
        self.password = known.password.clone();
        // Not `submitted_stored`: this did not come out of storage, so a
        // refusal must not erase the operator's saved sign-in.
        Some(known)
    }

    /// What was stored, offered once against the first challenge of a
    /// connection.
    fn stored_answer(&mut self, phase: &AuthPhase) -> Option<RemoteAccess> {
        if !answerable(phase) || self.offered_stored {
            return None;
        }
        self.offered_stored = true;
        let stored = load_stored()?;
        self.username = stored.username.clone();
        self.password = stored.password.clone();
        self.remember = true;
        self.submitted_stored = true;
        Some(stored)
    }
}

/// Whether the server is waiting for an answer that may be given without
/// troubling the operator.
///
/// A fresh challenge, plainly — and also one whose last answer came back
/// unjudged, because the station was busy comparing somebody else's. That is
/// not a refusal (see [`sdroxide_types::AUTH_BUSY`]), so the same answer is
/// still the right one to give; anything else the station says is a verdict on
/// the answer, and a verdict is the operator's to deal with.
fn answerable(phase: &AuthPhase) -> bool {
    match phase {
        AuthPhase::Prompt(None) => true,
        AuthPhase::Prompt(Some(why)) => is_auth_busy(why),
        AuthPhase::Open | AuthPhase::Checking => false,
    }
}

/// Draw the sign-in for one frame. `Some` is the answer to send to the server.
///
/// Call only while [`AuthPhase::is_pending`]; the caller draws nothing else.
/// `render_state` is what the backdrop is built from; pass `None` and the
/// screen is drawn flat.
pub fn screen(
    ui: &mut egui::Ui,
    form: &mut LoginForm,
    phase: &AuthPhase,
    render_state: Option<&eframe::egui_wgpu::RenderState>,
) -> Option<RemoteAccess> {
    // A tab of a station somebody has already signed in to — or a sign-in
    // kept on this device — lets itself in without troubling the operator.
    if let Some(known) = form.answer_without_asking(phase) {
        return Some(known);
    }
    let refused = match phase {
        AuthPhase::Prompt(why) => why.as_deref(),
        _ => None,
    };
    // A station too busy to compare it has not turned it down: it said nothing
    // about the password at all, and throwing away a working sign-in on the
    // strength of that is how a station of several radios loses the one the
    // operator ticked "remember" for.
    let turned_down = refused.is_some_and(|why| !is_auth_busy(why));
    if turned_down && std::mem::take(&mut form.submitted_stored) {
        // The stored sign-in was turned down, so it is no use to anybody —
        // least of all next time, when it would be offered again and fail
        // again before the operator was ever asked.
        forget();
    }
    // Waiting for a turn at the station is a wait, not a question: this
    // connection has an answer and is going to give it as soon as the turn
    // comes round, so the card says so rather than putting an empty box and a
    // red line in front of an operator with nothing to do.
    //
    // The "come back" is filtered out whether or not *this* form is the one
    // waiting: a station judges one sign-in at a time across every client on
    // it, so an answer can come back unjudged with nothing here to blame for
    // it — and a sentence about somebody else's sign-in, in the colour a
    // refusal is drawn in, reads as "wrong password" to the one person looking
    // at it.
    let waiting = form.waiting_for_turn();
    let refused = refused.filter(|why| !is_auth_busy(why));
    let checking = matches!(phase, AuthPhase::Checking) || waiting || form.manual.is_some();
    let tier = crate::layout::tier(ui.ctx());
    let touch = tier.touch();

    form.backdrop(ui, tier, render_state);

    // The card is sized to the screen it is on, not to a number: 360 pt of
    // phone leaves 328 once the margins are off, and a fixed 320-wide card
    // would sit in a 4 pt gutter — or overflow the moment the browser's own
    // chrome takes a few points more.
    let avail = ui.available_size();
    let card_w = (avail.x - 24.0).clamp(200.0, 360.0);
    // A little above centre, which is where a sign-in belongs and where the
    // globe's open sky is. On a screen too short to centre anything the
    // arithmetic goes negative and the floor puts the card at the top, which is
    // what keeps the fields clear of a phone's on-screen keyboard.
    let lead = ((avail.y - card_height(touch)) * 0.40).max(8.0);

    let mut submit = false;
    // A landscape phone is 393 pt tall and this card is most of that; the
    // scroll area is what stops the button being somewhere the finger cannot
    // reach when the keyboard is up.
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(lead);
            egui::Frame::new()
                .fill(crate::theme::PANEL().gamma_multiply(0.94))
                .stroke(egui::Stroke::new(1.0, crate::theme::LINE_LIT()))
                .inner_margin(egui::Margin::symmetric(if touch { 16 } else { 20 }, 16))
                .show(ui, |ui| {
                    ui.set_width(card_w);
                    ui.vertical(|ui| {
                        card(ui, form, refused, checking, touch, &mut submit);
                    });
                });
            ui.add_space(lead);
        });
    });

    // An empty password is never worth a round trip: the server would refuse
    // it and shut the door for three seconds, which the operator would then
    // have to sit out for the attempt they meant to make.
    if !submit || checking || form.password.is_empty() {
        return None;
    }
    form.submitted_stored = false;
    // A fresh intent, so the automatic budget starts again: what it bounds is
    // answers a station never compared, and the operator pressing the button is
    // not one of those.
    form.auto_attempts = 0;
    form.manual =
        Some(RemoteAccess { username: form.username.clone(), password: form.password.clone() });
    // Out now if this station's turn is free, and on a later frame if it is
    // not. Either way it leaves through the one gate every other answer does.
    form.answer_without_asking(phase)
}

/// Roughly how tall the card comes out, for placing it before it is laid out.
/// Only used to centre it, so being a few points out costs nothing.
fn card_height(touch: bool) -> f32 {
    if touch { 300.0 } else { 260.0 }
}

/// The card's contents.
fn card(
    ui: &mut egui::Ui,
    form: &mut LoginForm,
    refused: Option<&str>,
    checking: bool,
    touch: bool,
    submit: &mut bool,
) {
    let field_h = if touch { 30.0 } else { 22.0 };

    ui.label(RichText::new(tr!("login-sdr-oxide")).size(17.0).strong().color(crate::theme::CYAN()));
    ui.add_space(12.0);

    ui.add_enabled_ui(!checking, |ui| {
        ui.label(RichText::new(tr!("login-username")).size(11.5).weak());
        let user = crate::chrome::field_sized(
            ui,
            [ui.available_width(), field_h],
            egui::TextEdit::singleline(&mut form.username),
        );
        ui.add_space(8.0);
        ui.label(RichText::new(tr!("login-password")).size(11.5).weak());
        let pass = crate::chrome::field_sized(
            ui,
            [ui.available_width(), field_h],
            egui::TextEdit::singleline(&mut form.password).password(true),
        );
        // The keyboard lands in the first empty box, so a remembered username
        // can be typed past without reaching for the mouse. Not done on a
        // touched layout: focusing a field there raises the on-screen keyboard
        // over the card before the operator has asked for it.
        if !form.focused && !touch {
            form.focused = true;
            if form.username.is_empty() {
                user.request_focus();
            } else {
                pass.request_focus();
            }
        }
        // Enter in either box is the button.
        let entered = (user.lost_focus() || pass.lost_focus())
            && ui.input(|i| i.key_pressed(egui::Key::Enter));
        *submit |= entered;
    });

    ui.add_space(10.0);
    // The wording is the hint on a touched layout: there is no pointer to hover
    // with, so a tooltip there is a sentence nobody will ever read.
    let remember = crate::chrome::checkbox(ui, &mut form.remember, tr!("login-remember-device"));
    if !touch {
        remember.on_hover_text(tr!("login-signs-without-asking-next-time"));
    } else {
        ui.label(
            RichText::new(tr!("login-keeps-password-device-clear"))
                .size(10.5)
                .color(crate::theme::gray(140)),
        );
    }

    ui.add_space(12.0);
    ui.horizontal(|ui| {
        let label = if checking { tr!("login-checking") } else { tr!("login-sign") };
        let button = crate::chrome::chip_accent(
            ui,
            false,
            RichText::new(label).strong().size(if touch { 15.0 } else { 13.0 }),
            crate::theme::GREEN(),
            crate::theme::INK_ON_CYAN(),
        );
        *submit |= button.clicked() && !checking;
    });

    if let Some(why) = refused {
        ui.add_space(10.0);
        ui.label(RichText::new(why).size(12.0).color(crate::theme::ALERT()));
    } else if checking {
        ui.add_space(10.0);
        ui.label(
            RichText::new(tr!("login-wrong-password-shuts-server-s"))
                .size(11.0)
                .color(crate::theme::gray(140)),
        );
    }
}

// ── Where a remembered sign-in lives ─────────────────────────────────────────
//
// Native: `remote_login.json` in the config directory, beside the rest of this
// client's own state. Browser: local storage, which the solar tab shares
// because it shares an origin — that is what makes one sign-in do for both.
//
// In the clear, both of them. So is every other credential sdroxide stores (the
// cluster login, the QRZ and eQSL passwords), the box that writes it says so,
// and the manual says so; encrypting it would need a key kept beside it, which
// is not an improvement worth the impression of one.

#[cfg(not(target_arch = "wasm32"))]
fn load_stored() -> Option<RemoteAccess> {
    sdroxide_config::load_remote_login()
}

#[cfg(not(target_arch = "wasm32"))]
fn store(login: &RemoteAccess) {
    if let Err(e) = sdroxide_config::save_remote_login(Some(login)) {
        eprintln!("failed to remember the sign-in: {e}");
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn forget() {
    if let Err(e) = sdroxide_config::save_remote_login(None) {
        eprintln!("failed to forget the sign-in: {e}");
    }
}

#[cfg(target_arch = "wasm32")]
const USER_KEY: &str = "sdroxide_login_user";
#[cfg(target_arch = "wasm32")]
const PASS_KEY: &str = "sdroxide_login_pass";

/// Two keys rather than one encoded value: nothing to escape, and nothing that
/// can be mis-split by a password that happens to contain the separator.
#[cfg(target_arch = "wasm32")]
fn local_storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}

#[cfg(target_arch = "wasm32")]
fn load_stored() -> Option<RemoteAccess> {
    let store = local_storage()?;
    let login = RemoteAccess {
        username: store.get_item(USER_KEY).ok().flatten().unwrap_or_default(),
        password: store.get_item(PASS_KEY).ok().flatten().unwrap_or_default(),
    };
    login.is_enforced().then_some(login)
}

#[cfg(target_arch = "wasm32")]
fn store(login: &RemoteAccess) {
    let Some(store) = local_storage() else { return };
    let _ = store.set_item(USER_KEY, &login.username);
    let _ = store.set_item(PASS_KEY, &login.password);
}

#[cfg(target_arch = "wasm32")]
fn forget() {
    let Some(store) = local_storage() else { return };
    let _ = store.remove_item(USER_KEY);
    let _ = store.remove_item(PASS_KEY);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A server that asks for nothing must never make the form do anything —
    /// in particular it must not write or erase a stored sign-in belonging to
    /// some other server the operator uses.
    #[test]
    fn a_local_engine_settles_to_nothing() {
        let mut form = LoginForm { remember: true, ..LoginForm::default() };
        for _ in 0..5 {
            form.settle(&AuthPhase::Open, "");
        }
        assert!(!form.was_pending, "nothing was ever pending, so nothing was decided");
    }

    /// The transition that matters: pending, then accepted. Only then is there
    /// a sign-in worth keeping.
    #[test]
    fn acceptance_is_what_ends_the_exchange() {
        let mut form = LoginForm::default();
        form.settle(&AuthPhase::Prompt(None), "");
        assert!(form.was_pending);
        form.settle(&AuthPhase::Checking, "");
        assert!(form.was_pending);
        form.settle(&AuthPhase::Open, "");
        assert!(!form.was_pending, "the verdict was acted on exactly once");
        // ...and not again on every subsequent frame.
        form.settle(&AuthPhase::Open, "");
        assert!(!form.was_pending);
    }

    /// A station asks each of its radios' connections for the password
    /// separately. The operator answers once: the tab that got in leaves the
    /// answer where the others find it.
    #[test]
    fn one_sign_in_gets_the_stations_other_tabs_in() {
        let station = "ws://one-sign-in.test:4950";
        let mut first = LoginForm {
            username: "oe3jjs".into(),
            password: "hunter2".into(),
            ..LoginForm::default()
        };
        first.settle(&AuthPhase::Prompt(None), station);
        first.settle(&AuthPhase::Checking, station);
        first.settle(&AuthPhase::Open, station);

        // The second radio's tab, challenged afterwards, answers by itself.
        let mut second = LoginForm { station: station.to_string(), ..LoginForm::default() };
        let answer = second
            .session_answer(&AuthPhase::Prompt(None))
            .expect("the station's sign-in was not offered to its other tab");
        assert_eq!(answer.username, "oe3jjs");
        assert_eq!(answer.password, "hunter2");
        // Once only: a station that turns it down asks this tab rather than
        // posting the same answer back for ever.
        assert!(second.session_answer(&AuthPhase::Prompt(None)).is_none());

        // ...and a *different* station is a different door.
        let mut elsewhere =
            LoginForm { station: "ws://elsewhere.test:4950".into(), ..LoginForm::default() };
        assert!(
            elsewhere.session_answer(&AuthPhase::Prompt(None)).is_none(),
            "one station's password was offered to another"
        );
    }

    /// Every connection to one station — each radio, and the solar feed —
    /// resolves to the same door, and no other station's does.
    #[test]
    fn a_stations_addresses_share_one_key() {
        let station = "ws://shack.test:4950";
        assert_eq!(station_key("ws://shack.test:4950/ws"), station);
        assert_eq!(station_key("ws://shack.test:4950/ws/10"), station);
        assert_eq!(station_key("ws://shack.test:4950/solar-ws"), station);
        assert_eq!(station_key("ws://shack.test:4950/"), station);
        assert_eq!(station_key("ws://shack.test:4950"), station);
        // A path prefix belongs to the address, so two stations behind one
        // reverse proxy stay two stations.
        assert_eq!(station_key("wss://host/a/ws/1"), "wss://host/a");
        assert_ne!(station_key("wss://host/a/ws/1"), station_key("wss://host/b/ws/1"));
    }

    /// The password is dropped once the server has taken it: keeping it in the
    /// form would leave it in memory for the rest of the session, and put it
    /// back in the box if the link dropped and the dialog returned.
    #[test]
    fn the_password_does_not_linger_after_a_successful_sign_in() {
        let mut form = LoginForm { password: "hunter2".into(), ..LoginForm::default() };
        form.settle(&AuthPhase::Prompt(None), "ws://lingering.test:4950");
        form.settle(&AuthPhase::Checking, "ws://lingering.test:4950");
        form.settle(&AuthPhase::Open, "ws://lingering.test:4950");
        assert!(form.password.is_empty());
    }

    /// A stored sign-in is offered against the first challenge and only the
    /// first: if the server turns it down, the operator is asked.
    #[test]
    fn a_stored_sign_in_is_offered_once_per_connection() {
        let mut form = LoginForm::default();
        // Whatever is (or is not) on this machine, the second ask must not
        // produce an answer — that is the loop this guards against.
        form.stored_answer(&AuthPhase::Prompt(None));
        assert!(form.offered_stored);
        assert!(form.stored_answer(&AuthPhase::Prompt(None)).is_none());
        // Nor does a rejection re-open it.
        assert!(form.stored_answer(&AuthPhase::Prompt(Some("no".into()))).is_none());
    }

    /// A form that can answer by itself without ever reaching this machine's
    /// real `remote_login.json`: `offered_stored` is spent up front, so only
    /// the session answer below is ever in play.
    fn tab(station: &str) -> LoginForm {
        LoginForm { station: station.into(), offered_stored: true, ..LoginForm::default() }
    }

    fn known(station: &str, password: &str) {
        SESSION_LOGINS.lock().unwrap().insert(
            station.to_string(),
            RemoteAccess { username: "oe3jjs".into(), password: password.into() },
        );
    }

    /// The one that shut whole stations out (issue #176): a station judges one
    /// sign-in at a time and tells everybody else to come back, and "come
    /// back" is not a refusal. A tab that spent its answer on one would sit at
    /// the challenge for the rest of the run — and a tab that is not the one on
    /// screen never draws a card, so there is nobody to ask.
    #[test]
    fn an_unjudged_answer_is_offered_again() {
        let station = "ws://busy.test:4950";
        known(station, "hunter2");
        let mut form = tab(station);

        form.settle(&AuthPhase::Prompt(None), station);
        assert!(form.answer_without_asking(&AuthPhase::Prompt(None)).is_some());
        form.settle(&AuthPhase::Checking, station);

        // "Another sign-in is being checked" — nothing was compared.
        let busy = AuthPhase::Prompt(Some(sdroxide_types::AUTH_BUSY.into()));
        form.settle(&busy, station);
        let again = form.answer_without_asking(&busy).expect("the answer was never judged");
        assert_eq!(again.password, "hunter2");
    }

    /// ...but a station that actually turned the answer down is answered by
    /// the operator, not by posting the same password back for ever.
    #[test]
    fn a_refusal_is_final() {
        let station = "ws://refused.test:4950";
        known(station, "hunter2");
        let mut form = tab(station);

        form.settle(&AuthPhase::Prompt(None), station);
        assert!(form.answer_without_asking(&AuthPhase::Prompt(None)).is_some());
        form.settle(&AuthPhase::Checking, station);

        let no = AuthPhase::Prompt(Some(sdroxide_types::AUTH_REFUSED.into()));
        form.settle(&no, station);
        assert!(form.answer_without_asking(&no).is_none());
    }

    /// A station that says nothing but "come back" still ends up in front of
    /// the operator rather than looping for ever.
    #[test]
    fn the_automatic_answers_run_out() {
        let station = "ws://always-busy.test:4950";
        known(station, "hunter2");
        let busy = AuthPhase::Prompt(Some(sdroxide_types::AUTH_BUSY.into()));
        let mut form = tab(station);
        form.settle(&AuthPhase::Prompt(None), station);

        for _ in 0..MAX_AUTO_ATTEMPTS {
            assert!(form.answer_without_asking(&busy).is_some());
            form.settle(&AuthPhase::Checking, station);
            form.settle(&busy, station);
        }
        assert!(form.answer_without_asking(&busy).is_none(), "this could go round for ever");
    }

    /// A controller reconnects in place and keeps its form, so the
    /// once-per-connection flags have to come back with the new connection.
    /// Without this a tab that had dropped once could never let itself in
    /// again — and a hidden one has no other way in.
    #[test]
    fn a_reconnect_re_arms_the_automatic_answer() {
        let station = "ws://redial.test:4950";
        known(station, "hunter2");
        let mut form = tab(station);

        form.settle(&AuthPhase::Prompt(None), station);
        assert!(form.answer_without_asking(&AuthPhase::Prompt(None)).is_some());
        form.settle(&AuthPhase::Checking, station);
        // The link goes: the controller resets to `Open` and redials. Nothing
        // was accepted, so nothing is filed as this station's known-good
        // sign-in either.
        form.settle(&AuthPhase::Open, station);
        assert!(!form.was_checking);
        // ...and the fresh challenge is answered without the operator.
        form.settle(&AuthPhase::Prompt(None), station);
        assert!(form.answer_without_asking(&AuthPhase::Prompt(None)).is_some());
    }

    /// One station, one attempt in flight. Four radios' tabs answering on the
    /// same frame is exactly what a station turns three of away for.
    #[test]
    fn one_tab_of_a_station_signs_in_at_a_time() {
        let station = "ws://turnstile.test:4950";
        known(station, "hunter2");
        let (mut a, mut b) = (tab(station), tab(station));
        a.settle(&AuthPhase::Prompt(None), station);
        b.settle(&AuthPhase::Prompt(None), station);

        assert!(a.answer_without_asking(&AuthPhase::Prompt(None)).is_some());
        assert!(b.answer_without_asking(&AuthPhase::Prompt(None)).is_none(), "both went at once");
        assert!(b.waiting_for_turn(), "the second tab has to be looked at again");

        // The first one's attempt is answered, and the turn passes.
        a.settle(&AuthPhase::Checking, station);
        a.settle(&AuthPhase::Open, station);
        assert!(b.answer_without_asking(&AuthPhase::Prompt(None)).is_some());
        assert!(!b.waiting_for_turn());
        b.settle(&AuthPhase::Checking, station);
        b.settle(&AuthPhase::Open, station);
        assert!(SIGNING_IN.lock().unwrap().get(station).is_none(), "the turn was never given back");
    }

    /// The other half of issue #176: a tab that let itself in on another tab's
    /// sign-in was never shown the "remember" box, so its unticked state is
    /// nobody's instruction — and acting on it erased the sign-in the operator
    /// had ticked it for, every run, as soon as the station's second radio
    /// connected.
    #[test]
    fn a_tab_that_was_never_asked_does_not_forget_the_stored_sign_in() {
        let station = "ws://remembering.test:4950";
        known(station, "hunter2");
        let mut form = tab(station);
        form.settle(&AuthPhase::Prompt(None), station);
        assert!(form.answer_without_asking(&AuthPhase::Prompt(None)).is_some());
        form.settle(&AuthPhase::Checking, station);
        form.settle(&AuthPhase::Open, station);
        assert!(
            !form.answered_by_hand,
            "nobody was shown the card, so nothing here speaks for the operator"
        );
        assert!(!form.remember);
    }

    /// A form holding what somebody typed and pressed SIGN IN on, without
    /// going through `screen` — which needs an `egui::Ui`.
    fn typed(station: &str, password: &str) -> LoginForm {
        LoginForm {
            manual: Some(RemoteAccess { username: "oe3jjs".into(), password: password.into() }),
            ..tab(station)
        }
    }

    /// Issue #188: the operator's own answer takes the station's turn like
    /// every other one. It used to go straight out, so a tab letting itself in
    /// beside it had both compared at once — and a station compares one, which
    /// left the person watching looking at "another sign-in is being checked"
    /// where they expected their radio.
    #[test]
    fn a_typed_answer_takes_the_stations_turn() {
        let station = "ws://typed-turn.test:4950";
        known(station, "hunter2");
        let (mut other, mut mine) = (tab(station), typed(station, "correct-horse"));
        other.settle(&AuthPhase::Prompt(None), station);
        mine.settle(&AuthPhase::Prompt(None), station);

        // The other tab has the turn, so the typed answer waits for it rather
        // than being compared alongside.
        assert!(other.answer_without_asking(&AuthPhase::Prompt(None)).is_some());
        assert!(
            mine.answer_without_asking(&AuthPhase::Prompt(None)).is_none(),
            "both went at once"
        );
        assert!(mine.waiting_for_turn());

        other.settle(&AuthPhase::Checking, station);
        other.settle(&AuthPhase::Open, station);
        let answer = mine.answer_without_asking(&AuthPhase::Prompt(None)).expect("the turn passed");
        assert_eq!(answer.password, "correct-horse", "the station's answer won over the typed one");
        assert!(mine.answered_by_hand, "the operator was the one who answered");
    }

    /// A typed answer is offered against a *refusal* — that is the card the
    /// operator is looking at — where an automatic one never is, and it
    /// survives a "come back" for the same reason a stored one does.
    #[test]
    fn a_typed_answer_answers_a_refusal_and_outlives_a_busy_station() {
        let station = "ws://typed-again.test:4950";
        let no = AuthPhase::Prompt(Some(sdroxide_types::AUTH_REFUSED.into()));
        let busy = AuthPhase::Prompt(Some(sdroxide_types::AUTH_BUSY.into()));
        let mut form = typed(station, "hunter2");

        // The refusal on screen is what was typed over, so it must not stop
        // the answer going out.
        assert!(form.answer_without_asking(&no).is_some());
        form.settle(&AuthPhase::Checking, station);

        // Unjudged: the same answer is still the right one.
        form.settle(&busy, station);
        assert_eq!(form.answer_without_asking(&busy).map(|a| a.password), Some("hunter2".into()));
        form.settle(&AuthPhase::Checking, station);

        // Judged, and turned down: the operator's, not ours, to answer again.
        form.settle(&no, station);
        assert!(form.answer_without_asking(&no).is_none(), "a refused password was posted back");
    }

    /// A claim on the gate cannot outlive the connection that made it: a tab
    /// whose socket died mid-attempt is never answered, and the rest of the
    /// station must not be held out behind it.
    #[test]
    fn an_abandoned_turn_is_taken_back() {
        let station = "ws://abandoned.test:4950";
        let mut stuck = tab(station);
        assert!(stuck.claim_gate(1000.0));
        let mut next = tab(station);
        assert!(!next.claim_gate(1000.0 + GATE_HOLD_S / 2.0));
        assert!(next.claim_gate(1000.0 + GATE_HOLD_S + 1.0));
        // The abandoned tab coming back to life must not pull the claim out
        // from under the tab that took over.
        stuck.release_gate();
        assert!(!tab(station).claim_gate(1000.0 + GATE_HOLD_S + 2.0));
        next.release_gate();
        assert!(SIGNING_IN.lock().unwrap().get(station).is_none());
    }
}
