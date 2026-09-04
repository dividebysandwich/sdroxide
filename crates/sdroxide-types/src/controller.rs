use crate::{
    CallsignInfo, Command, Decode, DeviceCaps, DigiStatus, MemoryChannel, MemoryFolder, Meters,
    QsoRecord, RadioState, RifpMeta, RifpStatus, SkimmerSpot, SpectrumFrame, Spot, SstvMode,
    SstvStatus, UploadResult, VoiceStatus,
};

/// Events flowing engine → UI.
#[derive(Debug, Clone, PartialEq)]
pub enum RadioEvent {
    /// A front end this screen has not been looking at: an engine starting, a
    /// source adopted at runtime, a link that has just come up. Everything the
    /// screen holds about the radio belongs to the one before it.
    Capabilities(DeviceCaps),
    /// The *same* front end, revising what it said about itself.
    ///
    /// Some of what a device can do is not known when it is opened, and some
    /// of it is not a constant. A rig on a control port only answers whether
    /// it has an antenna selector once the link has been round; an RSP's LNA
    /// ladder is as long as the *band* allows, so it changes under the dial.
    /// Both have to reach the screen, and neither is a new radio — read as
    /// one, a tune across a band edge would wipe the wideband waterfall,
    /// re-read every image store and silence the announcer mid-QSY.
    CapabilitiesUpdated(DeviceCaps),
    /// Full state snapshot on any change (latest-wins).
    State(RadioState),
    Spectrum(SpectrumFrame),
    /// Full-band spectrum from a front end that sees far more than the IQ it
    /// delivers — the RX-888's whole 0–32 MHz. Only ever sent by sources that
    /// have one; every other backend leaves this lane silent.
    WideSpectrum(SpectrumFrame),
    Meters(Meters),
    Memories(Vec<MemoryChannel>),
    /// The memory folders, announced beside [`RadioEvent::Memories`] at startup
    /// and after every change. A separate event rather than a field of it:
    /// which folder a memory sits in rides in the [`MemoryChannel`] itself, so
    /// the two lists change independently.
    MemoryFolders(Vec<MemoryFolder>),
    ConnectionLost(String),
    /// A non-fatal, persistent status/warning for the operator (e.g. the radio
    /// audio input was unavailable or a mono card was selected for IQ). `None`
    /// clears it. Native-local only — not forwarded to remote clients.
    Notice(Option<String>),
    /// The outcome of a credential check asked for by
    /// [`crate::Command::TestLogin`]. Nothing was logged to produce it.
    LoginTest(crate::LoginTestResult),
    /// FT8/FT4 decodes from one receive slot.
    Ft8Decodes(Vec<Decode>),
    /// FT8/FT4 engine status change.
    Ft8Status(DigiStatus),
    /// A completed QSO, appended to the session log.
    Ft8QsoLogged(QsoRecord),
    /// Latest set of skimmer spots (CW etc.).
    SkimmerSpots(Vec<SkimmerSpot>),
    /// SSTV: one freshly decoded scanline `rgb` (3·width bytes) at row `y` of the
    /// image identified by `image_id`. Paints progressively.
    SstvLine {
        image_id: u32,
        y: u16,
        rgb: Vec<u8>,
    },
    /// SSTV: a completed image (PNG bytes) plus its identity and size.
    SstvImage {
        image_id: u32,
        mode: SstvMode,
        w: u16,
        h: u16,
        png: Vec<u8>,
    },
    /// SSTV: engine status change (tx/rx active, detected mode, progress).
    SstvStatus(SstvStatus),
    /// Weather fax: one freshly decoded scan line, one byte per pixel.
    WefaxLine {
        image_id: u32,
        y: u16,
        gray: Vec<u8>,
    },
    /// Weather fax: a completed chart (PNG bytes). The height is however many
    /// lines arrived before the stop tone, so it is carried rather than implied.
    WefaxImage {
        image_id: u32,
        w: u16,
        h: u16,
        png: Vec<u8>,
    },
    /// Weather fax: receiver status (tuning, phasing, line count).
    WefaxStatus(crate::WefaxStatus),
    /// RIFP: freshly reassembled raster rows of an incoming picture — `rows`
    /// grayscale bytes starting at row `y`, `w` bytes per row. Only the raster
    /// content-encodings can be painted before the object completes; a PNG,
    /// JPEG or facsimile object arrives as one [`RadioEvent::RifpImage`].
    RifpRows {
        image_id: u32,
        y: u16,
        /// Picture size, so a panel can size its canvas from the first row it
        /// sees rather than waiting for the manifest to reach it too.
        w: u16,
        h: u16,
        rows: Vec<u8>,
    },
    /// RIFP: a completed, digest-verified image (PNG bytes) plus what the
    /// manifest said about it.
    RifpImage {
        image_id: u32,
        meta: RifpMeta,
        png: Vec<u8>,
    },
    /// RIFP: engine status change (transfer progress, sessions, counters).
    RifpStatus(RifpStatus),
    /// FSQ image: a completed received picture (PNG bytes).
    DigiImage {
        png: Vec<u8>,
    },
    /// Hellschreiber: a run of freshly received dot columns, batched since the
    /// last poll. `cols` is column-major (`cols[c * rows + r]`, row 0 at the
    /// top), 0 = black … 255 = white.
    ///
    /// `seq` is the absolute index of the first column since the receiver
    /// started. Hell has no framing to resynchronise against, so the panel uses
    /// it to tell a dropped batch (leave a gap of the right width) from a
    /// restarted receiver (`seq` going backwards — clear the raster).
    HellColumns {
        seq: u64,
        rows: u8,
        cols: Vec<u8>,
    },
    /// Voice keyer: slot contents plus what is being recorded or transmitted.
    /// Emitted on every change and, while one of the two runs, often enough for
    /// the UI to animate a position.
    VoiceStatus(VoiceStatus),
    /// Latest set of network spots (DX cluster / POTA / SOTA / PSK Reporter),
    /// merged and de-duplicated by the spot manager.
    Spots(Vec<Spot>),
    /// A human-readable status line for the spot feeds (cluster connection
    /// state, feed errors). `None` clears it.
    NetStatus(Option<String>),
    /// Result of a [`Command::LookupCallsign`].
    CallsignResult(CallsignInfo),
    /// Result of one QSO/target upload from a [`Command::UploadQso`].
    Upload(UploadResult),
    /// Parsed QSL-confirmation records from a [`Command::SyncConfirmations`];
    /// the UI matches these against the log to set the `*_rcvd` flags.
    Confirmations(Vec<QsoRecord>),
    /// Built-in TCI server status: whether the listener is up, where it is
    /// bound, how many third-party clients are connected, and any bind error.
    /// Emitted on every transition, so the settings dialog never polls.
    TciServerStatus {
        running: bool,
        addr: String,
        clients: usize,
        error: Option<String>,
    },
    /// Built-in rigctld server status: whether the listener is up, where it is
    /// bound, how many clients are connected, and any bind error. Emitted on
    /// every transition, so the settings dialog never polls.
    RigctldStatus {
        running: bool,
        addr: String,
        clients: usize,
        error: Option<String>,
    },
    /// The transmit-image presets: what is in each slot, and the message
    /// composited over it. Emitted once at startup and on every change, like
    /// the voice keyer, because the pictures a station sends belong to the
    /// radio rather than to whichever screen is attached to it.
    ImagePresets(crate::ImagePresets),
    /// A preset's stored source picture, answering [`Command::ImageGetSlot`].
    /// `png` is empty for an empty slot. `version` is the fingerprint the
    /// picture had when it was read, so a client that asked during a change
    /// knows which one it got.
    ImageSlotSource {
        slot: u8,
        version: u32,
        png: Vec<u8>,
    },
    /// One page of a received store, answering [`Command::ImageList`].
    ImageListing(crate::ImageListing),
    /// One received picture at full size, answering [`Command::ImageGet`].
    /// An empty `png` means the store does not have it — a gallery that is
    /// waiting has to be able to stop waiting.
    ImageFile {
        kind: crate::ImageKind,
        name: String,
        png: Vec<u8>,
    },
    /// A freshly received picture has been stored. Carries the same entry a
    /// listing would, so a gallery prepends it without asking for the page
    /// again — and, unlike a listing, it is the only place a RIFP manifest
    /// survives.
    ImageSaved(crate::ImageEntry),
    /// Everything the engine host persists on the station's behalf — the
    /// network cockpit, the two built-in servers, the WSJT-X broadcast and the
    /// satellite additions. Emitted once at startup and after every change, so
    /// a settings dialog is populated from what the *station* is actually set
    /// to rather than from whatever file happens to sit beside the screen.
    ///
    /// Boxed: the bundle is by far the largest event here, and every other
    /// variant would otherwise pay for it.
    StationConfig(Box<crate::StationConfig>),
    /// What each TLE subscription's last fetch did. Emitted with the config it
    /// describes and again after a [`crate::Command::RefreshTleSubs`].
    TleSubStatus(Vec<crate::TleSubStatus>),
    /// A received picture is no longer in the store, answering
    /// [`Command::ImageDelete`].
    ///
    /// Broadcast rather than returned to whoever asked: the store belongs to the
    /// radio, and a gallery on another screen showing a thumbnail of a file that
    /// no longer exists would only find out by opening it.
    ImageDeleted {
        kind: crate::ImageKind,
        name: String,
    },
    /// What a WSPR slot decoded — or, from the WSPRnet feed, who decoded us.
    ///
    /// Its own event rather than [`RadioEvent::Ft8Decodes`]: WSPR reports a
    /// path, not a message, and the transmit power and drift that make it a
    /// measurement have nowhere to live in a [`Decode`].
    WsprSpots(Vec<crate::WsprSpot>),
    /// The scanner's settings, after a change or on connect — the same contract
    /// as [`RadioEvent::Memories`]. What the scanner is *doing* rides in
    /// [`crate::RadioState::scan`] instead, being small and constantly changing.
    Scanner(crate::ScannerConfig),
    /// Paths a spotting network demonstrated between two other stations —
    /// the Reverse Beacon Network's skimmers.
    ///
    /// The one event here that is deliberately **not** forwarded to a remote
    /// client. A few hundred of these arrive every five seconds, which is two
    /// orders of magnitude more traffic than the spot list, and the useful form
    /// of them is not the paths but the field they add up to. The engine host
    /// folds them into its own store and the solar relay ships the *result* —
    /// a handful of `BandPlane`s — to every viewer, which is both smaller and
    /// the thing a map actually draws.
    PropPaths(Vec<crate::PropPath>),
    /// What the satellite lock is doing: look angles, range, the Doppler
    /// corrections as applied. `Some` a couple of times a second while a lock
    /// is active, `None` once when it is released. Appended last, for the
    /// usual reason.
    SatTrack(Option<Box<crate::SatTrackStatus>>),
    /// The rotctld client's health: whether the daemon is reachable and where
    /// it reports the antenna pointing. Emitted on transitions and slowly
    /// while connected — the commanded target rides
    /// [`RadioEvent::SatTrack`] instead, this is what the hardware answered.
    RotatorStatus {
        connected: bool,
        az_deg: f64,
        el_deg: f64,
        error: Option<String>,
    },
    /// This radio's `radio.json`, as it stands on the machine the engine runs
    /// on: which interface is open and how every backend is configured.
    /// Emitted once at startup and after every [`Command::SetRadioConfig`],
    /// exactly as [`RadioEvent::StationConfig`] is.
    ///
    /// A client on another machine has no other way to learn any of it — the
    /// file is in the engine host's config directory — and without it the
    /// settings dialog there opens on defaults and writes those defaults back
    /// over the operator's real configuration the moment anything is touched.
    ///
    /// Boxed for the reason `StationConfig` is: it carries every backend's
    /// settings at once and every other variant would otherwise pay for it.
    RadioConfig(Box<crate::RadioConfig>),

