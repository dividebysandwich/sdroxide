//! Server mode: HTTP (static WASM client) + a WebSocket session per radio,
//! speaking `sdroxide-proto`.
//!
//! One session per radio, and one client per session: `/ws` is the station's
//! first radio — all a single-radio station has, and all a client that knows
//! nothing of a roster ever asks for — while `/ws/<id>` names any of them.
//! `/radios` lists what is on offer.
//!
//! Lanes into the socket (per the project plan):
//! - control/state/memories/meters: reliable queue, never intentionally dropped
//! - spectrum: latest-wins watch channel (slow socket → lower fps, no lag)
//! - audio: small bounded queue, drops blocks when the socket can't keep up
//!
//! A pump thread bridges the engine's sync endpoints (crossbeam events,
//! triple-buffered spectrum, rtrb audio) into those async lanes and caches
//! the latest state/caps/memories so a new session can be greeted instantly.

mod auth;
mod probe;
mod session;
mod solar;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::Router;
use axum::routing::get;
use tokio::sync::{mpsc, watch};
use tracing::{info, warn};

use sdroxide_proto::solar::SolarServerMsg;
use sdroxide_proto::{AudioCodec, ServerMsg};
use sdroxide_types::{
    Command, DeviceCaps, DigiStatus, MemoryChannel, RadioEvent, RadioState, RdsData, SpectrumFrame,
};

#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    #[error("I/O: {0}")]
    Io(#[from] std::io::Error),
    /// A station with no radio has nothing to serve, and `/ws` would have
    /// nothing to answer with. The roster on disk always has at least one, so
    /// this is a caller bug rather than something an operator can configure.
    #[error("the server was given no radios to serve")]
    NoRadios,
}

/// How the server finds out who may connect.
///
/// A function rather than a value because it is called afresh for every
/// connection and every sign-in attempt: the credentials are a file on this
/// machine, and an operator who changes their password — by hand, or from the
/// settings dialog of a GUI running beside the server — expects that to hold
/// without restarting the server and dropping whoever is on it.
///
/// `None`, or a [`RemoteAccess`](sdroxide_types::RemoteAccess) with both fields
/// empty, leaves the server open to anyone who can reach the port.
pub type AccessFn = Box<dyn Fn() -> sdroxide_types::RemoteAccess + Send + Sync>;

pub use probe::ProbeFn;

/// How this station puts another radio in its roster: create the radio's
/// configuration scope, start an engine on it, and hand back the endpoints the
/// server serves it through.
///
/// A callback for the same reason as [`ProbeFn`] and `ReopenFn`: only the
/// binary knows how to build a backend, and the engine a new radio needs is
/// exactly the one the binary already builds for every radio in the roster at
/// startup. `None` leaves the station's roster fixed for the life of the
/// process, and clients are told so ([`ServerMsg::Radios`]'s `editable`) rather
/// than left pressing a button that does nothing.
///
/// The `name` is the operator's name for the radio, empty for one that will be
/// named after whatever interface it ends up configured as.
pub type AddRadioFn = Box<dyn Fn(&str) -> Result<RadioParams, String> + Send + Sync>;

/// How this station takes a radio out of its roster — the roster file only:
/// the engine stops by itself once the server has let go of its command
/// channel, and the radio's configuration stays on disk, because closing a
/// radio is not a request to destroy what it was set up as.
pub type RemoveRadioFn = Box<dyn Fn(u32) -> Result<(), String> + Send + Sync>;

/// How this station records the operator's name for one of its radios. Empty
/// puts it back on the interface-derived default.
pub type RenameRadioFn = Box<dyn Fn(u32, &str) -> Result<(), String> + Send + Sync>;

/// Where this station keeps its radios' power switches — its roster file, which
/// is the host's, not the server's.
///
/// One callback for both directions: `None` asks where the switch stands,
/// `Some(on)` throws it, and either way what comes back is the position it is
/// now in. A switch a client could throw but not read — or read but not throw —
/// would be half a switch, and the client would have to guess the other half.
///
/// `None` for the whole hook leaves the station's radios as they were started
/// and clients are told so ([`sdroxide_proto::RadioInfo::enabled`] is then
/// `None`), rather than left pressing a button that does nothing.
pub type RadioPowerFn = Box<dyn Fn(u32, Option<bool>) -> Result<bool, String> + Send + Sync>;

/// One radio's engine endpoints. A station hands the server one of these per
/// radio in its roster, and each gets an address of its own on the socket —
/// see [`ServerParams::radios`].
pub struct RadioParams {
    /// This radio's id in the station roster (`radios.json`), which is also
    /// what addresses it on the wire: `/ws/<id>`. The roster id rather than a
    /// position in the list, so that deleting one radio cannot renumber the
    /// others under whatever a client bookmarked.
    pub id: u32,
    /// The operator's name for it. Often empty — the roster leaves it so the
    /// GUI can name the tab after whatever interface the radio turns out to
    /// have — in which case the listing falls back to what the front end calls
    /// itself.
    pub name: String,
    pub cmd_tx: crossbeam_channel::Sender<Command>,
    pub event_rx: crossbeam_channel::Receiver<RadioEvent>,
    pub spectrum_out: triple_buffer::Output<SpectrumFrame>,
    pub wide_spectrum_out: triple_buffer::Output<SpectrumFrame>,
    /// Interleaved stereo 48 kHz demod audio from the engine.
    pub audio_rx: rtrb::Consumer<f32>,
    /// Mono 48 kHz mic samples into the engine.
    pub mic_tx: rtrb::Producer<f32>,
}

pub struct ServerParams {
    /// Every radio this station serves, in roster order. The first one is what
    /// `/ws` reaches — the whole of what a single-radio station ever offered,
    /// and what every client that knows nothing of a roster still gets — and
    /// each radio is also addressable in its own right at `/ws/<id>`.
    pub radios: Vec<RadioParams>,
    pub bind: String,
    pub port: u16,
    /// Directory with the built web client; `None` uses embedded assets
    /// (feature `embed-web`) or a plain info page.
    pub web_root: Option<PathBuf>,
    /// Who may connect. `None` leaves the server open — see [`AccessFn`].
    pub access: Option<AccessFn>,
    /// How this machine answers a remote client's device questions — what is on
    /// its USB bus, which serial ports it has, whether an address answers from
    /// here. `None` refuses them all, and the client greys out the controls
    /// that ask rather than showing an empty list as if there were no radios.
    ///
    /// Supplied by the caller because only the binary knows how to reach each
    /// backend; see [`ProbeFn`].
    pub probe: Option<ProbeFn>,
    /// How a client adds a radio to this station, and how one is taken out
    /// again or renamed. All three `None` — the default for anything that just
    /// wants to serve the radios it was given — leaves the roster fixed, and
    /// the client shows no controls for it.
    ///
    /// Wired together rather than one at a time: a client that could add a
    /// radio but never remove one would be a trap, and the operator would have
    /// to go to the station to undo a mistake made from away.
    pub add_radio: Option<AddRadioFn>,
    pub remove_radio: Option<RemoveRadioFn>,
    pub rename_radio: Option<RenameRadioFn>,
    /// How a client switches one of this station's radios off and on again —
    /// the switch the station's own tab strip carries. `None` leaves it off
    /// every client's, which for a headless station means nobody has one at
    /// all. See [`RadioPowerFn`].
    pub radio_power: Option<RadioPowerFn>,
}

pub(crate) struct SessionTx {
    pub reliable: mpsc::Sender<ServerMsg>,
    pub audio: mpsc::Sender<ServerMsg>,
    pub rx_codec: AudioCodec,
}

