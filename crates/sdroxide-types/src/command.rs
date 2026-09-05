use serde::{Deserialize, Serialize};

use crate::{
    AgcMode, Band, DigiConfig, Direction, ImageKind, LoginTarget, Mode, NetworkConfig, NrLevel,
    QsoStep, RadioConfig, RigctldConfig, RotatorConfig, RxId, SatConfig, SatLockConfig,
    SkimmerSettings, SpectrumConfig, SstvMode, TciServerConfig, TxEqState, UploadTarget, Vfo,
    WsjtxConfig,
};

/// The single control vocabulary. The GUI, the WebSocket protocol, and the
/// future TCI server all speak `Command`; the DSP engine is its only consumer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Command {
    // VFO / tuning
    /// Put the dial here: the frequency readout, a keypad entry, a band or
    /// memory recall, a spot from the list, an external controller. On a rig
    /// that is its own front end this moves the radio. [`Command::TuneInSpan`]
    /// (the panadapter's gestures) now does exactly the same — see there for
    /// why its exemption was retired.
    SetVfo {
        vfo: Vfo,
        hz: f64,
    },
    SelectVfo(Vfo),
    SwapVfos,
    CopyAtoB,
    SetSplit(bool),
    SetCenter(f64),
    SetSampleRate(f64),
    /// Decimate the raw IQ by this power of two before the receiver sees it,
    /// trading span for processing gain and CPU. `1` turns it off.
    ///
    /// The engine rounds down to a power of two and clamps to what the device
    /// rate can carry (see [`crate::max_decimation`]), so a client may send any
    /// factor and get the nearest one it can actually have.
    SetDecimation(u32),
    /// Engine applies band-stack recall (or the band default entry).
    SetBand(Band),

    // Receiver settings
    SetMode {
        rx: RxId,
        mode: Mode,
    },
    SetFilter {
        rx: RxId,
        lo: f32,
        hi: f32,
    },
    SetAgc {
        rx: RxId,
        agc: AgcMode,
    },
    SetAgcMaxGain {
        rx: RxId,
        db: f32,
    },
    /// Fixed audio gain used while the AGC is off.
    SetManualGain {
        rx: RxId,
        db: f32,
    },
    SetVolume {
        rx: RxId,
        v: f32,
    },
    SetMute {
        rx: RxId,
        muted: bool,
    },
    /// Squelch threshold in dBFS ([`crate::SQUELCH_OPEN_DB`] = open).
    SetSquelch {
        rx: RxId,
        db: f32,
    },
    SetNoiseBlanker(bool),
    /// Audio noise reduction for a receiver: which engine, and how hard. A host
    /// that cannot run the engine asked for answers with the one it can.
    SetNoiseReduction {
        rx: RxId,
        level: NrLevel,
    },
    /// Adaptive auto-notch (constant-tone canceller) for a receiver.
    SetAutoNotch {
        rx: RxId,
        on: bool,
    },
    SetSubRx(bool),
    /// Park the sub receiver on an absolute frequency. The engine clamps it to
    /// the device passband — the sub is a DDC on the same IQ stream as the main
    /// receiver, so it can only reach what the hardware is already receiving.
    SetSubRxFreq(f64),
    SetRit {
        enabled: bool,
        hz: i32,
    },
    SetXit {
        enabled: bool,
        hz: i32,
    },
    /// Start (`true`) or stop (`false`) recording both sides of the QSO (RX
    /// left, TX right) to an MP3 file. The engine names the file (date/time/
    /// frequency/mode) and stores it in the user's music directory (or the
    /// config dir as a fallback).
    SetRecording(bool),
    /// Whether the *next* recording mixes RX/TX down to a single channel
    /// instead of splitting them left/right. No effect on a recording already
    /// in progress.
    SetRecordingMono(bool),

    // Transmit
    SetPtt(bool),
    SetTune(bool),
    /// Arm or disarm the SWR guard and set the ratio it trips at.
    ///
    /// The engine clamps `limit`; see [`crate::TxState::swr_limit`].
    SetSwrGuard {
        enabled: bool,
        limit: f32,
    },
    /// Acknowledge a tripped SWR guard and allow transmit again.
    ///
    /// Deliberately its own command rather than a side effect of the next
    /// key-up: the operator confirming they have looked at the antenna is the
    /// entire point of latching, and a latch any old PTT press clears is not a
    /// latch.
    ClearSwrTrip,
    SetTxDrive(f32),
    SetTuneDrive(f32),
    SetMicGain(f32),
    /// Whole new state for the transmit parametric EQ (voice modes only),
    /// sent as a full snapshot on every band/enable change, the same
    /// "always whole, never a delta" way [`crate::RadioState`] itself is
    /// broadcast.
    SetTxEq(TxEqState),

    // Voice keyer (10 recorded messages; see [`crate::VoiceStatus`])
    /// Start recording the microphone into a slot (`Some`), or stop and store
    /// what has been recorded (`None`). Refused while transmitting.
    VoiceRecord(Option<u8>),
    /// Transmit a recorded message (`Some`), or stop one in progress (`None`).
    /// The engine keys the transmitter itself and unkeys at the end of the
    /// message. In RADE the recording is fed to the codec in place of the
    /// microphone; in the other digital modes the keyer is refused.
    VoicePlay(Option<u8>),
    /// Listen to a recorded message through the speakers without transmitting
    /// (`Some`), or stop the monitor (`None`). Refused while transmitting.
    VoicePreview(Option<u8>),
    /// Erase a slot's recording.
    VoiceClear(u8),
    /// Rename a slot (the label the UI and the bindings editor show).
    VoiceRename {
        slot: u8,
        name: String,
    },

    // Hardware
    SetGain {
        dir: Direction,
        element: String,
        db: f64,
    },
    SetAntenna {
        dir: Direction,
        name: String,
    },
    /// Write one driver-specific setting on a SoapySDR device, by the key the
    /// device published for it. Applied to the running radio; persisting it is
    /// a separate `SetRadioConfig`, because an operator trying a switch out is
    /// not the same act as choosing to keep it.
    SetDeviceSetting {
        key: String,
        value: String,
    },

    // Memories
    StoreMemory {
        name: String,
    },
    RecallMemory(u32),
    DeleteMemory(u32),

    // Display
    SetSpectrumCfg(SpectrumConfig),
    /// The window the operator can actually see on the waterfall, in absolute
    /// Hz — `None` for "everything the skim window covers".
    ///
    /// Distinct from [`SpectrumConfig::viewport`], which is deliberately padded
    /// with slack so panning does not force a reconfiguration that would clear
    /// the waterfall history. This one is the real on-screen span, and it is
    /// what decides which signals the skimmers spend time decoding.
    SetSkimmerView(Option<(f64, f64)>),

    // Digital modes (FT8/FT4)
    SetDigiConfig(DigiConfig),
    /// Set our transmit tone offset within the passband (Hz).
    SetDigiAudioFreq(f32),
    /// Start calling CQ.
    DigiCallCq,
    /// Begin a QSO with a decoded station. `wait_for_cq` holds transmission
    /// until the station calls CQ (or calls us) — set when replying to a decode
    /// that is neither a CQ nor addressed to us, so we don't jump into an
    /// exchange already in progress.
    DigiStartQso {
        from: String,
        grid: Option<String>,
        snr: i16,
        audio_hz: f32,
        #[serde(default)]
        wait_for_cq: bool,
    },
    /// FT8/FT4: jump the exchange to this step, choosing by hand which message
    /// goes out next (WSJT-X's Tx1–Tx6). Steps that address a station are
    /// ignored when none is being worked.
    DigiSetStep(QsoStep),
    /// FT8/FT4: send this message verbatim in the next transmit slot, then
    /// carry on with the exchange. Empty text cancels one queued but unsent.
    DigiSendText(String),
    /// FT8/FT4: mark a station to work. Queued stations are taken in order, the
    /// next one starting as soon as the sequencer is free — so a run of callers
    /// can be marked in one pass over a busy slot and then worked hands-off.
    /// Adding a station already queued moves it to the end.
    DigiQueueAdd {
        from: String,
        grid: Option<String>,
        snr: i16,
        audio_hz: f32,
        /// Hold until they call CQ, as [`Command::DigiStartQso`] does.
        #[serde(default)]
        wait_for_cq: bool,
    },
    /// FT8/FT4: drop a station from the call queue. An empty callsign clears it.
    DigiQueueRemove(String),
    /// Gracefully stop the QSO sequence (finish the current burst, then idle).
    DigiStopQso,
    /// Abort any in-progress transmission immediately.
    DigiAbortTx,
    /// Continuous keyboard modes (PSK/RTTY): set the full outgoing text buffer.
    /// The engine keeps already-sent characters and streams the rest.
    DigiTxText(String),
    /// Continuous keyboard modes: enter (true) or leave (false) transmit.
    DigiTxActive(bool),
    /// Empty the received-text window: the decoded stream of a keyboard mode,
    /// the packet monitor, the JS8 conversation. Only what has been copied goes
    /// — nothing about the receiver or an over in progress changes, so this is
    /// safe to press mid-QSO to start a fresh page.
    DigiClearRx,
    /// SSTV: select the mode (also sizes the TX image). `None` = Auto — the RX
    /// auto-detects the mode and TX defaults to Martin 1.
    SstvSetMode(Option<SstvMode>),
    /// SSTV: transmit a composed image (PNG bytes) in the given mode. Keying
    /// starts immediately; `DigiAbortTx` stops it.
    SstvTx {
        mode: SstvMode,
        png: Vec<u8>,
    },
    /// Weather fax: begin a picture now, without waiting for a start tone.
    /// The usual way to catch a chart that was already running when you tuned.
    WefaxStart,
    /// Weather fax: end the picture in progress, keeping what has arrived.
    WefaxStop,
    /// Weather fax: shift the line alignment by whole pixels. Positive moves
    /// the picture right.
    WefaxNudge(i32),
    /// FSQ image: transmit a picture (PNG bytes; the engine grayscales/scales it).
    DigiImageTx {
        png: Vec<u8>,
    },
    /// RIFP: transmit a composed image (PNG bytes). The engine quantises it to
    /// the configured grayscale depth, encodes it as the configured
    /// content-encoding, and sends manifest + data + end frames. Keying starts
    /// immediately; `DigiAbortTx` stops it.
    RifpTx {
        png: Vec<u8>,
    },
    /// RIFP: drop an incomplete incoming session by its 16-hex-digit ID, or
    /// every session when the string is empty.
    RifpDropSession(String),

    // Skimmers
    /// Set which skimmers (CW / PSK / RTTY) run and how hard each squelches.
    SetSkimmerConfig(SkimmerSettings),

    // Network cockpit: spot feeds, lookups, uploads.
    /// Apply (and persist) the network-feature configuration: (re)connect the
    /// DX cluster, (dis)arm the POTA/SOTA/PSK feeds, and store credentials.
    SetNetworkConfig(NetworkConfig),
    /// Check one logging service's credentials, without logging anything.
    ///
    /// Tests the APPLIED network config, so a dialog with unsaved edits sends
    /// [`Command::SetNetworkConfig`] immediately before this one. The answer
    /// comes back as [`crate::RadioEvent::LoginTest`].
    TestLogin(LoginTarget),
    /// The operator's current dial frequency, so band-scoped feeds (PSK
    /// Reporter) can query the right slice. Sent by the engine on VFO change.
    SpotDialHint(f64),
    /// Look up a callsign via the configured provider; the result comes back as
    /// [`crate::RadioEvent::CallsignResult`].
    LookupCallsign {
        call: String,
    },
    /// Upload one QSO's ADIF to the given targets; each result comes back as
    /// [`crate::RadioEvent::Upload`].
    UploadQso {
        qso_id: u64,
        adif: String,
        targets: Vec<UploadTarget>,
    },
    /// Download QSL confirmations from LoTW/eQSL and return the parsed
    /// confirmation records as [`crate::RadioEvent::Confirmations`].
    SyncConfirmations,

    /// Apply (and persist) the built-in TCI server configuration: bind, rebind
    /// or stop the listener that third-party TCI clients connect to. The result
    /// comes back as [`crate::RadioEvent::TciServerStatus`].
    SetTciServerConfig(TciServerConfig),

    /// Apply (and persist) the built-in Hamlib rigctld server configuration:
    /// bind, rebind or stop the listener that "NET rigctl" clients (WSJT-X,
    /// fldigi, N1MM, GPredict, …) connect to. The result comes back as
    /// [`crate::RadioEvent::RigctldStatus`].
    SetRigctldConfig(RigctldConfig),
    /// Apply (and persist) the WSJT-X UDP broadcast configuration: start, retarget
    /// or stop the datagram stream that GridTracker, JTAlert, N1MM+ and Log4OM
    /// listen to. Output only — nothing arrives on that socket.
    SetWsjtxConfig(WsjtxConfig),

    /// Allow WFM broadcast stereo on a receiver. `false` forces mono even when
    /// the 19 kHz pilot is locked — worth having for a noisy station, since the
    /// difference channel carries far more noise than the sum. No effect on any
    /// other mode. Appended rather than filed next to the other per-RX audio
    /// commands: postcard numbers variants by position.
    SetWfmStereo {
        rx: RxId,
        on: bool,
    },

    // Transmit-image presets and the received-picture stores. Appended for the
    // usual reason: postcard numbers variants by position.
    /// Store a picture in transmit preset `slot`, from the raw bytes of a
    /// picked file (PNG or JPEG). The engine scales the long edge down to
    /// [`crate::IMAGE_SOURCE_MAX_EDGE`] and keeps a PNG of that, so the store
    /// never holds a phone camera's forty megapixels and every client composes
    /// from identical pixels. The result comes back as
    /// [`crate::RadioEvent::ImagePresets`]; an upload that is too big or is not
    /// a picture is refused with a [`crate::RadioEvent::Notice`].
    ImageSetSlot {
        slot: u8,
        bytes: Vec<u8>,
    },
    /// Empty a preset's picture. The overlay message is the operator's text and
    /// stays — clearing a picture is replacing it, not forgetting what it said.
    ImageClearSlot(u8),
    /// Set the text composited over a preset's picture.
    ImageSetMessage {
        slot: u8,
        message: String,
    },
    /// Ask for a preset's stored source picture; the answer is
    /// [`crate::RadioEvent::ImageSlotSource`]. Requested lazily and cached
    /// against the slot's version: composition happens client-side, so the
    /// pixels only have to cross once per picture, not once per keystroke.
    ImageGetSlot(u8),
    /// List a received store, newest first: `count` entries (capped at
    /// [`crate::IMAGE_PAGE_MAX`]) starting at `offset`. Answered with
    /// [`crate::RadioEvent::ImageListing`].
    ImageList {
        kind: ImageKind,
        offset: u32,
        count: u32,
    },
    /// Fetch one received picture at full size, by the name a listing gave.
    /// Answered with [`crate::RadioEvent::ImageFile`].
    ImageGet {
        kind: ImageKind,
        name: String,
    },

    // The operator's satellite additions. Appended for the usual reason:
    // postcard numbers variants by position.
    /// Apply (and persist) the satellite configuration: pasted element sets,
    /// subscribed listings and frequency overrides. Answered with a fresh
    /// [`crate::RadioEvent::StationConfig`] and, since the subscription list
    /// may have changed, a fresh [`crate::RadioEvent::TleSubStatus`].
    ///
    /// It rides the engine rather than being written client-side because the
    /// tracker's listings are fetched — and cached — on the machine the engine
    /// runs on, which is the only one a browser client can reach.
    SetSatConfig(SatConfig),
    /// Re-fetch every enabled TLE subscription now, rather than waiting for the
    /// six-hourly cadence (the settings dialog's UPDATE NOW). One HTTPS round
    /// trip per subscription, off the engine thread; the outcome comes back as
    /// [`crate::RadioEvent::TleSubStatus`].
    RefreshTleSubs,

    // Culling a received store. Appended for the usual reason: postcard numbers
    // variants by position.
    /// Delete one received picture, by the name a listing gave it.
    ///
    /// The file goes, on the machine the radio is plugged into — a store fills
    /// up with noise-only frames and half-decoded charts, and the alternative is
    /// walking over to that machine with a file manager. The name is sanitised
    /// and resolved inside the store exactly as [`Command::ImageGet`]'s is: it
    /// arrives over a socket nothing authenticates and ends up at
    /// `std::fs::remove_file`, which is the one place in this program where
    /// getting that wrong costs something that cannot be got back.
    ///
    /// Answered with [`crate::RadioEvent::ImageDeleted`] to *every* attached
    /// client, since a picture that has gone is gone from all of their galleries;
    /// a delete that fails becomes a [`crate::RadioEvent::Notice`] instead.
    ImageDelete {
        kind: ImageKind,
        name: String,
    },

    /// Require a CTCSS tone or DCS code before the audio gate opens on this
    /// receiver, or `None` for plain carrier squelch. NFM only. Appended for the
    /// usual reason: postcard numbers variants by position.
    SetToneSquelch {
        rx: RxId,
        tone: Option<crate::SubTone>,
    },

    // Scanning. Appended for the usual reason: postcard numbers variants by
    // position.
    /// Apply (and persist) the scanner settings. Echoed back to every client as
    /// [`crate::RadioEvent::Scanner`], the way memories are.
    SetScannerConfig(crate::ScannerConfig),
    /// Start or stop scanning. A scan also stops on its own if the operator
    /// tunes, changes band, recalls a memory or transmits.
    SetScanning(bool),
    /// Leave the channel the scan stopped on and carry on now.
    ScanNext,
    /// Leave it and don't stop here again: adds the memory to the skip list, or
    /// the frequency to the range scan's.
    ScanSkip,

    /// Attenuate receiver audio to `gain` (0.0..=1.0) while a local spoken
    /// announcement plays, and back to 1.0 when it finishes. Appended for the
    /// usual reason: postcard numbers variants by position.
    ///
    /// The speaker path only — the recording tap is left alone, so a duck never
    /// appears in an MP3. It does reach anyone listening remotely, since they
    /// tap the same mixer; the client therefore sends this only when it owns
    /// the engine rather than when it is somebody else's remote.
    SetAudioDuck(f32),

    // Memory folders. Appended for the usual reason: postcard numbers variants
    // by position.
    /// Create a folder in the memory list. Echoed back to every client as
    /// [`crate::RadioEvent::MemoryFolders`], the way memories are.
    CreateMemoryFolder {
        name: String,
    },
    /// Rename a folder.
    RenameMemoryFolder {
        id: u32,
        name: String,
    },
    /// Delete a folder. The memories filed under it move back to the top
    /// level — deleting a folder is never a way to delete a memory.
    DeleteMemoryFolder(u32),
    /// File a memory under a folder (`Some`), or back at the top level
    /// (`None`). Refused for a folder id that doesn't exist.
    MoveMemoryToFolder {
        id: u32,
        folder: Option<u32>,
    },

    // The satellite lock. Appended for the usual reason: postcard numbers
    // variants by position.
    /// Lock onto a satellite (`Some`) or release the lock (`None`). While
    /// locked the engine propagates the orbit, applies Doppler to the signal
    /// path, derives the uplink from the transponder mapping, and answers with
    /// a stream of [`crate::RadioEvent::SatTrack`]. Boxed because the config
    /// carries strings and every other variant would otherwise pay for them.
    SetSatLock(Option<Box<SatLockConfig>>),
    /// Configure the rotctld client a lock steers the antenna through.
    /// Persisted engine-side and echoed to every client in the
    /// [`crate::RadioEvent::StationConfig`] bundle, like the servers.
    SetRotatorConfig(RotatorConfig),

    /// Set the station's ITU / IARU region, which decides every band edge and
    /// sub-segment. Persisted to `config.toml` engine-side and echoed to every
    /// client in the [`crate::RadioEvent::StationConfig`] bundle. Appended for
    /// the usual reason: postcard numbers variants by position.
    ///
    /// Station-wide rather than per-radio: a second radio at the same desk is
    /// on the same continent as the first.
    SetRegion(crate::Region),

    /// Re-read `bandplan.json` from the engine's config directory and adopt it.
    ///
    /// The band plan is a file the operator edits in a text editor, so the
    /// alternative to this is restarting the program after every correction.
    /// Answered with a fresh [`crate::RadioEvent::StationConfig`] carrying the
    /// plan that was actually loaded — which is the built-in one if the file
    /// would not parse, and the loader says so through a
    /// [`crate::RadioEvent::Notice`].
    ReloadBandPlan,

    /// The engine host's `radio.json`, as edited by whoever is driving this
    /// radio. Appended for the usual reason: postcard numbers variants by
    /// position.
    ///
    /// The engine owns the file — it is in *its* config directory, not the
    /// screen's — so it does the writing, and echoes the result back as
    /// [`crate::RadioEvent::RadioConfig`]. That is what lets an operator away
    /// from the shack reach the settings only the device itself has: an
    /// RTL-SDR's AGC mode, its ppm correction, its bias tee. Those ride
    /// [`Command::SetGain`] pseudo-elements to the running device, but the
    /// figure that survives a restart lives here.
    ///
    /// `reopen` rebuilds the front end afterwards. The settings that are fixed
    /// when a device is opened — sample rate, addresses, sound cards — need it;
    /// the ones that apply as you move them do not. One command rather than a
    /// save and a separate reopen, so the two cannot arrive out of order and
    /// rebuild the device from the config it had a moment ago.
    ///
    /// Boxed: `RadioConfig` carries every backend's settings at once, and every
    /// other variant here would otherwise be as large as the biggest of them.
    SetRadioConfig {
        cfg: Box<RadioConfig>,
        reopen: bool,
    },

    // ── Winlink radio email ──
    //
    // The mailbox lives on the machine with the radio and is read lazily,
    // exactly like the picture store: a listing carries metadata, a fetch
    // carries one message. Appended for the usual reason — postcard numbers
    // variants by position.
    /// Run a forwarding session now. Answered by
    /// [`crate::RadioEvent::WinlinkStatus`] when it finishes; refused there too
    /// if one is already running.
    WinlinkConnect,
    /// Stop the forwarding session in progress. Cooperative: the link is torn
    /// down properly and whatever already arrived is still filed.
    WinlinkAbort,
    /// Packet: send one UNPROTO identification frame now. Still subject to
    /// CSMA — the operator asks for the channel, they do not take it.
    PacketBeacon,
    /// One page of a mail folder, newest first: `count` entries (capped at
    /// [`crate::MAIL_PAGE_MAX`]) from `offset`. Answered with
    /// [`crate::RadioEvent::MailListing`].
    MailList {
        folder: crate::MailFolder,
        offset: u32,
        count: u32,
    },
    /// Fetch one message with its attachments, by the id a listing gave.
    /// Answered with [`crate::RadioEvent::MailMessage`].
    MailGet {
        folder: crate::MailFolder,
        mid: String,
    },
    /// File a composed message in the outbox. The MID and date are minted by
    /// the engine, not the client: a duplicate MID is rejected by the CMS, and
    /// a client cannot guarantee uniqueness. Answered with
    /// [`crate::RadioEvent::MailSaved`].
    MailCompose(Box<crate::MailDraft>),
    /// Delete a message. Like the picture store's delete, this arrives over a
    /// socket nothing authenticates and ends at `remove_file`, so the id is
    /// validated against an allow-list before it names a path.
    MailDelete {
        folder: crate::MailFolder,
        mid: String,
    },
    /// Move a message between folders.
    MailMove {
        from: crate::MailFolder,
        to: crate::MailFolder,
        mid: String,
    },
    /// Tune from the panadapter's own gestures (a click, a spot box) as
    /// against [`Command::SetVfo`], which is the dial. The engine answers both
    /// identically; the discriminant survives because protocol v62 put it on
    /// the wire.
    ///
    /// On an SDR the window is a resource worth keeping, so the hardware only
    /// retunes when the VFO would leave the span, whichever command asked. On
    /// a rig that *is* the front end — a transceiver whose I/Q output feeds a
    /// sound card, an Icom sending its 12 kHz IF — the dial and the centre of
    /// what we capture are one synthesiser, and both commands move it. A click
    /// used to be exempt there, on the theory that a signal already inside the
    /// captured span needs no retune, until a field report (a Kenwood on its
    /// I/Q output) showed what the exemption costs: the rig's readout
    /// disagreeing with ours, its next frequency report snapping ours back —
    /// and any over the engine does not key itself, CW sent as text to the
    /// rig's own keyer or a microphone keyed at the radio, transmitting on the
    /// dial the click left behind.
    TuneInSpan {
        vfo: Vfo,
        hz: f64,
    },
    /// Which ISM decoders run and how hard they squelch. The engine persists this
    /// and echoes it back in [`crate::RadioState`], so there is no apply step.
    SetIsmConfig(crate::IsmSettings),

    /// Re-read `rtl433_flex.conf` and restart the rtl_433 decoders on it.
    /// Appended for the usual reason: postcard numbers variants by position.
    ///
    /// The operator's decoder file is edited outside sdroxide, so nothing else
    /// notices it changed — the same reason [`Command::ReloadBandPlan`] exists.
    /// The device table survives: adding a decoder should not cost somebody the
    /// sensors already on screen. Whatever the file had to say comes back as a
    /// [`crate::RadioEvent::Notice`] and in the ISM status.
    ReloadIsmDecoders,

    /// Rebuild the front end from the persisted radio configuration, without
    /// writing anything to it. Appended for the usual reason: postcard numbers
    /// variants by position.
    ///
    /// What [`Command::SetRadioConfig`]'s `reopen` does, for the caller that
    /// has nothing to save — a station switching one of its radios on or off.
    /// The switch is a line in the station's *roster*, not in this radio's
    /// configuration, and the factory that opens the interface reads it: so
    /// all that is left is to ask the engine to run the factory again, which
    /// is exactly this.
    ReopenSource,

    /// Edit a stored memory in place. Appended for the usual reason: postcard
    /// numbers variants by position.
    ///
    /// Correcting a typo in a name, or a frequency that has moved, used to
    /// mean deleting the channel and storing it again — which takes the
    /// operator to the frequency first, loses the channel's place in the list
    /// and its folder with it, and does not scale past a handful of memories.
    ///
    /// The filter is not here: it is the mode's, and the engine gives the
    /// channel the new mode's default whenever the mode (or the sideband that
    /// mode rides at the new dial) changes, and otherwise leaves the passband
    /// the operator stored. Neither is the folder — filing is
    /// [`Command::MoveMemoryToFolder`], which is the drag in the list, and one
    /// act should have one command. An empty name is ignored rather than
    /// stored: a memory called nothing is a row nobody can pick out.
    EditMemory {
        id: u32,
        name: String,
        freq_hz: f64,
        mode: Mode,
        /// The repeater setup to store with the channel. `None` clears it,
        /// which is what a memory written before the field existed already
        /// means: recall onto whatever the repeater controls are set to.
        repeater: Option<crate::RepeaterState>,
        /// The antenna socket to store with the channel, by the name the front
        /// end gives the port. `None` clears it, which means "recall this
        /// channel without moving the antenna" — see
        /// [`crate::MemoryChannel::antenna`], which reads an absent field the
        /// same way.
        ///
        /// A name the front end does not have is dropped rather than stored: a
        /// channel may perfectly well have been captured on another radio's
        /// socket, and remembering a port this receiver has never had would
        /// leave a memory that can only ever be recalled onto nothing.
        antenna: Option<String>,
    },

    /// Working a repeater: the transmit shift, the sub-audible tone under the
    /// voice and the 1750 Hz burst, all in one setting.
    ///
    /// The whole struct rather than a command per field, because these are set
    /// together — a directory entry gives the output, the shift and the tone as
    /// one line — and because a half-applied repeater setup transmits on the
    /// right frequency with the wrong tone, or the other way round. The engine
    /// clamps what arrives ([`crate::RepeaterState::clamped`]).
    ///
    /// The *receive* tone squelch is not in here: that is
    /// [`Command::SetToneSquelch`], and it is a property of the receiver rather
    /// than of the repeater.
    SetRepeater(crate::RepeaterState),

    /// Send the 1750 Hz tone burst that opens a carrier-access repeater.
    ///
    /// Mid-over it plays over the microphone for
    /// [`crate::RepeaterState::burst_ms`]. Keyed from receive it keys the
    /// transmitter, sends the burst and unkeys again — which is the whole of
    /// what the button on a European mobile does, and the reason it is one
    /// button. Every transmit rail applies: this is an ordinary key-down and it
    /// is refused for the same reasons any other one would be.
    ToneBurst,

    /// Decode a different service of the DRM multiplex, 0-based.
    ///
    /// Most broadcasts carry one audio service and this never comes up. A few
    /// carry two programmes, or a programme alongside a data service, and
    /// without this the receiver is stuck on whichever the transmission lists
    /// first. Out-of-range values are ignored rather than clamped: the number
    /// of services is a property of the transmission, and a stale click from a
    /// client that has not seen the multiplex change should do nothing rather
    /// than land somewhere else.
    SetDrmService {
        service: u8,
    },

    /// Start or stop reading back a DRM logical channel's constellation.
    ///
    /// `None` stops it. The plot is a few hundred floats several times a
    /// second, which is worth carrying to a remote client while somebody has it
    /// open and pure waste otherwise, so the client says when it is looking.
    SetDrmConstellation {
        channel: Option<crate::DrmChannel>,
    },

    /// APRS: send one position beacon now, rather than waiting for the timer.
    ///
    /// Still subject to CSMA and to every transmit rail: the operator asks for
    /// the channel, they do not take it. Appended for the usual reason —
    /// postcard numbers variants by position.
    AprsBeacon,

    /// APRS: send a message to a station, and keep retrying until it is
    /// acknowledged.
    ///
    /// The identifier is minted engine-side rather than by the client, for the
    /// same reason a Winlink MID is: it has to be unique against the messages
    /// already in flight, which only the engine can see.
    AprsSendMessage {
        to: String,
        text: String,
    },

    /// Packet: call a station in connected mode — a node, a BBS, a peer.
    ///
    /// `via` is the operator's string rather than a parsed list, because that
    /// is what a TNC takes and what a node's own listing prints: callsigns
    /// separated by commas or spaces, nearest hop first. Parsing it belongs
    /// where the callsign validation is, engine-side, so a typo comes back as a
    /// line in the terminal instead of a command that vanishes.
    ///
    /// Appended for the usual reason — postcard numbers variants by position.
    PacketConnect {
        call: String,
        via: String,
        /// Extended (mod-128) sequence numbers.
        ext: bool,
    },

    /// Packet: send one line to the connected station.
    ///
    /// A line, not bytes: the terminator is the link's business, because a BBS
    /// wants a CR and what the operator typed has neither.
    PacketSend {
        text: String,
    },

    /// Packet: hang up. A clean DISC, waited out — not a link dropped on the
    /// floor for the far end to time out.
    PacketDisconnect,

    /// Packet: empty the terminal transcript. The link is untouched.
    PacketTermClear,

    /// Set one mode's transmit-audio level, on a radio that modulates what we
    /// send it (issue #186). `level` is linear, clamped to
    /// [`sdroxide_types::TX_AUDIO_LEVEL_MIN`]`..=1.0` engine-side.
    ///
    /// [`sdroxide_types::TX_AUDIO_LEVEL_MIN`]: crate::TX_AUDIO_LEVEL_MIN
    ///
    /// The mode travels rather than being read off the dial when this arrives,
    /// even though the engine knows it: this control is a rail an operator
    /// drags while transmitting, and a mode change landing between the drag and
    /// the command would write one mode's level onto another's entry. That
    /// failure is silent, persistent, and corrupts exactly the thing the
    /// setting exists to hold.
    ///
    /// Its own command rather than a whole [`DigiConfig`](crate::DigiConfig)
    /// through `SetDigiConfig` because a drag emits one of these per frame, and
    /// the configuration is kilobytes — which a remote client would be sending
    /// over the wire for the length of every adjustment. That also makes
    /// `tx_audio_levels` a field with a write route outside `SetDigiConfig`,
    /// which is what puts it in the engine's `keep_engine_owned`.
    ///
    /// Appended for the usual reason — postcard numbers variants by position.
    SetDigiTxLevel {
        mode: Mode,
        level: f32,
    },

    /// Set the *radio's own* squelch threshold, as a `0..1` fraction of its
    /// scale — `0` open, `1` closed (issue #192).
    ///
    /// Not [`Command::SetSquelch`], and the difference is which receiver is
    /// being quietened. That one is sdroxide's gate on a passband sdroxide
    /// demodulated, in dBFS. This is a level in the rig, and on a transceiver
    /// that hands us audio it has already gated it is the only one that can
    /// open: what reaches the sound card is what the radio let through.
    ///
    /// Ignored by a front end with no such control
    /// ([`crate::DeviceCaps::commands_squelch`]), which is every SDR.
    ///
    /// Appended for the usual reason — postcard numbers variants by position.
    SetRigSquelch {
        frac: f32,
    },

    /// Set how the ADS-B decoder behaves (issue #160).
    ///
    /// The engine persists it to `adsb.json` and echoes it back in
    /// [`crate::RadioState::adsb`], so there is no apply step and no way for
    /// the panel's copy and the engine's to drift apart.
    ///
    /// Appended for the usual reason — postcard numbers variants by position.
    SetAdsbConfig(crate::AdsbSettings),

    /// Whether the QO-100 beacon decoder runs, and how wide it searches
    /// around [`crate::QO100_BEACON_HZ`]. The engine persists this and
    /// echoes it back in [`crate::RadioState`], so there is no apply step —
    /// the same convention [`Command::SetIsmConfig`] follows. Appended for
    /// the usual reason: postcard numbers variants by position.
    SetQo100Config(crate::Qo100Settings),

    /// Start (`true`) or stop (`false`) recording the receiver's raw I/Q to a
    /// WAV file (issue #217).
    ///
    /// Independent of [`Command::SetRecording`]: one writes what the operator
    /// *hears*, the other what the receiver *received*, and an operator
    /// capturing a band for later analysis wants the second whether or not the
    /// first is running. Both may run at once.
    ///
    /// The engine names the file — date, time, centre frequency and sample rate
    /// — and puts it beside the audio recordings. Refused on a demod-audio
    /// front end, which has no I/Q to capture.
    ///
    /// Appended for the usual reason — postcard numbers variants by position.
    SetIqRecording(bool),

    /// Set the serial number the next contest exchange will carry (issue #223).
    ///
    /// Its own command rather than a whole [`DigiConfig`](crate::DigiConfig)
    /// through `SetDigiConfig` for the same reason
    /// [`Command::SetDigiTxLevel`] is: the *engine* advances this number, once
    /// per logged contact, so every client's copy of the configuration is stale
    /// the moment a contact completes and sending one back would put the count
    /// where it was an hour ago. That makes `contest_serial` a field with a
    /// write route outside `SetDigiConfig`, which is what puts it in the
    /// engine's `keep_engine_owned` — and this is the route.
    ///
    /// Clamped to `1..=`[`crate::CONTEST_SERIAL_MAX`] engine-side: the layout's
    /// field is eleven bits, and a number outside it is not a message the far
    /// end can read.
    ///
    /// Appended for the usual reason — postcard numbers variants by position.
    SetContestSerial(u32),

    /// Switch the *radio* off (`false`), or back on again (`true`) — its own
    /// power switch, over the control link (issue #239).
    ///
    /// Not sdroxide's power switch: the tab strip's one closes the interface
    /// and leaves the radio running, and this one leaves the interface open and
    /// switches the radio off — which is exactly what an operator away from the
    /// shack wants, because the link has to survive for the switch back on to
    /// reach anything.
    ///
    /// Ignored on a front end with no such command
    /// ([`crate::DeviceCaps::commands_rig_power`]), which is every SDR: their
    /// only power switch is the USB cable.
    ///
    /// Appended for the usual reason — postcard numbers variants by position.
    SetRigPower(bool),

    /// Switch the radio's separate *receiving* antenna into the receive path
    /// (`true`), or out of it (`false`) — an IC-7300MK2's RX ANT IN/OUT, an
    /// IC-7610's RX ANT (issue #229). The transmit aerial is not touched.
    ///
    /// The one time sdroxide writes that setting: everywhere else it is read
    /// from the radio and adopted, because the radio recalls it per band and
    /// switching a receive aerial nobody asked about takes it out of use with
    /// nothing on screen to say so.
    ///
    /// Ignored where [`crate::DeviceCaps::has_rx_antenna`] is false.
    ///
    /// Appended for the usual reason — postcard numbers variants by position.
    SetRxAntenna(bool),

    /// Add channels to the memory list — a repeater directory or a channel
    /// table read from a file (issue #234).
    ///
    /// The ids the caller sends are ignored: the engine owns the numbering,
    /// because only it knows what is already stored. Channels whose frequency
    /// and mode a memory already carries are skipped rather than duplicated, so
    /// re-importing an updated directory adds what is new instead of doubling
    /// what is not.
    ///
    /// Appended for the usual reason — postcard numbers variants by position.
    ImportMemories(Vec<crate::MemoryChannel>),

    /// Set how the VDL Mode 2 decoder behaves: which of the seven channels to
    /// listen on, how hard a burst has to be, and how much log to keep.
    ///
    /// The engine persists it to `vdl2.json` and echoes it back in
    /// [`crate::RadioState::vdl2`], so there is no apply step and no way for
    /// the panel's copy and the engine's to drift apart — the same bargain
    /// [`Command::SetAdsbConfig`] strikes.
    ///
    /// Appended for the usual reason — postcard numbers variants by position.
    SetVdl2Config(crate::Vdl2Settings),

    /// Binaural (pseudo-stereo) audio on a receiver: spread the passband across
    /// the stereo image so that pitch becomes direction, and tuning a signal
    /// floats it from one ear to the other (issue #263). CW and SSB — see
    /// [`crate::Mode::binaural_audio`] — and ignored while the sub receiver has
    /// the right ear. Only the main receiver's audio is ever spread: the sub
    /// receiver *is* the other ear, so it has nothing to be placed across.
    ///
    /// Appended for the usual reason — postcard numbers variants by position.
    SetBinaural {
        rx: RxId,
        on: bool,
    },

    /// Set up the station's external transmit/receive switch: the relay board
    /// or contact closure that grounds the SDR's antenna while the station
    /// transmits, and sequences whatever else has to move with the over.
    ///
    /// Saved to `relay.json` and echoed back in
    /// [`crate::RadioEvent::StationConfig`], because the operator setting it up
    /// may be on another machine entirely and the hardware is on this one.
    ///
    /// Boxed: the configuration carries a channel table and four strings, and
    /// an enum is as big as its largest variant everywhere it is held.
    ///
    /// Appended for the usual reason — postcard numbers variants by position.
    SetRelayConfig(Box<crate::RelayConfig>),

    /// Set how the AIS decoder behaves: which of the two channels to listen on,
    /// how hard a slot has to be, how long a vessel stays on the map and how
    /// much trail it leaves.
    ///
    /// The engine persists it to `ais.json` and echoes it back in
    /// [`crate::RadioState::ais`], so there is no apply step and no way for the
    /// panel's copy and the engine's to drift apart — the same bargain
    /// [`Command::SetAdsbConfig`] strikes.
    ///
    /// Appended for the usual reason — postcard numbers variants by position.
    SetAisConfig(crate::AisSettings),

    /// Close one of the T/R switch's contacts briefly, so the operator can hear
    /// the relay and check their wiring with the transmitter cold.
    ///
    /// Refused by the driver while anything is on the air: throwing a relay
    /// under live RF is the accident the whole subsystem exists to prevent, and
    /// a test button is exactly the thing somebody presses while wondering why
    /// their transmission sounds odd.
    ///
    /// Appended for the usual reason — postcard numbers variants by position.
    TestRelay {
        channel: u8,
    },

    /// Replace the operator's own additions to the digital modes' frequency
    /// tables (issue #268), saved to `digi_presets.json` and echoed back in
    /// [`crate::RadioEvent::StationConfig`].
    ///
    /// The whole list rather than one entry: it is short, the picker edits it
    /// in place, and sending it whole means a client and the station cannot
    /// come to disagree about what is in it — the same latest-wins rule the
    /// rest of the station's configuration follows.
    ///
    /// Appended for the usual reason — postcard numbers variants by position.
    SetDigiPresets(Vec<crate::DigiPreset>),

    /// Controlled-envelope SSB: how hard voice is driven into the envelope
    /// processor, in decibels, 0 being off (issue #283). Clamped to
    /// `0..=`[`crate::CESSB_MAX_DB`], and ignored by every mode but USB and LSB.
    ///
    /// Appended for the usual reason — postcard numbers variants by position.
    SetCessb(f32),

    /// Point the front end so that a *wideband* decoder's window is centred
    /// here — the ISM window's band buttons, and anything else that means "go
    /// to this band" rather than "listen to this frequency".
    ///
    /// Not [`Command::SetVfo`], and the difference is not cosmetic. The dial is
    /// where the demodulator listens; the wideband lanes are placed from the
    /// front end's own centre frequency, and on a zero-IF receiver those are
    /// not the same place — a PlutoSDR parks its local oscillator a quarter of
    /// a span above the dial. Tuning the dial to a band centre therefore lands
    /// the window a quarter-span above the band, and on a receiver with no
    /// slack to slide the window back — one whose whole stream is barely wider
    /// than the plan — it stays there. That is issue #310: pressing **868 MHz
    /// EU** on a 2.5 Msps PlutoSDR put the window on 869.275 MHz and left
    /// 868.300 MHz outside it.
    ///
    /// The engine subtracts its own LO offset, so on a front end that has none
    /// this is exactly [`Command::SetVfo`] on the active VFO, and unlike a dial
    /// move it always retunes rather than only when the old centre had drifted
    /// out of span.
    ///
    /// Appended for the usual reason — postcard numbers variants by position.
    TuneWidebandTo(f64),

    /// A contact was entered in the log by hand — pass it on to whatever is
    /// listening for logged QSOs (issue #341).
    ///
    /// The digital modes' own contacts do not come this way: the sequencer logs
    /// them inside the engine, which broadcasts them there. What this carries
    /// is everything else — an SSB or CW contact typed into the log window —
    /// which until now reached the operator's own logbook and nothing beyond
    /// it, so a station forwarding to MacLoggerDX or N1MM saw its FT8 and
    /// nothing else.
    ///
    /// Deliberately not sent for an ADIF *import*: a file of last year's
    /// contacts is not a contact being made, and pushing a few thousand of them
    /// at a logger would be worse than useless. Nor for an *edit* — the
    /// protocol has no message for one, and re-sending the record would log it
    /// twice.
    ///
    /// Appended for the usual reason — postcard numbers variants by position.
    LogQso(Box<crate::QsoRecord>),
}