    // ── Winlink radio email ──
    //
    // Appended for the usual reason: postcard numbers variants by position.
    /// Session state and per-folder counts, after any change.
    WinlinkStatus(crate::WinlinkStatus),
    /// One page of a folder, answering [`crate::Command::MailList`].
    MailListing(crate::MailListing),
    /// One whole message, answering [`crate::Command::MailGet`].
    MailMessage(Box<crate::MailMessage>),
    /// A composed message was filed in the outbox, with the id it was given.
    MailSaved(String),
    /// A message is gone. Sent to whichever client is attached, whether or not
    /// it asked, since a deleted message is gone from every mailbox view.
    MailDeleted {
        folder: crate::MailFolder,
        mid: String,
    },
    /// What the RDS/RBDS decoder has made of the WFM station on the main
    /// receiver: the station picture, plus the groups decoded since the previous
    /// one. Emitted twice a second while anything is moving, and once with
    /// everything cleared whenever the dial leaves a station.
    ///
    /// A snapshot everywhere except [`crate::RdsData::groups`], which is a delta
    /// — see the type for why the diagnostics log is not resent whole.
    Rds(crate::RdsData),
    /// Every ISM device heard since the decoder started, re-sent as a whole table
    /// a couple of times a second. A snapshot, not a delta — see
    /// [`crate::IsmReport`] for why the decoder reports devices rather than
    /// bursts.
    IsmReports(Vec<crate::IsmReport>),
    /// Which ISM channels are being listened to, and what the gate is seeing.
    /// Sent alongside the reports so an empty table can explain itself.
    IsmStatus(crate::IsmStatus),
    /// What the DRM decoder has made of the broadcast on the main receiver.
    /// A whole snapshot, emitted a few times a second while anything moves —
    /// the sync lights and the scrolling text both change on their own.
    Drm(crate::DrmStatus),
    /// Every aircraft the ADS-B decoder is still tracking, plus what the
    /// demodulator is seeing, re-sent whole a couple of times a second
    /// (issue #160).
    ///
    /// One message rather than the ISM decoder's pair, because unlike a room of
    /// weather sensors the whole table moves at once: an airliner reports its
    /// position twice a second, so a snapshot that carried the aircraft without
    /// the counters — or the other way round — would be describing two
    /// different instants.
    ///
    /// Boxed because a busy sector's table is far larger than anything else in
    /// this enum, and an enum is as big as its largest variant everywhere it is
    /// held.
    AdsbStatus(Box<crate::AdsbStatus>),
    /// What the QO-100 beacon decoder has made of the 10489.750 MHz downlink:
    /// lock state, the measured frequency offset, and the last decoded
    /// telemetry text. Sent whenever it changes, the same convention
    /// [`RadioEvent::IsmStatus`] follows — settings travel separately, in
    /// [`crate::RadioState::qo100`].
    ///
    /// Native-engine only for now: bridging this to a remote/WASM client
    /// would mean a matching variant in `sdroxide_proto::ServerMsg`, the way
    /// `IsmStatus` has one — not done yet, since every station this shipped
    /// for runs its own hardware locally.
    Qo100Status(crate::Qo100Status),
    /// Everything the VDL Mode 2 decoder has: the message log, the station
    /// table, and what every channel of the plan is doing. A whole snapshot, a
    /// couple of times a second.
    ///
    /// One message rather than a stream of decoded frames, for the reason
    /// [`RadioEvent::AdsbStatus`] is one: a snapshot carrying the log without
    /// the stations, or either without the counters, would be describing two
    /// different instants — and a dropped snapshot then costs nothing, because
    /// the next carries the same information.
    ///
    /// Boxed for the same reason as well: a busy channel's log and station
    /// table are far larger than anything else in this enum, and an enum is as
    /// big as its largest variant everywhere it is held.
    Vdl2Status(Box<crate::Vdl2Status>),
    /// The station's external T/R switch: whether it is configured, whether the
    /// hardware is answering, what it is, and whether the contacts are in their
    /// transmit state.
    ///
    /// Emitted by the primary engine only — one station, one antenna line — and
    /// only when it changes. Boxed for the usual reason: it carries two strings
    /// and an enum is as big as its largest variant everywhere it is held.
    ///
    /// Appended last, for the usual reason.
    RelayStatus(Box<crate::RelayStatus>),
    /// Everything the AIS decoder has: the vessel table, what both channels are
    /// doing, and why it is not running when it is not. A whole snapshot, a
    /// couple of times a second.
    ///
    /// One message rather than a stream of decoded reports, for the reason
    /// [`RadioEvent::AdsbStatus`] is one — and boxed for the same reason as
    /// well: a busy estuary's vessel table, each row carrying a trail, is far
    /// larger than anything else in this enum, and an enum is as big as its
    /// largest variant everywhere it is held.
    ///
    /// Appended last, for the usual reason.
    AisStatus(Box<crate::AisStatus>),
}