#[derive(Default, Clone)]
pub(crate) struct Latest {
    pub caps: DeviceCaps,
    pub state: RadioState,
    pub memories: Vec<MemoryChannel>,
    /// The memory folders, cached and replayed on connect for the same reason
    /// as `memories`.
    pub mem_folders: Vec<sdroxide_types::MemoryFolder>,
    /// The scanner's settings, announced at startup and on every change, and
    /// replayed on connect for the same reason as `memories`: a client that
    /// attaches later would otherwise open the scanner window on defaults it
    /// would then write back over the operator's.
    pub scanner: sdroxide_types::ScannerConfig,
    /// The engine announces the operator config (callsign, grid, templates)
    /// exactly once, at startup, so the settings editors are populated before
    /// any digital mode is selected. In server mode that happens long before
    /// anybody connects, so it has to be kept and replayed on connect — without
    /// it a client's callsign and grid stay empty *and disabled*, because the
    /// editors deliberately refuse to write until they have been seeded.
    pub digi: Option<DigiStatus>,
    /// The voice keyer's slot list, announced once at startup for the same
    /// reason as `digi` and replayed on connect so the keyer window opens
    /// populated instead of showing ten empty slots.
    pub voice: Option<sdroxide_types::VoiceStatus>,
    /// The transmit-image presets, announced once at startup for the same
    /// reason as `digi` and `voice` and replayed on connect. Without it a
    /// browser tab opens on five empty slots beside a console showing five full
    /// ones, and nothing will announce them again.
    pub images: Option<sdroxide_types::ImagePresets>,
    /// The standing operator notice, if any. Replayed on connect because it is
    /// a *condition*, not an event: a client that attaches while the radio is
    /// reconnecting has to be told so, and it would otherwise wait for a notice
    /// that was already sent to somebody else.
    pub notice: Option<String>,
    /// What the station is set up to do — the network cockpit, the two built-in
    /// servers, the WSJT-X broadcast and the satellite additions. Announced at
    /// startup and on every change, for the same reason as `digi`, and the only
    /// way a client on another machine can learn any of it: these are files in
    /// *this* machine's config directory.
    pub station: Option<Box<sdroxide_types::StationConfig>>,
    /// What each TLE subscription's cached listing holds, alongside the config
    /// it annotates.
    pub tle_subs: Vec<sdroxide_types::TleSubStatus>,
    /// The satellite lock's latest status. Replayed on connect because a lock
    /// is a *condition*: a client attaching mid-pass has to see it now, not at
    /// the next half-second tick — and above all must not offer to start a
    /// lock that is already running.
    pub sat_track: Option<Box<sdroxide_types::SatTrackStatus>>,
    /// Which radio interface this machine has open and how its backends are
    /// configured — `radio.json`. Announced by the engine at startup and after
    /// every change, and replayed on connect for `station`'s reason: it is a
    /// file here, so a client elsewhere has no copy, and the settings panel it
    /// feeds would otherwise open on defaults and write them back.
    pub radio: Option<Box<sdroxide_types::RadioConfig>>,
    /// The external T/R switch's health. Replayed on connect for `sat_track`'s
    /// reason and one more: it is the only thing that tells a remote operator
    /// whether the relay standing between a kilowatt and their receiver is
    /// answering, and "nothing has changed since you attached" must not read as
    /// "nothing is wrong".
    pub relay: Option<Box<sdroxide_types::RelayStatus>>,
    /// What the RDS decoder has made of the station currently tuned. Replayed on
    /// connect for the same reason as `digi` and `voice`: which station is being
    /// listened to is a standing condition, and a client that attaches after the
    /// dial stopped moving would otherwise show nothing until the radio text next
    /// changed — which on some stations is never.
    pub rds: Option<sdroxide_types::RdsData>,
    /// Every ISM device heard so far, and where the decoder is listening.
    ///
    /// Replayed on connect because both are *conditions*, and slow ones: a
    /// weather sensor speaks every 48 seconds and a meter every few minutes, so a
    /// client that attaches without the table would sit in front of an empty
    /// panel for a minute before the first row appeared — and would have no way
    /// to tell that from a decoder that is not running.
    pub ism_reports: Vec<sdroxide_types::IsmReport>,
    pub ism_status: Option<sdroxide_types::IsmStatus>,
    /// The aircraft the ADS-B decoder is tracking. Replayed on connect for the
    /// same reason the ISM table is: a target that has been overhead for five
    /// minutes is a standing condition, and a client that attaches without it
    /// would show an empty radar in front of a working decoder.
    pub adsb_status: Option<Box<sdroxide_types::AdsbStatus>>,
    /// What the VDL2 decoder has heard. Replayed on connect for the same reason
    /// the aircraft table is: an hour of messages and a list of who is out there
    /// are standing conditions, and a client that attaches without them would
    /// show an empty panel in front of a working decoder.
    pub vdl2_status: Option<Box<sdroxide_types::Vdl2Status>>,
    /// What the AIS decoder has heard. Replayed on connect for the same reason
    /// the aircraft table is, and rather more so: a ship at anchor reports once
    /// every three minutes, so a client that attached without the table would
    /// show an empty sea for minutes in front of a working decoder.
    pub ais_status: Option<Box<sdroxide_types::AisStatus>>,
    /// What the DRM decoder has made of the broadcast currently tuned. Replayed
    /// on connect for the same reason as `rds`: a station's label and the
    /// transmission's parameters are standing conditions, and a client that
    /// attaches after the decoder locked would otherwise show an empty panel
    /// in front of a perfectly good decode.
    pub drm: Option<sdroxide_types::DrmStatus>,
}

/// Everything the routes are served out of: the station's radios and the
/// things that belong to the station rather than to any one of them — the
/// solar feed on `/solar-ws`, the sign-in gate, the probe worker, and how the
/// roster itself is edited.
pub(crate) struct Station {
    /// In roster order; the first is what `/ws` reaches.
    ///
    /// Behind a lock because the roster is no longer fixed for the life of the
    /// process: a client may add a radio to the station and take one away
    /// again ([`Station::add_radio`]). Held only long enough to clone what is
    /// wanted out of it — never across an await, and never while an engine is
    /// being started.
    radios: std::sync::RwLock<Vec<Arc<Shared>>>,
    pub solar: Arc<solar::SolarHub>,
    pub auth: Arc<auth::AuthGate>,
    /// The one probe worker, kept here so a radio added later shares it rather
    /// than starting a second thread that would scan the same bus.
    probes: Arc<probe::ProbeHub>,
    add: Option<AddRadioFn>,
    remove: Option<RemoveRadioFn>,
    rename: Option<RenameRadioFn>,
    power: Option<RadioPowerFn>,
    /// One roster edit at a time, across every client on the station. The
    /// callbacks below read `radios.json`, change it and write it back, so two
    /// sessions adding a radio at the same moment would each hand out the id
    /// the other was about to use — and the second write would drop the first
    /// radio out of the file it had just been put in. A session takes its edits
    /// one at a time by itself (it awaits each one); this is what makes two
    /// sessions do the same.
    edit: Mutex<()>,
}

impl Station {
    /// The radio a client asked for by id, or `None` — which the route turns
    /// into a 404 rather than quietly handing over a different radio.
    pub(crate) fn radio(&self, id: u32) -> Option<Arc<Shared>> {
        self.radios.read().unwrap().iter().find(|r| r.id == id).cloned()
    }

    /// The station's first radio: what `/ws` means, and the one radio a client
    /// that knows nothing of a roster ever asks for. Always there — a station
    /// with none is refused at startup ([`ServerError::NoRadios`]) and the
    /// first radio is the one removal will not take.
    pub(crate) fn first(&self) -> Arc<Shared> {
        self.radios.read().unwrap()[0].clone()
    }

    /// Every radio, cloned out of the lock so callers can hold them across an
    /// await without holding the roster.
    pub(crate) fn list(&self) -> Vec<Arc<Shared>> {
        self.radios.read().unwrap().clone()
    }