/// Snapshot of the frontend's switchable sound devices (native clients).
/// `selected_*` of `None` means "system default".
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AudioDevices {
    pub outputs: Vec<String>,
    pub inputs: Vec<String>,
    pub selected_output: Option<String>,
    pub selected_input: Option<String>,
}

/// The seam that lets the same UI run against an in-process radio engine
/// (native GUI) or a WebSocket session (WASM remote client).
pub trait RadioController {
    fn send(&mut self, cmd: Command);

    /// Non-blocking; the UI drains this each frame until `None`.
    fn poll_event(&mut self) -> Option<RadioEvent>;

    /// Microphone audio, 48 kHz mono. The local implementation is a no-op
    /// because cpal feeds the engine directly.
    fn send_mic(&mut self, _pcm_48k_mono: &[f32]) {}

    /// Whether the UI should schedule a repaint soon (fresh data pending).
    fn wants_repaint_soon(&self) -> bool {
        false
    }

    /// Whether this controller can be re-established after the link to the
    /// radio dropped, i.e. whether the UI should offer a "Reconnect" button on
    /// [`RadioEvent::ConnectionLost`]. False for an in-process engine: there is
    /// no link to redial, and an engine that has stopped needs the app
    /// restarted.
    fn can_reconnect(&self) -> bool {
        false
    }

    /// Whether the engine is on another machine, so the paths it reports —
    /// where received pictures are being saved, where a recording went — name
    /// *its* filesystem rather than the one the operator is sitting at.
    ///
    /// Wording only, and false by default: an in-process engine writes to the
    /// disk the UI reads.
    fn engine_is_remote(&self) -> bool {
        false
    }

    /// Dial the radio again after a lost connection, discarding whatever was
    /// left of the old session. `Ok` means the attempt is under way — the
    /// handshake completes asynchronously, so the UI learns the outcome from
    /// the events that follow, exactly as at startup.
    fn reconnect(&mut self) -> Result<(), String> {
        Err("this connection cannot be re-established".into())
    }

    /// Where this connection stands with the server's sign-in challenge, so
    /// the UI knows whether to put a dialog up instead of the radio.
    ///
    /// Always [`AuthPhase::Open`] for an in-process engine: there is no
    /// network between the UI and the radio, so there is nothing to prove.
    fn auth_phase(&self) -> crate::AuthPhase {
        crate::AuthPhase::Open
    }

    /// Answer the challenge. No-op where there is none.
    fn send_auth(&mut self, username: String, password: String) {
        let _ = (username, password);
    }

    /// Stop this radio's audio from being played *here* — the browser
    /// client's one-output rule, under which only the focused tab may feed
    /// the page's single audio worklet. Not the operator's mute: that is
    /// [`Command::SetMute`], a per-receiver setting the engine keeps (and
    /// remembers across restarts). No-op by default — a native radio plays
    /// through a stream of its own and needs no gate.
    fn set_muted(&mut self, muted: bool) {
        let _ = muted;
    }