    /// Whether this station takes roster edits from a client. False leaves the
    /// controls off at the far end rather than offering buttons that would be
    /// quietly ignored — see [`AddRadioFn`].
    pub(crate) fn editable(&self) -> bool {
        self.add.is_some() && self.remove.is_some()
    }

    /// The roster as a client sees it.
    pub(crate) fn roster(&self) -> Vec<sdroxide_proto::RadioInfo> {
        self.radios
            .read()
            .unwrap()
            .iter()
            .map(|r| sdroxide_proto::RadioInfo {
                id: r.id,
                name: r.display_name(),
                named: !r.name.lock().unwrap().trim().is_empty(),
                // Read through the hook on every announcement rather than
                // cached here: the switch lives in the host's roster file, and
                // that file is the one thing both this and the factory that
                // opens the interface agree on. A radio the host cannot answer
                // for gets no switch on any client, which is the same answer as
                // a station that wired none.
                enabled: self.power.as_ref().and_then(|p| p(r.id, None).ok()),
            })
            .collect()
    }

    /// Tell one radio's client what the station now has. Also how a client is
    /// told that *its own* radio has gone: the roster it gets simply does not
    /// list `me` any more.
    pub(crate) fn tell_roster(&self, r: &Shared, list: &[sdroxide_proto::RadioInfo]) {
        if let Some(s) = r.session.lock().unwrap().as_ref() {
            let _ = s.reliable.try_send(ServerMsg::Radios {
                me: r.id,
                radios: list.to_vec(),
                editable: self.editable(),
            });
        }
    }

    /// Tell every client what the station now has. Sent to all of them, not
    /// just to whoever asked: two operators on one station both have a tab
    /// strip, and only one of them pressed the button.
    pub(crate) fn announce_roster(&self) {
        let list = self.roster();
        for r in self.list() {
            self.tell_roster(&r, &list);
        }
    }

    /// Put another radio in the roster and serve it: the host creates its
    /// configuration scope and starts its engine ([`AddRadioFn`]), and it is
    /// on the air at `/ws/<id>` by the time this returns.
    ///
    /// Blocking — it starts an engine — so callers run it off the socket task.
    pub(crate) fn add_radio(self: &Arc<Self>, name: &str) -> Result<u32, String> {
        let Some(add) = self.add.as_ref() else {
            return Err("this station's radios cannot be changed from here".into());
        };
        let _edit = self.edit.lock().unwrap_or_else(|e| e.into_inner());
        let params = add(name)?;
        let id = params.id;
        if self.radio(id).is_some() {
            // A roster that hands out an id twice — hand-edited, or two
            // stations sharing a config directory. Refusing here drops the
            // engine the host just started with `params`, which stops by
            // itself once its command channel goes; serving it would put two
            // radios on one address instead.
            return Err(format!("this station already has a radio {id}"));
        }
        // Never primary: the station's network services belong to the radio
        // that was here first, and a radio added at runtime must not take them
        // over from a radio that is already running them.
        let shared = self.attach(params, false);
        self.radios.write().unwrap().push(shared);
        self.announce_roster();
        info!(radio = id, "radio added to the roster on /ws/{id}");
        Ok(id)
    }

    /// Take a radio out of the roster. Its configuration stays on disk: this
    /// is closing a radio, not destroying what it was set up as.
    ///
    /// The station's first radio is refused — it holds the shared network
    /// services and the legacy configuration, and `/ws` has to keep meaning
    /// something.
    pub(crate) fn remove_radio(&self, id: u32) -> Result<(), String> {
        let Some(remove) = self.remove.as_ref() else {
            return Err("this station's radios cannot be changed from here".into());
        };
        let _edit = self.edit.lock().unwrap_or_else(|e| e.into_inner());
        let gone = {
            let mut radios = self.radios.write().unwrap();
            if radios.first().is_some_and(|r| r.id == id) {
                return Err("the station's first radio cannot be closed".into());
            }
            let Some(i) = radios.iter().position(|r| r.id == id) else {
                return Err(format!("this station has no radio {id}"));
            };
            radios.remove(i)
        };
        // The one whose radio has gone hears first, and hears it while its
        // socket is still open: the roster it gets no longer lists `me`, which
        // is how its tab knows to close itself instead of trying to dial an
        // address that is now a 404. Dropping the session then closes that
        // socket — the queued message goes out ahead of the close, because a
        // dropped sender leaves what it has already queued to be drained.
        let list = self.roster();
        self.tell_roster(&gone, &list);
        *gone.session.lock().unwrap() = None;
        for r in self.list() {
            self.tell_roster(&r, &list);
        }
        // Stop the pump, which is what lets the engine stop: the engine runs
        // until its last command sender drops, that sender lives in `gone`, and
        // the pump holds a reference to `gone` of its own. So the pump goes
        // first, dropping its reference on the way out; the session's reference
        // has just been dropped with the session above, and ours goes at the
        // end of this function. The engine then finds its command channel
        // disconnected, stands down and releases the device.
        gone.stop.store(true, Ordering::Relaxed);
        // The roster file, on the host.
        remove(id)?;
        info!(radio = id, "radio removed from the roster");
        Ok(())
    }

    /// Record the operator's name for one of the station's radios.
    pub(crate) fn rename_radio(&self, id: u32, name: &str) -> Result<(), String> {
        let Some(rename) = self.rename.as_ref() else {
            return Err("this station's radios cannot be changed from here".into());
        };
        let _edit = self.edit.lock().unwrap_or_else(|e| e.into_inner());
        let Some(radio) = self.radio(id) else {
            return Err(format!("this station has no radio {id}"));
        };
        rename(id, name)?;
        *radio.name.lock().unwrap() = name.to_string();
        self.announce_roster();
        Ok(())
    }

    /// Switch one of the station's radios on or off: the host's roster records
    /// it, and the radio's engine rebuilds its front end from that roster — so
    /// switching off is a device let go, and switching on is one claimed again.
    ///
    /// The engine, its configuration scope and its address all stay: this is a
    /// radio put down, not one closed ([`Station::remove_radio`]), and the
    /// session looking at it stays connected to watch it come back.
    pub(crate) fn set_radio_power(&self, id: u32, on: bool) -> Result<(), String> {
        let Some(power) = self.power.as_ref() else {
            return Err("this station's radios cannot be switched from here".into());
        };
        let _edit = self.edit.lock().unwrap_or_else(|e| e.into_inner());
        let Some(radio) = self.radio(id) else {
            return Err(format!("this station has no radio {id}"));
        };
        power(id, Some(on))?;
        // The roster is what the interface factory reads, so the engine is
        // simply asked to run it again. Sent after the write and before the
        // announcement, so nobody is told the switch moved until the radio it
        // belongs to has been told the same.
        let _ = radio.cmd_tx.send(Command::ReopenSource);
        self.announce_roster();
        info!(radio = id, "radio switched {}", if on { "on" } else { "off" });
        Ok(())
    }

    /// Build one radio's shared state and start its pump thread. The whole of
    /// what serving a radio takes, so that a radio added at runtime is set up
    /// by exactly the same code as one the station started with.
    fn attach(self: &Arc<Self>, r: RadioParams, primary: bool) -> Arc<Shared> {
        let (spectrum_watch, spectrum_rx) = watch::channel(None);
        let (wide_watch, wide_spectrum_rx) = watch::channel(None);
        let shared = Arc::new(Shared {
            id: r.id,
            name: Mutex::new(r.name),
            primary,
            cmd_tx: r.cmd_tx,
            latest: Mutex::new(Latest::default()),
            session: Mutex::new(None),
            busy: AtomicBool::new(false),
            mic_tx: Mutex::new(r.mic_tx),
            spectrum_rx,
            wide_spectrum_rx,
            solar: self.solar.clone(),
            auth: self.auth.clone(),
            probes: self.probes.clone(),
            station: Arc::downgrade(self),
            stop: AtomicBool::new(false),
        });
        let pumped = shared.clone();
        // Radio 0 keeps the historical thread name, as the engine's own does:
        // it is what profiling notes and thread filters key on.
        let name = match r.id {
            0 => "sdroxide-pump".to_string(),
            id => format!("sdroxide-pump{id}"),
        };
        std::thread::Builder::new()
            .name(name)
            .spawn(move || {
                pump(
                    pumped,
                    r.event_rx,
                    r.spectrum_out,
                    r.wide_spectrum_out,
                    r.audio_rx,
                    spectrum_watch,
                    wide_watch,
                )
            })
            .expect("spawn pump thread");
        shared
    }
}