    /// Tell this radio's engine to re-read the station-shared stores
    /// (memories, band stacks, digi operator config) from disk, because
    /// another radio in the same process just saved one of them. No-op for
    /// remote controllers — there is no second local engine to have written.
    fn nudge_shared_stores(&mut self) {}

    /// Detach from the radio and reclaim what this controller holds: for an
    /// in-process engine, disconnect it (it stops), drop the audio streams and
    /// join the DSP thread. Called when a radio tab closes and on app exit; a
    /// controller that owns nothing needs nothing here.
    fn shutdown(&mut self) {}

    /// The frontend's switchable audio devices, or `None` when the platform
    /// has none to offer (e.g. the browser client, where the browser owns
    /// device routing). Enumeration may be slow — call on demand, not per
    /// frame.
    fn audio_devices(&self) -> Option<AudioDevices> {
        None
    }

    /// Switch the audio output (`output = true`) or input device by name;
    /// `None` selects the system default. No-op on platforms without
    /// switchable audio.
    fn set_audio_device(&mut self, output: bool, name: Option<String>) {
        let _ = (output, name);
    }

    /// Ask the machine the radio is attached to about its devices.
    ///
    /// Three answers, and the caller has to handle all of them:
    ///
    /// - `Some(answer)` — the radio is on this machine and here is the answer.
    /// - `None` — the question was sent to the machine that has the radio; the
    ///   answer arrives later from [`RadioController::poll_probe`].
    /// - `Some(ProbeAnswer::Unsupported)` — nobody can answer it from here.
    ///
    /// The default is the last of those: a controller that owns no hardware and
    /// has nowhere to forward the question to says so, so the buttons that ask
    /// can be greyed out with a reason rather than left looking broken.
    ///
    /// Several of these block for a second or more (a LAN scan, a connection
    /// test), so this is called on demand — a dialog opening, a button pressed —
    /// and never per frame.
    fn probe(&mut self, req: crate::DeviceProbe) -> Option<crate::ProbeAnswer> {
        let _ = req;
        Some(crate::ProbeAnswer::Unsupported)
    }