pub(crate) struct Shared {
    /// This radio's roster id, as it is addressed on the wire.
    pub id: u32,
    /// The operator's name for it, empty when they never gave one. Behind a
    /// lock because it can be changed from a client now
    /// ([`Station::rename_radio`]) — the roster file on the host is where it
    /// really lives, and this is the copy the listing is drawn from.
    pub name: Mutex<String>,
    /// The station's first radio, which is the one that relays the
    /// station-wide facts — the operating status on the solar arc, the
    /// satellite subscriptions — to the viewers on `/solar-ws`. Every radio
    /// contributes its *observations* (decodes, spots) to that feed; only this
    /// one says what the station is doing, because two radios both announcing
    /// it would take turns overwriting each other.
    pub primary: bool,
    pub cmd_tx: crossbeam_channel::Sender<Command>,
    pub latest: Mutex<Latest>,
    pub session: Mutex<Option<SessionTx>>,
    pub busy: AtomicBool,
    pub mic_tx: Mutex<rtrb::Producer<f32>>,
    pub spectrum_rx: watch::Receiver<Option<SpectrumFrame>>,
    pub wide_spectrum_rx: watch::Receiver<Option<SpectrumFrame>>,
    /// The solar-system viewers on `/solar-ws`. Independent of `busy`: they
    /// control nothing, so any number may watch alongside the one control
    /// client. See [`solar`].
    ///
    /// One feed for the station, held by every radio: it is a picture of the
    /// sky over this site, and a second radio does not put the station under a
    /// second sun.
    pub solar: Arc<solar::SolarHub>,
    /// The sign-in gate every endpoint passes through — shared, so the
    /// one-attempt-at-a-time rule and the lockout after a wrong password apply
    /// across the whole server rather than per socket, and so that a station
    /// with several radios cannot be attacked one radio at a time. See
    /// [`auth`].
    pub auth: Arc<auth::AuthGate>,
    /// The thread that answers device questions, shared by every radio: a bus
    /// scan is about this machine, and running one per radio at once is
    /// exactly what [`probe`] exists to prevent.
    pub probes: Arc<probe::ProbeHub>,
    /// The station this radio belongs to, so the pump can have the roster
    /// announced again when what this radio is *called* changes — see
    /// [`Shared::display_name`]. Weak because the station owns the radio: an
    /// `Arc` here would be a cycle, and neither would ever be dropped.
    pub station: std::sync::Weak<Station>,
    /// Set when this radio has been taken out of the roster, to stop its pump
    /// thread ([`Station::remove_radio`]).
    ///
    /// Needed because the pump holds an `Arc<Shared>` of its own, and this
    /// struct holds the engine's command sender: the engine stops when the last
    /// sender drops, and the pump stops when the engine does, so a pump that
    /// waited for the engine would be waiting for itself — and the device would
    /// stay claimed for the life of the process.
    pub stop: AtomicBool,
}

impl Shared {
    /// What to call this radio to somebody who cannot see the shack: the name
    /// the operator gave it, failing that whatever its interface calls itself
    /// — which is what the tab strip here falls back to as well — and failing
    /// both, its number, because a client still has to be able to point at it.
    pub(crate) fn display_name(&self) -> String {
        let name = self.name.lock().unwrap().clone();
        if !name.trim().is_empty() {
            return name;
        }
        let label = self.latest.lock().unwrap().caps.label.clone();
        if !label.trim().is_empty() {
            return label;
        }
        format!("Radio {}", self.id + 1)
    }
}

/// Build a tokio runtime and serve until the process exits.
pub fn run_blocking(params: ServerParams) -> Result<(), ServerError> {
    tokio::runtime::Builder::new_multi_thread().enable_all().build()?.block_on(serve(params))
}

pub async fn serve(params: ServerParams) -> Result<(), ServerError> {
    if params.radios.is_empty() {
        return Err(ServerError::NoRadios);
    }
    let station = Arc::new(Station {
        radios: std::sync::RwLock::new(Vec::new()),
        solar: Arc::new(solar::SolarHub::default()),
        auth: Arc::new(auth::AuthGate::new(params.access)),
        probes: Arc::new(probe::ProbeHub::start(params.probe)),
        add: params.add_radio,
        remove: params.remove_radio,
        rename: params.rename_radio,
        power: params.radio_power,
        edit: Mutex::new(()),
    });

    // The roster the station started with, through the same path a radio added
    // later takes.
    for (i, r) in params.radios.into_iter().enumerate() {
        let shared = station.attach(r, i == 0);
        station.radios.write().unwrap().push(shared);
    }

    // Worth saying out loud either way. This port hands out a transmitter, and
    // an operator who forwarded it through a router without noticing that it
    // asks for nothing should find that out from the log rather than from a
    // complaint about what went out on their callsign.
    match station.auth.required() {
        Some(a) if a.username.is_empty() => info!("remote clients must give the password"),
        Some(a) => info!("remote clients must sign in as {:?}", a.username),
        None => warn!(
            "no remote-access credentials configured — anyone who can reach this port can \
             operate the radio; set [remote_access] in config.toml, or Settings → General"
        ),
    }
    // Which radio is where, because the addresses are the only way a client
    // reaches the second one and nothing else on this machine will say.
    let radios = station.list();
    if radios.len() > 1 {
        for (i, r) in radios.iter().enumerate() {
            let path =
                if i == 0 { format!("/ws (also /ws/{})", r.id) } else { format!("/ws/{}", r.id) };
            match r.name.lock().unwrap().as_str() {
                "" => info!("radio {} on {path}", r.id),
                name => info!("radio {} ({name}) on {path}", r.id),
            }
        }
    }

    let mut app = Router::new()
        .route("/ws", get(session::ws_route))
        .route("/ws/{id}", get(session::ws_route_for))
        .route("/radios", get(radios_route))
        .route("/solar-ws", get(solar::ws_route))
        .with_state(station);
    app = add_static_routes(app, params.web_root);
    // Outside the router rather than inside it: this one has to run *before*
    // the path is matched against a route, and `Router::layer` runs after. See
    // `strip_proxy_prefix`.
    let app = tower::Layer::layer(&axum::middleware::from_fn(strip_prefix_layer), app);

    let addr: SocketAddr = format!("{}:{}", params.bind, params.port)
        .parse()
        .unwrap_or_else(|_| SocketAddr::from(([0, 0, 0, 0], params.port)));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("server listening on http://{addr}/");
    axum::serve(listener, axum::ServiceExt::<axum::extract::Request>::into_make_service(app))
        .await?;
    Ok(())
}

/// The endpoints this server answers at the top of its host.
///
/// Named in one place because [`strip_proxy_prefix`] has to recognise one at
/// the end of a longer path, and a list that drifted from the router would
/// leave a reverse-proxied station with an endpoint that works from the root
/// and nowhere else.
fn is_root_route(path: &str) -> bool {
    match path {
        "/ws" | "/solar-ws" | "/radios" => true,
        _ => path
            .strip_prefix("/ws/")
            .is_some_and(|id| !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit())),
    }
}