    /// Answers to earlier [`RadioController::probe`] calls that went to another
    /// machine. Drained each frame like [`RadioController::poll_event`]; always
    /// empty where probes are answered in-process.
    fn poll_probe(&mut self) -> Option<crate::ProbeAnswer> {
        None
    }

    /// The persisted radio-backend config (SoapySDR vs CAT), or `None` when the
    /// client can't own it (the browser remote client).
    fn radio_config(&self) -> Option<crate::RadioConfig> {
        None
    }

    /// Persist an updated radio-backend config. This only stores it; call
    /// [`RadioController::reopen_source`] to apply it to the live engine. No-op
    /// where unsupported.
    fn set_radio_config(&mut self, cfg: crate::RadioConfig) {
        let _ = cfg;
    }

    /// Rebuild the IQ source from the persisted radio config at runtime, so a
    /// backend / CAT-audio / HPSDR-TCI-address change takes effect without a
    /// restart. Call after [`RadioController::set_radio_config`].
    ///
    /// Implemented by the remote client too — it asks the engine host to do it,
    /// which is what lets an operator away from the shack switch the server's
    /// radio over to another device without anyone restarting it.
    fn reopen_source(&mut self) {}

    /// The *other* radios the station at the far end serves, each with the
    /// address a further connection would reach it at. The shell opens them
    /// beside this one, so dialling a station gives the same tab strip as
    /// standing in it.
    ///
    /// Empty by default and always for an in-process engine: this machine's
    /// own radios come from its roster, not from a controller. Empty as well
    /// until the far end has said — the answer arrives with the handshake, a
    /// few frames after the connection is made.
    fn peer_radios(&self) -> Vec<PeerRadio> {
        Vec::new()
    }

    /// The address this controller is connected to, in the one form the far
    /// end names it by, so the shell can tell whether a radio it was offered
    /// is already on screen. `None` where there is no address — an in-process
    /// engine.
    fn peer_url(&self) -> Option<String> {
        None
    }

    /// The *operator's* name for the radio this connection is on, at the
    /// station that holds it. Used to name a tab that arrived without a name of
    /// its own — the browser client, where nobody typed an address to name it
    /// after.
    ///
    /// `None` until the far end has said, for an in-process engine, and for a
    /// radio nobody has named — one of those is called after its interface, and
    /// the tab derives that for itself.
    fn peer_name(&self) -> Option<String> {
        None
    }