/// The path a request really names, once whatever a reverse proxy left in
/// front of it has been taken off — or `None` where there is nothing to take
/// off, which is every request to a station reached directly.
///
/// sdroxide behind a proxy on a *subpath* is an ordinary arrangement and not
/// an exotic one: a Tailscale certificate covers a host and offers no
/// subdomains, so the only place to put a second service on it is
/// `https://host/sdroxide/` (issue #241). Some proxies strip the prefix before
/// forwarding (Caddy's `handle_path`) and some do not (`reverse_proxy` on its
/// own); with an unstripped prefix every request arrives naming something this
/// server has never heard of, and the answer was a flat "not found" for the
/// page and everything under it.
///
/// The two shapes are answered separately because they are different questions:
///
/// * An **endpoint** is recognised by name. Anything ending in one of
///   [`is_root_route`]'s paths is that endpoint reached through a prefix, so
///   the prefix is dropped: `/sdroxide/ws/1` is `/ws/1`.
/// * Everything else is a **file**, and the client's `dist` is flat — no
///   subdirectories at all — so a path with a directory in it can only be a
///   prefix, and the last segment is the file: `/sdroxide/app.wasm` is
///   `/app.wasm`. A path that ends in `/` names a directory, which is the
///   client's own page, so it becomes `/`.
///
/// Neither can collide with a real request, because there is nothing real
/// underneath: a station serves four endpoints and one flat directory.
fn strip_proxy_prefix(path: &str) -> Option<String> {
    if is_root_route(path) {
        return None;
    }
    // An endpoint under a prefix: the longest suffix that names one wins, so a
    // station proxied under `/ws/` (which nobody should do, but which costs
    // nothing to get right) still resolves to its own `/ws`.
    for (i, _) in path.match_indices('/').skip(1) {
        if is_root_route(&path[i..]) {
            return Some(path[i..].to_string());
        }
    }
    // A file under a prefix — or a directory, which is the page itself.
    let rest = path.trim_start_matches('/');
    if rest.is_empty() || !rest.contains('/') {
        // Already at the top: `/`, or a single name the static handler
        // resolves for itself.
        return None;
    }
    match rest.rsplit('/').next().filter(|f| !f.is_empty()) {
        Some(file) => Some(format!("/{file}")),
        None => Some("/".to_string()),
    }
}

/// Rewrite the request's path through [`strip_proxy_prefix`] before it is
/// matched against a route. The query string is carried across untouched — it
/// is what `?view=solar` and `?radio=N` ride in.
async fn strip_prefix_layer(
    mut req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    if let Some(path) = strip_proxy_prefix(req.uri().path()) {
        let with_query = match req.uri().query() {
            Some(q) => format!("{path}?{q}"),
            None => path,
        };
        let mut parts = req.uri().clone().into_parts();
        // A path built out of one this request already carried, so it parses;
        // a failure here leaves the request exactly as it arrived, which is
        // the behaviour of every station that is not behind a proxy anyway.
        if let Ok(pq) = with_query.parse() {
            parts.path_and_query = Some(pq);
            if let Ok(uri) = axum::http::Uri::from_parts(parts) {
                *req.uri_mut() = uri;
            }
        }
    }
    next.run(req).await
}

/// The station's radios and where to reach each one, as JSON:
/// `[{"id":0,"name":"RTL-SDR","path":"/ws/0"}, …]`.
///
/// Plain HTTP rather than something on the socket, because this answers the
/// question a client has *before* it opens one — and because it is then
/// readable with a browser or curl by an operator working out why their second
/// radio is not where they expected it.
async fn radios_route(
    axum::extract::State(station): axum::extract::State<Arc<Station>>,
) -> axum::response::Response {
    use axum::response::IntoResponse;

    #[derive(serde::Serialize)]
    struct Listing {
        id: u32,
        name: String,
        /// Always the explicit form, including for radio 0: `/ws` is the alias
        /// that keeps single-radio clients working, not a second identity.
        path: String,
    }

    let list: Vec<Listing> = station
        .list()
        .iter()
        .map(|r| Listing { id: r.id, name: r.display_name(), path: format!("/ws/{}", r.id) })
        .collect();
    match serde_json::to_string(&list) {
        Ok(body) => {
            ([(axum::http::header::CONTENT_TYPE, "application/json")], body).into_response()
        }
        Err(e) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Bridge thread: engine endpoints → session lanes + latest-state cache.
fn pump(
    shared: Arc<Shared>,
    event_rx: crossbeam_channel::Receiver<RadioEvent>,
    mut spectrum_out: triple_buffer::Output<SpectrumFrame>,
    mut wide_spectrum_out: triple_buffer::Output<SpectrumFrame>,
    mut audio_rx: rtrb::Consumer<f32>,
    spectrum_watch: watch::Sender<Option<SpectrumFrame>>,
    wide_watch: watch::Sender<Option<SpectrumFrame>>,
) {
    let mut mono = Vec::<f32>::new();
    let mut opus_enc: Option<opus::Encoder> = None;
    let mut audio_seq = 0u32;

    loop {
        // Taken out of the roster: let go of the radio, so that the engine
        // behind it can stop and hand its device back. Checked at the top of
        // the loop, which runs at ~200 Hz, so "closed" and "the dongle is free
        // again" are the same moment to anybody watching.
        if shared.stop.load(Ordering::Relaxed) {
            info!(radio = shared.id, "radio closed; pump stopping");
            return;
        }

        // Events: block briefly so the loop runs ~200 Hz even when idle.
        match event_rx.recv_timeout(Duration::from_millis(5)) {
            Ok(ev) => {
                handle_event(&shared, ev);
                while let Ok(ev) = event_rx.try_recv() {
                    handle_event(&shared, ev);
                }
            }
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {}
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                warn!("engine gone; pump stopping");
                return;
            }
        }

        // Spectrum: latest wins.
        if wide_spectrum_out.update() {
            let f = wide_spectrum_out.output_buffer();
            if !f.bins.is_empty() {
                let _ = wide_watch.send_replace(Some(f.clone()));
            }
        }
        if spectrum_out.update() {
            let f = spectrum_out.output_buffer();
            if !f.bins.is_empty() {
                let _ = spectrum_watch.send_replace(Some(f.clone()));
            }
        }

        // Audio: drain stereo ring → mono → 20 ms frames → encode → lane.
        let pairs = audio_rx.slots() / 2;
        for _ in 0..pairs {
            let l = audio_rx.pop().unwrap_or(0.0);
            let r = audio_rx.pop().unwrap_or(0.0);
            mono.push(0.5 * (l + r));
        }

        let session_codec = shared.session.lock().unwrap().as_ref().map(|s| s.rx_codec);
        match session_codec {
            None => mono.clear(),
            Some(codec) => {
                while mono.len() >= 960 {
                    let payload = match codec {
                        AudioCodec::Opus48kMono => {
                            let enc = opus_enc.get_or_insert_with(|| {
                                opus::Encoder::new(
                                    48_000,
                                    opus::Channels::Mono,
                                    opus::Application::Audio,
                                )
                                .expect("opus encoder")
                            });
                            match enc.encode_vec_float(&mono[..960], 1400) {
                                Ok(v) => v,
                                Err(e) => {
                                    warn!("opus encode: {e}");
                                    mono.drain(..960);
                                    continue;
                                }
                            }
                        }
                        AudioCodec::Pcm16_48k => {
                            let mut v = Vec::with_capacity(1920);
                            for &s in &mono[..960] {
                                let i = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
                                v.extend_from_slice(&i.to_le_bytes());
                            }
                            v
                        }
                    };
                    audio_seq = audio_seq.wrapping_add(1);
                    if let Some(s) = shared.session.lock().unwrap().as_ref() {
                        // Bounded lane: dropping is the backpressure policy.
                        let _ = s.audio.try_send(ServerMsg::RxAudio { seq: audio_seq, payload });
                    }
                    mono.drain(..960);
                }
            }
        }
        // Bound the accumulator against pathological stalls.
        if mono.len() > 48_000 {
            let cut = mono.len() - 4_800;
            mono.drain(..cut);
        }
    }
}

/// UTC seconds now — the clock the propagation field stamps observations with.
fn now_utc() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn handle_event(shared: &Shared, ev: RadioEvent) {
    // Two of these also feed the globe's QSO layer. Relayed unconditionally
    // rather than only while a map is open: `broadcast::send` with no receivers
    // is a no-op, and keeping the cache warm means a viewer that connects
    // mid-slot draws the current traffic instead of waiting up to 15 s for the
    // next decode.
    match &ev {
        RadioEvent::Ft8Decodes(d) => {
            shared.solar.publish(SolarServerMsg::Decodes(d.clone()));
            // …and into the propagation field, which the solar tab draws. The
            // dial and the operator's locator come from the state and config
            // this server already holds: the tab has neither.
            let (grid, mode, dial) = {
                let l = shared.latest.lock().unwrap();
                (
                    l.digi.as_ref().map(|s| s.config.my_grid.clone()).unwrap_or_default(),
                    l.state.rx[0].mode,
                    l.state.rx_freq_hz(),
                )
            };
            let src = match mode {
                sdroxide_types::Mode::Ft8 => Some(sdroxide_types::PropSource::Ft8),
                sdroxide_types::Mode::Ft4 => Some(sdroxide_types::PropSource::Ft4),
                sdroxide_types::Mode::Ft2 => Some(sdroxide_types::PropSource::Ft2),
                sdroxide_types::Mode::Js8 => Some(sdroxide_types::PropSource::Js8),
                _ => None,
            };
            if let Some(src) = src
                && !grid.trim().is_empty()
            {
                shared.solar.observe_decodes(d, src, dial, &grid, now_utc());
            }
        }
        RadioEvent::WsprSpots(spots) => {
            let grid = {
                let l = shared.latest.lock().unwrap();
                l.digi.as_ref().map(|s| s.config.my_grid.clone()).unwrap_or_default()
            };
            if !grid.trim().is_empty() {
                shared.solar.observe_wspr(spots, &grid, now_utc());
            }
        }
        RadioEvent::PropPaths(paths) => {
            // No grid needed and none asked for: these paths touch this station
            // at neither end, which is exactly why they are worth having.
            shared.solar.observe_paths(paths, sdroxide_types::PropSource::Rbn, now_utc());
        }
        // What the station is doing right now, as against what it has heard:
        // the first radio speaks for the station here. See [`Shared::primary`].
        RadioEvent::Ft8Status(s) if shared.primary => {
            // The dial comes from the state this server holds, as it does for
            // the decodes above: the card over the arc names a band, and the
            // solar tab has no radio of its own to ask.
            let dial = shared.latest.lock().unwrap().state.rx_freq_hz();
            shared.solar.publish(SolarServerMsg::Digi {
                my_grid: s.config.my_grid.clone(),
                dx_grid: s.dx_grid.clone(),
                transmitting: s.transmitting,
                dx_call: s.dx_call.clone(),
                mode: s.mode.label().to_string(),
                freq_hz: dial,
                qso: s.qso,
            });
        }
        _ => {}
    }

    // Set by the station-config arm below, and acted on after the lock.
    let mut sat: Option<sdroxide_types::SatConfig> = None;
    // Whether this event changed what the radio is called (see the
    // `Capabilities` arm), which is a fact about the station's roster and not
    // about this radio's session.
    let mut renamed = false;
    let msg = {
        let mut latest = shared.latest.lock().unwrap();
        match ev {
            RadioEvent::Capabilities(c) => {
                // Cached for the next client's `HelloAck`, and sent to whoever
                // is on now: the engine emits this when it adopts a source, so
                // it means the radio under this session has just changed —
                // either because a client switched the interface or because one
                // that was missing at startup has attached.
                //
                // A radio with no name of its own is called after its
                // interface, so this is also the moment its name in the roster
                // changes. Noted here and announced below, outside the lock: a
                // radio that has just been added is announced before its engine
                // has said what it is, and without this it would stay "Radio 2"
                // on every client's tab strip until something else changed.
                renamed = label_of(&latest.caps) != label_of(&c);
                latest.caps = c.clone();
                Some(ServerMsg::Capabilities(c))
            }
            // The same radio, revising what it said about itself. Cached like
            // the above — a client arriving after the dial crossed a band edge
            // must be told the ladder that band has, not the one the session
            // opened on — but forwarded as its own message, so nobody reads it
            // as a radio that changed underneath them. It cannot rename
            // anything: only the label does that, and a revision leaves it.
            RadioEvent::CapabilitiesUpdated(c) => {
                latest.caps = c.clone();
                Some(ServerMsg::CapabilitiesUpdated(c))
            }
            RadioEvent::State(s) => {
                latest.state = s.clone();
                Some(ServerMsg::State(s))
            }
            RadioEvent::Memories(m) => {
                latest.memories = m.clone();
                Some(ServerMsg::Memories(m))
            }
            RadioEvent::MemoryFolders(f) => {
                latest.mem_folders = f.clone();
                Some(ServerMsg::MemoryFolders(f))
            }
            RadioEvent::Scanner(c) => {
                latest.scanner = c.clone();
                Some(ServerMsg::Scanner(c))
            }
            RadioEvent::Meters(m) => Some(ServerMsg::Meters(m)),
            RadioEvent::Spectrum(_) => None, // spectrum travels via the watch lane
            RadioEvent::WideSpectrum(_) => None, // ...and so does the full-band lane
            RadioEvent::ConnectionLost(e) => Some(ServerMsg::Error(e)),
            // Forwarded: a notice is about the radio this client is operating —
            // a refused tune, an interface reconnecting, a sound card the
            // engine could not open for it — and the operator needs it wherever
            // they happen to be sitting.
            RadioEvent::Notice(n) => {
                latest.notice = n.clone();
                Some(ServerMsg::Notice(n))
            }
            RadioEvent::Ft8Decodes(d) => Some(ServerMsg::Ft8Decodes(d)),
            RadioEvent::Ft8Status(s) => {
                latest.digi = Some(s.clone());
                Some(ServerMsg::Ft8Status(s))
            }
            RadioEvent::Ft8QsoLogged(r) => Some(ServerMsg::Ft8QsoLogged(r)),
            RadioEvent::WsprSpots(s) => Some(ServerMsg::WsprSpots(s)),
            RadioEvent::Rds(d) => {
                // Cached without the group log: that part is a delta, and
                // replaying one batch of it to a client that joined later would
                // put a few seconds of somebody else's diagnostics at the head
                // of its own log. The station picture is what a fresh client
                // needs, and it is the part that is a condition rather than an
                // event.
                latest.rds = Some(RdsData { groups: Vec::new(), ..d.clone() });
                Some(ServerMsg::Rds(d))
            }
            RadioEvent::Drm(d) => {
                latest.drm = Some(d.clone());
                Some(ServerMsg::Drm(d))
            }
            RadioEvent::SkimmerSpots(s) => Some(ServerMsg::SkimmerSpots(s)),
            RadioEvent::IsmReports(r) => {
                latest.ism_reports = r.clone();
                Some(ServerMsg::IsmReports(r))
            }
            RadioEvent::IsmStatus(st) => {
                latest.ism_status = Some(st.clone());
                Some(ServerMsg::IsmStatus(st))
            }
            RadioEvent::AdsbStatus(st) => {
                latest.adsb_status = Some(st.clone());
                Some(ServerMsg::AdsbStatus(st))
            }
            RadioEvent::Vdl2Status(st) => {
                latest.vdl2_status = Some(st.clone());
                Some(ServerMsg::Vdl2Status(st))
            }
            RadioEvent::AisStatus(st) => {
                latest.ais_status = Some(st.clone());
                Some(ServerMsg::AisStatus(st))
            }
            // Native-only for now: every station this shipped for runs its
            // own hardware locally, so there is no `ServerMsg` variant for
            // this yet — see `RadioEvent::Qo100Status`'s own doc.
            RadioEvent::Qo100Status(_) => None,
            RadioEvent::SstvLine { image_id, y, rgb } => {
                Some(ServerMsg::SstvLine { image_id, y, rgb })
            }
            RadioEvent::SstvImage { image_id, mode, w, h, png } => {
                Some(ServerMsg::SstvImage { image_id, mode, w, h, png })
            }
            RadioEvent::SstvStatus(s) => Some(ServerMsg::SstvStatus(s)),
            RadioEvent::WefaxLine { image_id, y, gray } => {
                Some(ServerMsg::WefaxLine { image_id, y, gray })
            }
            RadioEvent::WefaxImage { image_id, w, h, png } => {
                Some(ServerMsg::WefaxImage { image_id, w, h, png })
            }
            RadioEvent::WefaxStatus(s) => Some(ServerMsg::WefaxStatus(s)),
            RadioEvent::RifpRows { image_id, y, w, h, rows } => {
                Some(ServerMsg::RifpRows { image_id, y, w, h, rows })
            }
            RadioEvent::RifpImage { image_id, meta, png } => {
                Some(ServerMsg::RifpImage { image_id, meta, png })
            }
            RadioEvent::RifpStatus(s) => Some(ServerMsg::RifpStatus(s)),
            RadioEvent::DigiImage { png } => Some(ServerMsg::DigiImage { png }),
            RadioEvent::HellColumns { seq, rows, cols } => {
                Some(ServerMsg::HellColumns { seq, rows, cols })
            }
            RadioEvent::VoiceStatus(v) => {
                latest.voice = Some(v.clone());
                Some(ServerMsg::VoiceStatus(v))
            }
            RadioEvent::Spots(s) => Some(ServerMsg::Spots(s)),
            RadioEvent::NetStatus(s) => Some(ServerMsg::NetStatus(s)),
            RadioEvent::CallsignResult(c) => Some(ServerMsg::CallsignResult(c)),
            RadioEvent::Upload(r) => Some(ServerMsg::Upload(r)),
            RadioEvent::Confirmations(r) => Some(ServerMsg::Confirmations(r)),
            RadioEvent::RigctldStatus { running, addr, clients, error } => {
                Some(ServerMsg::RigctldStatus { running, addr, clients, error })
            }
            RadioEvent::TciServerStatus { running, addr, clients, error } => {
                Some(ServerMsg::TciServerStatus { running, addr, clients, error })
            }
            RadioEvent::ImagePresets(p) => {
                latest.images = Some(p.clone());
                Some(ServerMsg::ImagePresets(p))
            }
            // The other four are answers to a question this client asked.
            // Caching one would replay a page nobody wanted, and a full-size
            // chart is megabytes a fresh session has no use for.
            RadioEvent::ImageSlotSource { slot, version, png } => {
                Some(ServerMsg::ImageSlotSource { slot, version, png })
            }
            RadioEvent::ImageListing(l) => Some(ServerMsg::ImageListing(l)),
            RadioEvent::ImageFile { kind, name, png } => {
                Some(ServerMsg::ImageFile { kind, name, png })
            }
            // Unsolicited, but still not cached: a client attaching later gets
            // the store by listing it, which is the authoritative view.
            // Replaying this would stack a duplicate of the newest picture on
            // top of that listing.
            RadioEvent::ImageSaved(e) => Some(ServerMsg::ImageSaved(e)),
            // Likewise unsolicited and likewise not cached: a client that
            // attaches after the deletion lists a store the picture is already
            // out of, and replaying it would be an instruction to remove a
            // thumbnail that was never there.
            RadioEvent::ImageDeleted { kind, name } => Some(ServerMsg::ImageDeleted { kind, name }),
            // Winlink. All pass straight through: the mailbox stays on this
            // machine and the client reads it a page at a time, so there is
            // nothing to cache for replay on connect except the status, which
            // the engine re-emits whenever it changes.
            RadioEvent::WinlinkStatus(st) => Some(ServerMsg::WinlinkStatus(st)),
            RadioEvent::MailListing(l) => Some(ServerMsg::MailListing(l)),
            RadioEvent::MailMessage(m) => Some(ServerMsg::MailMessage(m)),
            RadioEvent::MailSaved(mid) => Some(ServerMsg::MailSaved(mid)),
            RadioEvent::MailDeleted { folder, mid } => Some(ServerMsg::MailDeleted { folder, mid }),
            RadioEvent::StationConfig(c) => {
                latest.station = Some(c.clone());
                // The satellite half drives the station's own tracker, which
                // there is one of: every engine announces the same file, so
                // the first radio is the one that acts on it.
                if shared.primary {
                    sat = Some(c.sat.clone());
                }
                Some(ServerMsg::StationConfig(c))
            }
            RadioEvent::TleSubStatus(s) => {
                latest.tle_subs = s.clone();
                Some(ServerMsg::TleSubStatus(s))
            }
            RadioEvent::RadioConfig(c) => {
                latest.radio = Some(c.clone());
                Some(ServerMsg::RadioConfig(c))
            }
            RadioEvent::SatTrack(t) => {
                latest.sat_track = t.clone();
                Some(ServerMsg::SatTrack(t))
            }
            RadioEvent::RotatorStatus { connected, az_deg, el_deg, error } => {
                Some(ServerMsg::RotatorStatus { connected, az_deg, el_deg, error })
            }
            RadioEvent::RelayStatus(st) => {
                latest.relay = Some(st.clone());
                Some(ServerMsg::RelayStatus(st))
            }
            // Handled above, and deliberately not forwarded: the skimmer
            // firehose is a hundred times the spot list's traffic, and the
            // solar relay already carries what it adds up to. See
            // `RadioEvent::PropPaths`.
            RadioEvent::PropPaths(_) => None,
            // Not forwarded, and the UI hides the buttons on a remote engine to
            // match. The answer names the account the service recognised, which
            // is the station owner's business rather than every connected
            // client's, and it is only ever asked for from the machine the
            // credentials live on. Same treatment as `RadioEvent::Notice`.
            RadioEvent::LoginTest(_) => None,
        }
    };
    // The satellite half of the station config also drives this machine's own
    // tracker: the browser's solar view is a viewer on the feed running here,
    // so an element set the operator subscribes to has to reach that feed, not
    // just the settings dialog it was typed into. After the lock, because the
    // hub takes one of its own.
    if let Some(sat) = sat {
        shared.solar.set_sat_config(sat);
    }
    if let Some(msg) = msg {
        if let Some(s) = shared.session.lock().unwrap().as_ref() {
            if s.reliable.try_send(msg).is_err() {
                warn!("reliable lane full; dropping message");
            }
        }
    }
    // ...and to everybody on the station, not just to this radio's client: the
    // roster is what every tab strip is drawn from, and a radio that has just
    // said what it is has just changed the name it appears under.
    if renamed && let Some(station) = shared.station.upgrade() {
        station.announce_roster();
    }
}

/// What a radio's *interface* contributes to its name — empty when it has not
/// said yet. An operator's own name wins over it, so a change here only shows
/// on a radio that has none; announcing the roster anyway is a message a
/// second, at worst, on a radio whose front end is flapping.
fn label_of(caps: &DeviceCaps) -> String {
    caps.label.trim().to_string()
}

fn add_static_routes(app: Router, web_root: Option<PathBuf>) -> Router {
    use axum::http::HeaderValue;
    use tower_http::set_header::SetResponseHeaderLayer;

    let app = match web_root {
        Some(dir) => {
            info!("serving web client from {}", dir.display());
            app.fallback_service(tower_http::services::ServeDir::new(dir))
        }
        None => add_embedded_or_placeholder(app),
    };
    // Trunk fingerprints the wasm bundle, but index.html and the hand-written
    // JS beside it keep fixed names. With no validator on those a browser is
    // free to pair a cached `audio_bridge.js` with a freshly fingerprinted
    // wasm, and the two then disagree about which functions exist — which
    // presents as the client failing to start, not as an audio glitch.
    // `no-cache` still caches; it just requires revalidation, which the
    // embedded handler answers with a 304 off the ETag.
    let app = app.layer(SetResponseHeaderLayer::if_not_present(
        axum::http::header::CACHE_CONTROL,
        HeaderValue::from_static("no-cache"),
    ));
    // Cross-origin isolation: lets the client use SharedArrayBuffer if the
    // audio path is ever upgraded to it.
    app.layer(SetResponseHeaderLayer::if_not_present(
        axum::http::header::HeaderName::from_static("cross-origin-opener-policy"),
        HeaderValue::from_static("same-origin"),
    ))
    .layer(SetResponseHeaderLayer::if_not_present(
        axum::http::header::HeaderName::from_static("cross-origin-embedder-policy"),
        HeaderValue::from_static("require-corp"),
    ))
}

#[cfg(feature = "embed-web")]
fn add_embedded_or_placeholder(app: Router) -> Router {
    use axum::http::{StatusCode, Uri, header};
    use axum::response::IntoResponse;

    #[derive(rust_embed::RustEmbed)]
    #[folder = "$CARGO_MANIFEST_DIR/../sdroxide-web/dist"]
    struct WebAssets;

    async fn serve_embedded(headers: axum::http::HeaderMap, uri: Uri) -> axum::response::Response {
        use std::fmt::Write as _;

        let path = asset_name(uri.path());
        match WebAssets::get(path) {
            Some(f) => {
                // The content hash rust-embed already computed at build time,
                // so revalidating a fixed-name asset costs a 304 and no body.
                let mut etag = String::with_capacity(66);
                etag.push('"');
                for b in f.metadata.sha256_hash() {
                    let _ = write!(etag, "{b:02x}");
                }
                etag.push('"');
                let fresh = headers
                    .get(header::IF_NONE_MATCH)
                    .and_then(|v| v.to_str().ok())
                    .is_some_and(|v| v.split(',').any(|t| t.trim() == etag));
                if fresh {
                    return (StatusCode::NOT_MODIFIED, [(header::ETAG, etag)]).into_response();
                }
                let mime = mime_guess::from_path(path).first_or_octet_stream();
                (
                    [(header::CONTENT_TYPE, mime.as_ref().to_string()), (header::ETAG, etag)],
                    f.data.into_owned(),
                )
                    .into_response()
            }
            None => (StatusCode::NOT_FOUND, "not found").into_response(),
        }
    }
    app.fallback(serve_embedded)
}

/// Which file in the client's `dist` a request path asks for.
///
/// A directory — the root, or anything ending in `/` — is the client's own
/// page. So is a name with no extension: that is a reverse-proxy prefix
/// arriving without its trailing slash (`https://host/sdroxide`), where the
/// browser has not yet been given a page to resolve `./bundle.js` against.
/// Serving it the page is what makes the address work with or without the
/// slash, and the page's own links are relative, so the assets follow.
///
/// Anything with an extension is asked for by name and answered by name, so a
/// bundle that really is missing still says so instead of returning HTML that
/// the browser would try to run as JavaScript.
///
/// Paths arrive here already stripped of any prefix — see
/// [`strip_proxy_prefix`] — so there is nothing left to trim but the leading
/// slash.
#[cfg(feature = "embed-web")]
fn asset_name(path: &str) -> &str {
    let name = path.trim_start_matches('/');
    if name.is_empty() || name.ends_with('/') || !name.contains('.') { "index.html" } else { name }
}

#[cfg(not(feature = "embed-web"))]
fn add_embedded_or_placeholder(app: Router) -> Router {
    app.fallback(|| async {
        "sdroxide server is running. Build the web client with `trunk build` \
         in crates/sdroxide-web and pass --web-root, or rebuild the server \
         with the embed-web feature."
    })
}

#[cfg(test)]
mod proxy_prefix_tests {
    use super::strip_proxy_prefix;

    /// A station reached directly is left entirely alone: every path it can be
    /// asked for already means what it says.
    #[test]
    fn nothing_in_front_means_nothing_to_strip() {
        for path in ["/", "/ws", "/ws/0", "/ws/12", "/radios", "/solar-ws", "/index.html"] {
            assert_eq!(strip_proxy_prefix(path), None, "{path}");
        }
    }

    /// Behind a proxy that forwards its prefix — Caddy's plain `reverse_proxy`
    /// — every one of those arrives with the prefix still on it, and every one
    /// used to be answered with "not found" (issue #241).
    #[test]
    fn a_prefix_comes_off_the_endpoints_and_the_files() {
        let strip = |p: &str| strip_proxy_prefix(p).unwrap_or_else(|| panic!("{p} was left alone"));
        assert_eq!(strip("/sdroxide/"), "/");
        assert_eq!(strip("/sdroxide/index.html"), "/index.html");
        assert_eq!(strip("/sdroxide/sdroxide-web-abc123_bg.wasm"), "/sdroxide-web-abc123_bg.wasm");
        assert_eq!(strip("/sdroxide/ws"), "/ws");
        assert_eq!(strip("/sdroxide/ws/2"), "/ws/2");
        assert_eq!(strip("/sdroxide/solar-ws"), "/solar-ws");
        assert_eq!(strip("/sdroxide/radios"), "/radios");
        // More than one level deep is the same answer: the client's dist is
        // flat, so anything above the file is somebody's routing.
        assert_eq!(strip("/shack/sdr/ws/1"), "/ws/1");
        assert_eq!(strip("/shack/sdr/audio_bridge.js"), "/audio_bridge.js");
    }

    /// `/ws/<id>` is a number, and only a number: `/ws/latest` is a file called
    /// `latest` somewhere, not radio number nothing.
    #[test]
    fn only_a_number_names_a_radio() {
        assert_eq!(strip_proxy_prefix("/ws/latest"), Some("/latest".to_string()));
        assert_eq!(strip_proxy_prefix("/sdroxide/ws/latest"), Some("/latest".to_string()));
        // …and the real one still resolves through a prefix that is itself /ws.
        assert_eq!(strip_proxy_prefix("/ws/ws/3"), Some("/ws/3".to_string()));
    }
}

#[cfg(all(test, feature = "embed-web"))]
mod asset_name_tests {
    use super::asset_name;

    /// A directory is the client's page, and so is a bare prefix with no
    /// trailing slash — which is what a browser asks for when somebody types
    /// `https://host/sdroxide` without one.
    #[test]
    fn a_directory_is_the_page() {
        assert_eq!(asset_name("/"), "index.html");
        assert_eq!(asset_name(""), "index.html");
        assert_eq!(asset_name("/sdroxide/"), "index.html");
        assert_eq!(asset_name("/sdroxide"), "index.html");
    }

    /// A file asked for by name is answered by name, so a bundle that really
    /// is missing says so rather than handing the browser HTML to run as
    /// JavaScript.
    #[test]
    fn a_file_is_a_file() {
        assert_eq!(asset_name("/index.html"), "index.html");
        assert_eq!(asset_name("/audio_bridge.js"), "audio_bridge.js");
        assert_eq!(asset_name("/sdroxide-web-abc_bg.wasm"), "sdroxide-web-abc_bg.wasm");
    }
}