    /// The id of the far station's *first* radio — the one at its `/ws`, which
    /// holds its shared network services and which it will not let a client
    /// close. `None` for an in-process engine and until the roster has landed.
    ///
    /// Named so the client can leave the control off that radio rather than
    /// offering one the station is going to refuse.
    fn peer_first_radio(&self) -> Option<u32> {
        None
    }

    /// Whether the station at the far end takes roster edits from here — the
    /// three calls below. False for an in-process engine (this machine's own
    /// roster is a file, not something a controller speaks for), for a station
    /// whose host wired none of it up, and until the far end has said.
    ///
    /// The client leaves the controls off rather than offering buttons that
    /// would be quietly ignored.
    fn station_roster_editable(&self) -> bool {
        false
    }

    /// Ask the station at the far end to put another radio in its roster.
    /// `name` is the operator's name for it, empty for one named after
    /// whatever interface it ends up configured as.
    ///
    /// Nothing comes back here: what answers is the station's roster, which it
    /// announces again to everyone on it — see [`RadioController::peer_radios`].
    /// The new radio then arrives as one more of the station's radios, and the
    /// shell opens it beside this one exactly as it does at connect time.
    fn add_station_radio(&mut self, name: &str) {
        let _ = name;
    }

    /// Ask the station at the far end to take one of its radios out of its
    /// roster. Its configuration is kept there, as it is when a radio is closed
    /// at the station itself.
    ///
    /// `id` is the station's own id for the radio
    /// ([`PeerRadio::id`]) — not a tab id here, which means nothing at the
    /// other end.
    fn remove_station_radio(&mut self, id: u32) {
        let _ = id;
    }

    /// Record the operator's name for one of the far station's radios, in that
    /// station's roster. Empty puts it back on the interface-derived default.
    ///
    /// On the wire rather than kept here because that roster is a file on the
    /// station: a name remembered only on this screen would be gone at the next
    /// reconnect and invisible to everybody else on it.
    fn rename_station_radio(&mut self, id: u32, name: &str) {
        let (_, _) = (id, name);
    }

    /// Whether the radio this connection is on is switched on at the station
    /// that holds it, where that station lets a client throw the switch.
    ///
    /// `None` where there is no switch to show: an in-process engine (this
    /// machine's own roster answers for those, and the shell reads it
    /// directly), a station whose host wired none of it up, and until the
    /// roster has landed.
    fn station_power(&self) -> Option<bool> {
        None
    }

    /// Ask the station at the far end to switch one of its radios on or off.
    /// `id` is the station's own id for it ([`PeerRadio::id`]), as with the
    /// roster edits above.
    ///
    /// Nothing comes back here either: what answers is the station's roster,
    /// which carries the switch's new position to every client on it —
    /// including this one, which is why the button shown follows the station
    /// rather than this screen's guess at what it just did.
    fn switch_station_radio(&mut self, id: u32, on: bool) {
        let (_, _) = (id, on);
    }

    /// Whether the station has just said that the radio this connection is on
    /// is no longer one of its own — the roster it announced does not list it.
    ///
    /// How a tab finds out that its radio was closed from somewhere else: at
    /// the station, or by another client. The socket is about to shut, so
    /// without this the tab would show a lost connection and offer to redial an
    /// address that is now a 404.
    fn peer_removed(&self) -> bool {
        false
    }
}

/// One of the far end's other radios: what to call it, and where to reach it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerRadio {
    /// The station's id for it, which is how it is addressed.
    pub id: u32,
    /// What the station calls it — the operator's name for it where they gave
    /// one, otherwise its interface's.
    pub name: String,
    /// Whether that name is the operator's own rather than one derived from the
    /// radio's interface. A derived one is not adopted as a tab's name: the tab
    /// derives its own, from the radio it is connected to, and so follows the
    /// interface instead of going stale on what it was when the tab opened.
    pub named: bool,
    /// The full URL to dial, built from the one this connection is on.
    pub url: String,
}
