//! WebSocket wire protocol between the sdroxide server and remote clients.
//!
//! Framing: every binary WS message is `[PROTO_VERSION_BYTE][postcard bytes]`.
//! The version byte is a fast sanity check; the real version negotiation
//! happens in `Hello`/`HelloAck`.
//!
//! Compiles for native and `wasm32-unknown-unknown`.

pub mod solar;

use serde::{Deserialize, Serialize};

use sdroxide_types::{
    CallsignInfo, Command, Decode, DeviceCaps, DigiStatus, ImageEntry, ImageKind, ImageListing,
    ImagePresets, MemoryChannel, MemoryFolder, Meters, QsoRecord, RadioState, RifpMeta, RifpStatus,
    SkimmerSpot, SpectrumFrame, Spot, SstvMode, SstvStatus, StationConfig, TleSubStatus,
    UploadResult, VoiceStatus,
};

/// Bump on any incompatible change to the message enums (this includes the
/// payload structs from `sdroxide-types` that ride the wire, e.g. `QsoRecord`).
/// v3: `QsoRecord` gained `id` + `comment` fields.
/// v4: added `ServerMsg::SkimmerSpots` + `Command::SetSkimmerEnabled` + a
/// `RadioState.skimmer_enabled` field.
/// v5: added SSTV — `Mode::Sstv`, `ServerMsg::Sstv*`, and
/// `Command::SstvTx`/`SstvSetMode`.
/// v6: added audio noise reduction + auto-notch — `Command::SetNoiseReduction`,
/// `Command::SetAutoNotch`, and `RxState.noise_reduction` / `RxState.auto_notch`.
/// v7: added keyboard modes Olivia/Thor/FSQ — new `Mode` variants, `DigiConfig`
/// submode fields (Olivia tones/bw, THOR submode, FSQ speed/call), `DigiStatus`
/// FSQ heard-list + directed-message fields, and a mode-agnostic digi image path
/// (`Command::DigiImageTx` / `RadioEvent::DigiImage` for the FSQ image sub-mode).
/// v8: added the audio recorder — `Command::SetRecording` and
/// `RadioState.recording` / `RadioState.recording_file`.
/// v9: FT8/FT4 QSO handling — `QsoStep::WaitCq` / `Confirming` and
/// `Command::DigiStartQso.wait_for_cq`.
/// v10: neural (RNNoise) noise reduction — new `NrLevel::Ai{Low,Med,High}`
/// variants can appear in `RxState.noise_reduction`.
/// v11: network cockpit — spot feeds, callsign lookup, and uploads. New
/// `Command::SetNetworkConfig`/`SpotDialHint`/`LookupCallsign`/`UploadQso`/
/// `SyncConfirmations` and `ServerMsg::Spots`/`NetStatus`/`CallsignResult`/
/// `Upload`/`Confirmations`, plus new `QsoRecord` fields.
/// v12: per-kind skimmer control — `RadioState.skimmer_enabled` became
/// `RadioState.skimmer: SkimmerSettings` (CW/PSK/RTTY enables + squelch) and
/// `Command::SetSkimmerEnabled` became `Command::SetSkimmerConfig`.
/// v13: built-in TCI server — `Command::SetTciServerConfig` and
/// `ServerMsg::TciServerStatus`.
/// v14: FreeDV Reporter — new `SpotKind::FreeDv` (extends the postcard
/// discriminant space of `ServerMsg::Spots`) and `NetworkConfig`'s new
/// `freedv_reporter` field. `NetworkConfig` also lost `my_call`/`my_grid`: the
/// operator identity is `DigiConfig`'s alone, so both ends must agree on the
/// shape `Command::SetNetworkConfig` carries.
/// v15: built-in Hamlib rigctld server — `Command::SetRigctldConfig` and
/// `ServerMsg::RigctldStatus` (both extend the postcard discriminant space).
/// v16: FT8 message handling and reporting. `Decode` gained `cq_dx` and
/// `free_text`, `TranscriptLine` gained `overheard`, and `PskConfig` gained the
/// upload fields — postcard is not self-describing, so every added field
/// changes the layout of the messages carrying them. Also new:
/// `Command::SetWsjtxConfig` (WSJT-X UDP broadcast).
/// v17: manual FT8 message control — `Command::DigiSetStep` and
/// `Command::DigiSendText` (both extend the postcard discriminant space).
/// v18: FT8 transmit watchdog — `DigiStatus.tx_watchdog` plus `DigiConfig`'s
/// `tx_watchdog_min` / `max_tx_repeats`, which both ends must agree on.
/// v19: voice keyer — `Command::VoiceRecord`/`VoicePlay`/`VoicePreview`/
/// `VoiceClear`/`VoiceRename` and `ServerMsg::VoiceStatus` (both extend the
/// postcard discriminant space).
/// v20: Hellschreiber — `ServerMsg::HellColumns` plus `DigiConfig`'s
/// `hell_variant` / `hell_rx_agc`, which both ends must agree on because
/// `DigiStatus` carries the config. (`Mode::Hell` alone would have been
/// compatible: it is appended to the enum, so no existing discriminant moves.)
/// v21: FT8 DXpedition mode — `DigiConfig`'s `dxped_mode` / `fox_slots`,
/// `DigiStatus.fox_queue`, and `Decode.rr73_to` (the RR73 half of a Fox
/// message, which is how a Hound learns its contact completed). Postcard is not
/// self-describing, so both ends must agree on every one of those fields.
/// v22: clock-offset monitoring — `DigiStatus.clock_offset_s`.
/// v23: directed CQs — `Decode.cq_dx` became `Decode.cq_to`, the modifier
/// itself (`DX`, `EU`, `JA`, `POTA`, …) rather than a single DX flag.
/// v24: the FT8/FT4 call queue — `Command::DigiQueueAdd`/`DigiQueueRemove` and
/// `DigiStatus.call_queue`.
/// v25: automatic transmit-frequency choice — `DigiConfig.auto_tx_freq`.
/// v26: RIFP (draft-dulaunoy-rifp-00) — `Mode::Rifp` and `Band::M70` (both
/// appended, so no existing discriminant moves), `Command::RifpTx` /
/// `RifpDropSession`, `ServerMsg::RifpRows` / `RifpImage` / `RifpStatus`, and
/// `DigiConfig`'s `rifp_*` fields, which both ends must agree on because
/// `DigiStatus` carries the config.
/// v27: WFM broadcast stereo — `RxState.wfm_stereo`, `Meters.stereo` and
/// `Command::SetWfmStereo`. The command is appended so no existing discriminant
/// moves, but postcard is not self-describing, so the two added struct fields
/// change the layout of every message carrying `RadioState` or `Meters`.
/// v28: JS8 — `Mode::Js8`, the `js8_*` fields on `DigiConfig`, and
/// `DigiStatus.js8` carrying the heard list, the reassembled conversation and
/// transmit-queue progress. No message enum gained a variant, but postcard is
/// not self-describing and the added struct fields change the layout of every
/// message carrying `DigiConfig` or `DigiStatus`.
/// v29: JS8 beaconing — `DigiConfig`'s `js8_hb_ack` (answer a heard heartbeat
/// with a signal report) and `js8_hb_anywhere` (beacon on the working frequency
/// instead of the 500–1000 Hz sub-band), plus `Js8Status.hb_hz`, the frequency
/// the last beacon actually went out on. `Js8Status.next_hb_in_s` is now
/// populated rather than always `None`, which is a behaviour change but not a
/// layout one. Both ends must agree on the three added fields, postcard being
/// what it is.
/// v30: broadcast station labels — new `SpotKind::Broadcast`, which extends the
/// postcard discriminant space of `ServerMsg::Spots` exactly as `FreeDv` did in
/// v14. The engine never emits it (the stations are synthesised client-side from
/// a bundled table), but the enum both ends decode has changed shape, so they
/// must agree on it.
/// v31: the full-band panadapter — a new `ServerMsg::WideSpectrum`, carrying an
/// ordinary `SpectrumFrame` on its own lane. Appended at the end of the enum so
/// no existing postcard discriminant moves, but an older client cannot decode
/// the new message, so the handshake has to reject it.
/// v32: engine notices reach remote clients — a new `ServerMsg::Notice`,
/// likewise appended at the end. What a notice says is the operator's business
/// wherever they are sitting: a radio refusing a tune, or an interface that has
/// dropped and is reconnecting, is not a local-console detail.
/// v33: the picture stores moved server-side — five new `ServerMsg`s
/// (`ImagePresets`, `ImageSlotSource`, `ImageListing`, `ImageFile`,
/// `ImageSaved`) and six new `Command`s (`ImageSetSlot`, `ImageClearSlot`,
/// `ImageSetMessage`, `ImageGetSlot`, `ImageList`, `ImageGet`), all appended so
/// no existing discriminant moves. The transmit slots, their overlay messages
/// and the received galleries used to be client state, which meant a browser
/// tab and the console attached to the same radio disagreed about both — and
/// the browser, having no filesystem, had neither. They belong to the radio:
/// the engine owns the files and hands out metadata, thumbnails and pixels on
/// request. Composition stays client-side, and transmit still rides the
/// existing `SstvTx` / `RifpTx`.
/// v34: the station configuration reaches remote clients — two new `ServerMsg`s
/// (`StationConfig`, `TleSubStatus`) and two new `Command`s (`SetSatConfig`,
/// `RefreshTleSubs`), all appended so no existing discriminant moves. The
/// network cockpit, the two built-in servers, the WSJT-X broadcast and the
/// satellite additions all describe the *station*, and all of them are files in
/// the engine host's config directory. A remote settings dialog used to read
/// its own machine's copy — nonexistent in a browser — so those tabs opened on
/// defaults, and pressing APPLY wrote the defaults back over the operator's
/// real configuration. The engine announces them instead, and the server caches
/// and replays them like the digi config.
/// v35: received pictures can be deleted — one new `ServerMsg` (`ImageDeleted`)
/// and one new `Command` (`ImageDelete`), both appended so no existing
/// discriminant moves. The store is on the engine host, so until now the only
/// way to clear out a season of half-decoded charts and noise-only frames was to
/// go to that machine with a file manager; a browser tab could not do it at all.
/// The deletion is broadcast rather than answered, because a picture that has
/// gone is gone from every gallery, not just the one that asked.
/// v36: sign-in — one new `ClientMsg` (`Auth`) and two new `ServerMsg`s
/// (`AuthRequired`, `AuthRejected`), all appended so no existing discriminant
/// moves. A server with credentials configured answers `Hello` with
/// `AuthRequired` instead of `HelloAck` and waits; everything that used to
/// happen next happens after the credentials are accepted. The version still
/// has to be bumped despite the appends: a v35 client cannot decode
/// `AuthRequired`, so it would report a protocol error rather than the truth,
/// which is that it needs a password and cannot ask for one.
/// v37: manual audio gain — `RxState.manual_gain_db` and a new
/// `Command::SetManualGain`, which is *not* appended: it sits with the other
/// receiver commands, so the discriminants after it move. Both ends must agree
/// on the added field anyway, postcard not being self-describing, so the
/// version has to be bumped either way. AGC off used to mean unity gain on the
/// demodulator's own output, which for an SSB signal 60 dB down is silence at
/// any volume setting; it now means this fixed gain instead.
/// v38: a finished FT8/FT4 contact says so — `TranscriptLine` gained `done`,
/// the flag on the line that marks the QSO complete and logged. Appended to the
/// struct, but postcard is not self-describing, so every message carrying a
/// `DigiStatus` changes layout and both ends have to agree on the field.
/// v39: `QsoStep::Done` is gone. Nothing ever set it — `Confirming` is the
/// state a finished contact sits in — so no engine has ever put it on the wire.
/// It was the last variant, so no surviving discriminant moves, but the enum
/// both ends decode has changed shape and this codebase bumps for that.
/// v40: sub-audible squelch signalling on NFM — `Meters.tone` carries the CTCSS
/// tone or DCS code being received, `RxState.tone_sql` the one the operator
/// requires before the audio gate opens, and `Command::SetToneSquelch` sets it.
/// The command is appended, so no surviving discriminant moves, but postcard is
/// not self-describing and the two added struct fields change the layout of
/// every message carrying `RadioState` or `Meters`.
/// v41: the frequency scanner — `RadioState.scan` says whether it is running
/// and whether it has stopped on something, `RadioEvent::Scanner` carries the
/// settings the way `Memories` carries the memory list, and four appended
/// commands (`SetScannerConfig`, `SetScanning`, `ScanNext`, `ScanSkip`) drive
/// it. The commands are appended, but the added `RadioState` field changes the
/// layout of every message carrying one, postcard not being self-describing.
/// v42: recording both sides of the QSO (RX left, TX right) instead of just
/// the receiver, plus an optional mono downmix — `Command::SetRecordingMono`
/// (appended) and `RadioState.recording_mono` (changes the layout of every
/// message carrying `RadioState`).
/// v43: two more noise-reduction engines. `NrLevel` gained
/// `Spec{Low,Med,High}` (a Rust port of libspecbleach's adaptive denoiser) and
/// `Df{Low,Med,High}` (DeepFilterNet3), both appended so no surviving
/// discriminant moves — but a v42 client cannot decode discriminants 7..12 at
/// all, so it would report a protocol error rather than the truth, which is
/// that the operator picked an engine it has never heard of. The `Ai*`
/// variants were also renamed `Rnn*`, which the wire cannot see (postcard is
/// positional and this enum is persisted nowhere else) but the labels can: the
/// chip reads "NR RNN Med" where it read "NR AI Med".
/// v44: WSPR. `Mode::Wspr` is appended, as is `SpotKind::Wspr`, so no surviving
/// discriminant moves — but `DigiConfig` gained five fields and `DigiStatus` a
/// `wspr: Option<WsprStatus>`, and postcard is not self-describing, so every
/// message carrying either changes layout and both ends have to agree. The new
/// `ServerMsg::WsprSpots` carries what a slot decoded: a WSPR reception is a
/// measurement of a path rather than a message somebody sent, so it travels as
/// its own event instead of being squeezed into `Decode`. `WsprStatus` also
/// carries `tx_blocked`, the engine's own answer to "can this station transmit
/// as configured" — the panel used to work that out for itself and got it
/// wrong, and one authority is the point.
/// v45: the CW skimmer's decoder is the operator's choice. `SkimmerSettings`
/// gained `cw_decoder` (DeepCW or the envelope-timing decoder) and `cw_slots`
/// (how many stations the neural one reads at once), so a receiver that cannot
/// spare the cores for a Conformer per station can still skim. Both are
/// appended, but `SkimmerSettings` is a field of `RadioState` as well as the
/// payload of `Command::SetSkimmerConfig`, and postcard is not self-describing:
/// the layout of every state broadcast changes and both ends have to agree.
/// v46: memory folders — `MemoryChannel` gained `folder`, which changes the
/// layout of every `Memories` message, postcard not being self-describing. The
/// folder list itself rides the new `ServerMsg::MemoryFolders` (appended last,
/// so no surviving discriminant moves), and four appended `Command`s
/// (`CreateMemoryFolder` / `RenameMemoryFolder` / `DeleteMemoryFolder` /
/// `MoveMemoryToFolder`) manage them.
/// v47: an RTTY memory carries its modem setup — `MemoryChannel` gained
/// `rtty: Option<RttyMemory>` (baud / shift / reverse / AFC), captured when a
/// memory is stored in RTTY mode and re-applied on recall. Changes the layout
/// of every `Memories` message, postcard not being self-describing.
/// v48: the satellite lock. Two appended `Command`s (`SetSatLock`,
/// `SetRotatorConfig`) and two appended `ServerMsg`s (`SatTrack`,
/// `RotatorStatus`), so no surviving discriminant moves — but `SatLink` gained
/// `inverting` and `StationConfig` gained `rotator`, and postcard is not
/// self-describing, so every message carrying either (`SetSatConfig`, the
/// `StationConfig` bundle) changes layout and both ends have to agree.
///
/// v49: `DeviceCaps` gained `shared_lo_rx` (a Pluto 2R2T's chains share one
/// LO). Appended field, but postcard is not self-describing, so every message
/// carrying capabilities changes layout and both ends have to agree.
/// v50: the band plan became data. `StationConfig` gained `region` (which of
/// the three IARU regions the station is in) and `band_plan` (the edges and
/// sub-segments themselves, from the engine machine's `bandplan.json`), so the
/// bundle changes layout and both ends have to agree. `Command::SetRegion` and
/// `Command::ReloadBandPlan` are appended last, so no surviving discriminant
/// moves.
/// v51: the interface configuration reaches remote clients — a new
/// `ServerMsg::RadioConfig` and a new `Command::SetRadioConfig`, both appended
/// last, so no surviving discriminant moves. What an SDR can be told about
/// itself is not all expressible as a gain stage: an RTL-SDR's AGC mode, its
/// ppm correction, its HF path and its bias tee ride `SetGain` pseudo-elements
/// to the running device, but the panel that drives them reads and writes
/// `radio.json` — a file in the engine host's config directory, which a remote
/// client had no copy of and was therefore shown "only available in the native
/// app" instead of. A headless server's dongle could only be configured by
/// editing that file by hand and restarting. The engine announces it instead,
/// and the server caches and replays it like the station config. The version
/// still has to be bumped despite the appends: a v50 client cannot decode the
/// announcement, so it would report a protocol error rather than the truth,
/// which is that the server is offering it something it has never heard of.
/// v52: Winlink radio email. Five appended `Command`s (`WinlinkConnect`,
/// `MailList`, `MailGet`, `MailCompose`, `MailDelete`, `MailMove`) and five
/// appended `ServerMsg`s (`WinlinkStatus`, `MailListing`, `MailMessage`,
/// `MailSaved`, `MailDeleted`), so no surviving discriminant moves — but
/// `NetworkConfig` gained `winlink`, and postcard is not self-describing, so
/// every message carrying it changes layout and both ends have to agree. The
/// mailbox itself stays on the engine host and is read a page and a message at
/// a time, like the picture store: mirroring a mailbox with attachments in it
/// to every client on connect is not something a phone link would survive.
/// v53: AX.25 packet radio. Two appended `Mode` variants — `Packet` (VHF/UHF
/// FM) and `PacketHf` (HF sideband) — so no surviving discriminant moves and a
/// v52 client decodes every mode it already knew. The bump is for the other
/// direction: a v53 engine on a packet mode sends a discriminant a v52 client
/// has never heard of, and postcard is not self-describing, so what it would
/// report is a protocol error rather than the truth. Same reasoning as v51's.
/// `DigiConfig` also gained the `packet_*` settings and `DigiStatus` an
/// `Option<PacketStatus>` carrying the heard list, which changes the layout of
/// every message either of them rides in — the same reason v52 had to bump for
/// `NetworkConfig`. The heard list is capped (`PACKET_HEARD_MAX`) and travels
/// whole rather than as a delta: it is a rolling view of a busy channel, not a
/// log, and a client that reconnects wants what is on the air now.
/// `WinlinkConfig` also gained `lane`, `gateway` and `gateway_via`, so the
/// operator picks the radio lane in settings rather than the client picking it
/// per connect — which keeps `Command::WinlinkConnect` unchanged and means an
/// older client still forwards by telnet instead of failing to encode a command
/// it has never heard of.
/// v54: the 4 m band. `Band::M4` is appended to the enum, so no postcard
/// discriminant moves and every band stack and memory already on disk still
/// decodes — but `Band::ALL` puts it between 6 m and 2 m, where it belongs on
/// screen, and that list's *position* is what the propagation messages index by
/// (`SolarServerMsg`'s per-band planes, and the band masks the globe and map
/// carry). A v53 client would file 2 m's plane under 4 m and 70 cm's under 2 m,
/// which is the one thing a version byte exists to prevent.
/// v55: SpyServer, as two interfaces. `Backend::SpyServer` and
/// `Backend::SpyServerVfo` are appended so no existing discriminant moves, and
/// `RadioConfig` gained a `spyserver` and a `spyserver_vfo` block. Both halves
/// force the bump, for the two reasons v51 and v53 already set out: a new
/// `Backend` discriminant is a value a v54 client has never heard of, and two
/// new `RadioConfig` fields change the layout of every message that struct
/// rides in — which is `ServerMsg::RadioConfig`, the message the settings
/// dialog is built from. Postcard is not self-describing, so a v54 client
/// would not fail to find them; it would decode the fields after them from the
/// wrong offset.
/// v56: device questions travel. A new `ClientMsg::Probe` and a new
/// `ServerMsg::ProbeAnswer`, both appended last, so no surviving discriminant
/// moves. Every other setting in `radio.json` describes a device and therefore
/// means the same thing wherever it is read; the discovery controls beside them
/// ask about a *machine* — what is on its USB bus, which serial ports it has,
/// what a broadcast finds on its network, which sound cards the rig could be
/// plugged into — and a client that answered those locally would be describing
/// the wrong computer. So they were greyed out, and with them the interface
/// selector: with no list to pick from there was nothing to choose. Now the
/// question is asked of the machine the radio is attached to and the answer
/// comes back over the session, which is what makes changing the radio on a
/// `--server` instance possible from a remote or browser client at all. The
/// version bump is for the usual reason: a v55 client cannot decode an answer
/// it has never heard of, and would report a protocol error instead of the
/// truth, which is that the server is offering it something new. `ServerMsg`
/// also gained `Capabilities`, appended after it: with the interface
/// changeable from a client, the radio a session is talking to can become a
/// different one mid-session, and what it can do has to follow.
///
/// **57** — a station serves every radio in its roster, each on its own
/// address, so a session now says which radio it is on and what else the
/// station has: [`ServerMsg::Radios`], sent once, right after `HelloAck`.
/// Without it a client could reach a second radio only if somebody typed its
/// address in by hand, and could never put the station's radios in tabs the
/// way it does with the ones plugged into this machine.
/// **58** — RDS/RBDS on WFM broadcast: [`ServerMsg::Rds`], appended last. A
/// remote client gets the station name, programme type, radio text and
/// now-playing tags the same way the local one does, because the decoding
/// happens where the radio is and only the result travels. Cached and replayed
/// on connect like the other announced-once state: the station being listened to
/// is a *condition*, not an event, and a browser tab that attaches after the
/// dial stopped moving would otherwise sit blank until the text next changed.
///
/// **59** — an RMS gateway carries its own speed and channel. `WinlinkConfig`
/// gained `gateway_baud` and `gateway_freq_hz`, so picking a 9600 gateway from
/// the list calls it at 9600 instead of in silence at 1200. `WinlinkConfig`
/// travels inside `Command::SetNetworkConfig`, and postcard is not
/// self-describing: two appended fields shift every byte after them, so a v58
/// client and a v59 server would disagree about where the gateway list starts.
/// `#[serde(default)]` covers the config file on disk, not the wire — hence
/// the bump.
///
/// **60** — the microwave bands: 1.25 m and 33 cm (Region 2's alone, like 4 m is
/// Region 1's), and 23 cm, 13 cm, 9 cm and 6 cm. `Band::M125`, `Band::Cm33`,
/// `Band::Cm23`, `Band::Cm13`, `Band::Cm9` and `Band::Cm6` are appended to the
/// enum, so no postcard discriminant moves and every band stack and memory
/// already on disk still decodes. The bump is for the same reason v54's was:
/// `Band::ALL` puts each new band where it belongs on screen — 1.25 m between
/// 2 m and 70 cm, the rest above it — and that list's *position* is what the
/// propagation messages index by (`SolarServerMsg`'s per-band planes, and the
/// band masks the globe and the flat map carry). A v59 client would file 70 cm's
/// plane under 1.25 m and every plane above it one band low.
///
/// **61** — a second radio used as this one's panadapter. `DeviceCaps` gained
/// `rx_audio_external` and `RadioConfig` gained a whole `panadapter` block, and
/// both travel over the wire — `DeviceCaps` in `ServerMsg::Capabilities`,
/// `RadioConfig` in `ServerMsg::RadioConfig` and `Command::SetRadioConfig`.
/// Both are appended last, so nothing already on disk moves, but postcard is
/// positional and not self-describing: a v60 client reading a v61
/// `RadioConfig` would run off the end of the message, and `#[serde(default)]`
/// covers the file, not the stream.
///
/// **62** — the panadapter's gestures say so. `Command::TuneInSpan` is appended
/// last, so no surviving discriminant moves, but a v61 engine would answer a
/// command it cannot decode with a protocol error. It separates tuning inside
/// the span already on screen (click, drag, wheel on the panadapter) from
/// setting the dial (`Command::SetVfo` — the readout, a memory, an external
/// controller), which are the same thing on an SDR and are not on a rig whose
/// own synthesiser is the centre of what we capture. (The semantic split has
/// since been retired — the engine answers both commands identically, see
/// `Command::TuneInSpan` — but the discriminant is on the wire and stays.)
///
/// **63** — the ISM decoder. `RadioState` gained an `ism` block, which puts it in
/// *every* state broadcast, so this is not an append a v62 client can survive:
/// it would run off the end of the first state message it received. The new
/// `ServerMsg::IsmReports` / `IsmStatus` and `Command::SetIsmConfig` are appended
/// last as usual. `IsmSettings` carries its per-family switches as a bitmask
/// rather than a fixed-length array precisely so that the *next* protocol family
/// added does not force this bump again.
///
/// **64** — the ELAD backend. `Backend`, `DeviceProbe`, `ProbeAnswer` and
/// `ReportKind` each gained a variant and `RadioConfig` an `elad` block. All
/// appends, but a v63 client asked to decode the new `Backend::Elad` — or handed
/// a `RadioConfig` with one more field on the end — has nowhere to put it, and
/// the config is what a client reads to draw the settings dialog.
///
/// **65** — the LimeSDR backend and LimeRFE control. `Backend`, `DeviceProbe`,
/// `ProbeAnswer` and `ReportKind` each gained a variant and `RadioConfig` a
/// `lime` block, which carries a nested `LimeRfeConfig` of its own. Appends
/// again, and again not survivable: a v64 client handed a `RadioConfig` with
/// the new block on the end stops at the wrong byte, and the LimeRFE settings
/// it cannot decode are the ones that decide whether an amplifier is switched
/// into the transmit path.
///
/// **66** — the transmit tone can be held. `DigiConfig` gained `hold_tx_freq`,
/// and `DigiConfig` travels inside `DigiStatus`, so this is not an append a v65
/// client can survive: postcard is positional and every field after it shifts.
/// `#[serde(default)]` covers the config file on disk, not the wire. The engine
/// gained `Settings.tx_guard_offset`, but that is local and not on the wire.
///
/// **67** — the transmit tone is remembered per band. `DigiConfig` gained
/// `tx_audio_hz`, a map from `Band` to the offset last chosen there, and the
/// same reasoning as 66 applies unchanged: it sits inside `DigiStatus`,
/// postcard is positional, and a v66 client reading a v67 status runs into the
/// wrong field. `#[serde(default)]` again covers the config file and not the
/// wire. The engine's `digi_tx_band` is local state and is not sent.
///
/// **68** — the rig's own power-output meter. `TxMeters` gained `po`, a
/// `0.0..=1.0` fraction of the rig's own scale, and `TxMeters` is not the last
/// field of `Meters`: `stereo` and `tone` follow it. So this is not even an
/// append a client can stop short of — every byte after `tx` shifts, and a v67
/// client handed a v68 `Meters` with the transmitter keyed reads the new
/// `Option` tag as `stereo` and then runs off the end of the message. Same
/// reasoning as v27 and v40, which bumped for exactly this struct.
///
/// The ALC reading that landed alongside it is NOT a wire change: it went into
/// `TxTelemetry`, which the engine folds into `TxMeters::alc` — an existing
/// field — and which never leaves the process on its own.
///
/// **69** — the SoapySDR settings panel. `DeviceCaps` gained `bandwidths`,
/// `bandwidth_ranges` and `settings`, so that a driver's own controls can be
/// drawn from what the device says about itself rather than from per-driver
/// code. They are appended last and `#[serde(default)]`, but postcard is not
/// self-describing and `DeviceCaps` is sent whole: a v68 client would stop
/// reading where the struct used to end and treat the remainder as the next
/// message. Same reasoning as every other `DeviceCaps` append.
///
/// **70** — the flrig CAT family. `CatConfig` gained `flrig_addr` and
/// `CatFamily` a trailing `Flrig` variant. The variant alone would be safe —
/// postcard numbers variants by declaration index, and nobody sends a value
/// they don't have — but the field sits mid-struct and `CatConfig` rides
/// `Command::SetRadioConfig` whole, so for a v69 peer every byte after it
/// shifts. `#[serde(default)]` covers the config file on disk, not the wire.
///
/// **71** — the serial CI-V scope (issue #96 on USB). `CatConfig` gained
/// `scope` and `scope_span`, appended last — but postcard is not
/// self-describing and `CatConfig` rides `Command::SetRadioConfig` whole, so a
/// v70 peer stops reading where the struct used to end and treats the two new
/// fields as the next message. Same reasoning as v69's `DeviceCaps` append.
///
/// **72** — the embedded rtl_433 ISM decoders. Four changes, any one of which
/// would force the bump: `IsmSettings` gained an `rtl433` block and it rides
/// inside every `RadioState`, so a v71 peer misreads everything after it;
/// `IsmStatus` gained a trailing `rtl433` field, and postcard is not
/// self-describing, so the old reader stops where the struct used to end;
/// `IsmProtocol` gained `Rtl433` and `IsmQuantity` four energy quantities,
/// appended so existing discriminants keep their numbers, but a v71 client
/// handed one has nowhere to put it. No new `ServerMsg` or `Command`: the lane
/// reuses `IsmReports`, `IsmStatus` and `SetIsmConfig`.
///
/// **73** — the station's roster is editable from a client. Three appended
/// `ClientMsg`s (`AddRadio`, `RemoveRadio`, `RenameRadio`), which alone would
/// be survivable — postcard numbers variants by declaration index and nobody
/// sends a message they don't have — but `ServerMsg::Radios` gained `editable`
/// mid-variant, and postcard is not self-describing: a v72 client reading it
/// would take the flag for the start of the next message. Configuring a
/// station's radios from away already worked; what did not was *how many* it
/// has, which meant a headless station's second radio could only be added by
/// editing `radios.json` on that machine and restarting it.
///
/// **74** — HamQTH joins eQSL/QRZ/Club Log as an upload target. Two appended
/// fields and two appended variants, and the fields are what force the bump:
/// `NetworkConfig` gained `auto_upload_hamqth` and `QsoRecord` gained
/// `hamqth_sent`, both of which ride whole inside `Command::SetNetworkConfig`
/// and `RadioEvent::Ft8QsoLogged`, so a v73 peer stops reading where each
/// struct used to end and takes the new field for the start of the next
/// message. `UploadTarget` and `LoginTarget` gained `HamQth` last, so existing
/// discriminants keep their numbers, but a v73 client handed one has nowhere to
/// put it. No new `ServerMsg` or `Command`: the lane reuses `SetNetworkConfig`,
/// `UploadQso` and `TestLogin`.
///
/// **75** — a radio's power switch reaches a client. `RadioInfo` gained
/// `enabled`, and it rides inside `ServerMsg::Radios` ahead of that message's
/// `editable` flag, so a v74 client reading the announcement would take the
/// new field for the roster's next entry. Appended beside it:
/// `ClientMsg::SetRadioEnabled`, and `Command::ReopenSource` — which the
/// station sends its own engine, but which shares the discriminant space every
/// command rides in. Switching a radio off already worked at the station; what
/// did not was doing it from away, which meant the browser client — a headless
/// station's *only* screen — could not put a radio down and pick it back up.
///
/// **76** — a stored memory can be edited (issue #138).
/// `Command::EditMemory` carries the channel's new name, dial and mode.
/// Appended last, so no surviving discriminant moves and a v75 *engine* would
/// simply never be sent one — but the handshake is an equality test, and the
/// client that does send one has no way to find out beforehand that the
/// station on the other end has never heard of it. Correcting a typo used to
/// mean deleting the memory and storing it again from the frequency itself.
///
/// The sort that landed with it is not on the wire at all: which order a
/// screen lists its memories in is a client preference (`UiSettings`), the
/// store keeps them in the order they were stored, and the engine is never
/// told.
///
/// **77** — repeater operation (issue #137). [`sdroxide_types::RadioState`]
/// carries a `RepeaterState`: the transmit shift, the CTCSS/DCS tone that goes
/// out under the voice, and the 1750 Hz burst. `Command::SetRepeater` and
/// `Command::ToneBurst` set and fire them, `Command::EditMemory` gained the
/// setup a memory stores, and `MemoryChannel` gained the field it stores it in.
///
/// A field added to a struct in the state, so this is not a variant append that
/// an older peer could simply never be sent: a v76 client decoding a v77 state
/// would read the tail of it as garbage. The handshake is an equality test and
/// catches that before a single frame is exchanged, which is what it is for.
/// **78** — the ISM decoder's rtl_433 lane gained a chosen bandwidth and a
/// fifth band (issue #141). `Rtl433Settings` carries `bandwidth_hz`, which is
/// the width to watch or zero for the band's own, and `Rtl433Status` carries
/// `rate_hz`, the width actually being decoded. Both are fields added to
/// structs — the settings ride in [`sdroxide_types::RadioState`] and the status
/// in an event — so a v77 peer would read the tail of either as garbage, and
/// the handshake's equality test is what stops it trying.
///
/// The 345 MHz band itself is not on the wire: it is a row in a table both
/// sides already have.
///
/// **79** — dragging the panadapter past the end of the captured window moves
/// the window (issue #133). [`sdroxide_types::DeviceCaps`] carries
/// `center_is_dial`, which says whether the front end has a centre that can be
/// commanded on its own; a client needs it to know whether the pan may ask for
/// one, or whether the dial it is already turning moves the window by itself.
/// A field appended to a struct that rides in an event, so a v78 client would
/// read the tail of the capabilities as garbage — the handshake's equality test
/// is what stops it trying.
///
/// **80** — the banner across the top of a transmitted SSTV/RIFP picture is
/// the operator's to write (issue #145). [`sdroxide_types::DigiConfig`] gained
/// `sstv_banner`, `sstv_banner_left`, `sstv_banner_right`, `sstv_banner_fill`,
/// `sstv_banner_ink` and `sstv_banner_height`: what the strip prints at each
/// end, what colour it and its text are, and how tall it is. The two texts
/// carry `{call}` / `{grid}` / `{version}` placeholders, resolved by whichever
/// client composes the picture.
///
/// On the station rather than in this screen's `UiSettings`, where the
/// waterfall's own colours live, because this one is drawn *into* the picture
/// that goes on the air — it is the station identifying itself, and the
/// browser tab and the console have to compose the same slot identically.
///
/// Fields added to a struct that rides in both a command and an event, so a
/// v79 peer would read the tail of either as garbage; the handshake's equality
/// test is what stops it trying.
///
/// **81** — the HydraSDR RFOne is an interface of its own (issue #144).
/// [`sdroxide_types::Backend`] gained `HydraSdr` and
/// [`sdroxide_types::RadioConfig`] a `hydrasdr` block, with `DeviceProbe`,
/// `ProbeAnswer` and `ReportKind` gaining the variants that enumerate one and
/// fetch its session trace.
///
/// A field appended to the radio configuration, which rides in both a command
/// and an event, so a v80 peer would read the tail of either as garbage — the
/// handshake's equality test is what stops it trying. The new `Backend` variant
/// on its own would have been harmless (an older peer is simply never sent
/// one); the config block is not.
///
/// **82** — a LimeSDR's second receive chain can carry a second aerial, and
/// the two are combined (issue #98). [`sdroxide_types::LimeConfig`] gained an
/// `aux` block — [`sdroxide_types::LimeAuxConfig`] — carrying what the chain is
/// for, its socket and gain, and the adaptive filter's mode, length, rate and
/// hold.
///
/// A field appended to the radio configuration, which rides in both a command
/// and an event, so a v81 peer would read the tail of either as garbage — the
/// handshake's equality test is what stops it trying.
/// **83** — that second chain can instead carry a directional coupler, and
/// linearise the transmitter from a sample of what it emitted — PureSignal
/// (issue #98). [`sdroxide_types::LimeAuxRole`] gained a `PureSignal` variant
/// and [`sdroxide_types::LimeAuxConfig`] the `ps_bins`, `ps_rate` and
/// `ps_frozen` that drive the correction.
///
/// Fields appended to the radio configuration, which rides in both a command
/// and an event, so a v82 peer would read the tail of either as garbage — the
/// handshake's equality test is what stops it trying. The new role variant
/// would have been survivable on its own; the fields are not.
///
/// **84** — a PlutoSDR can key an external amplifier, LNA or transmit-receive
/// switch from its own GPO pins (issue #135). [`sdroxide_types::PlutoConfig`]
/// gained `duplex` ([`sdroxide_types::PlutoDuplex`]) and `ptt_gpo`
/// ([`sdroxide_types::PlutoPtt`]): which duplex the AD9361's enable state
/// machine runs in, and which pair of pins follows the radio.
///
/// Fields appended to the radio configuration, which rides in both a command
/// and an event, so a v83 peer would read the tail of either as garbage — the
/// handshake's equality test is what stops it trying.
/// v85: DRM (Digital Radio Mondiale) — `Mode::Drm`, `ServerMsg::Drm` carrying
/// `DrmStatus`, and `Command::SetDrmService` / `SetDrmConstellation`. All are
/// appended, so no existing discriminant moves.
///
/// **86** — a rig's I/Q sound card gets the front-end correction every other
/// quadrature interface here already had (issue #147).
/// [`sdroxide_types::CatConfig`] gained `iq_correction` and `iq_dc_block_hz`:
/// whether the mirror image and the DC spike are cancelled, and how wide a
/// notch to take out of the centre.
///
/// Fields in the radio configuration, which rides in both a command and an
/// event, so a v85 peer would read the tail of either as garbage — the
/// handshake's equality test is what stops it trying.
///
/// **87** — an RSPduo can run *both* of its tuners and combine them, the same
/// way a LimeSDR's second chain is combined at v82 (issue #153).
/// [`sdroxide_types::SdrPlayConfig`] gained a `diversity` block —
/// [`sdroxide_types::SdrPlayDuo`] — carrying whether the second tuner
/// runs at all, its own two gains, and the adaptive filter's mode, length,
/// rate and hold.
///
/// A field appended to the radio configuration, which rides in both a command
/// and an event, so a v86 peer would read the tail of either as garbage — the
/// handshake's equality test is what stops it trying.
///
/// **88** — QRP Labs radios (QMX, QMX+, QDX) are a CAT family of their own
/// (issue #95). [`sdroxide_types::CatFamily`] gained a trailing `QrpLabs`
/// variant.
///
/// Nothing moved: postcard numbers variants by declaration index and the new
/// one went on the end, so every existing family keeps its number and a v87
/// peer's own configuration still round-trips. What forces the bump is the
/// other direction — a v87 client handed `QrpLabs` has no such variant to put
/// it in, and postcard is not self-describing, so it does not fail on the field
/// but on everything after it. Same reasoning as v72's `IsmProtocol` append.
///
/// **89** — APRS is a mode of its own (issue #150). Three appends, all at the
/// tail of their type: [`sdroxide_types::Mode`] gained `Aprs`,
/// [`sdroxide_types::Command`] gained `AprsBeacon` and `AprsSendMessage`, and
/// [`sdroxide_types::DigiStatus`] gained an `aprs` field carrying the map, the
/// messages and the channel.
///
/// The `Mode` append is the one that forces the bump, and it is the same trap
/// as v88's: postcard numbers variants by declaration index, so every existing
/// mode keeps its number — but a v88 client handed `Aprs` has no variant to
/// decode it into and desynchronises on everything after it, rather than
/// failing on the field itself.
///
/// **90** — an RSPduo's two tuners can be two *radios* rather than two aerials
/// combined, and the diversity filter's controls have moved to the main strip
/// (issue #165). [`sdroxide_types::SdrPlayDuo`] gained `role`
/// ([`sdroxide_types::SdrPlayDuoRole`]): whether the second tuner is combined
/// with the first or handed to a radio of its own.
/// [`sdroxide_types::DeviceCaps`] gained `diversity`, which is what tells the
/// strip that a filter is running without it having to know which backend has
/// one.
///
/// Fields appended to the radio configuration and to the capabilities. Both
/// ride the wire — the configuration in a command and an event, the
/// capabilities in an event of their own — so a v89 peer would read the tail
/// of any of them as garbage; the handshake's equality test is what stops it
/// trying.
///
/// The block itself is written under a new name with this version — `duo`
/// rather than `diversity`, since combining is no longer all it does — which
/// is invisible here: postcard numbers fields by position and never sends a
/// name. It is `radio.json` that the old name matters to, and a serde alias
/// keeps those files loading.
///
/// **91** — the panadapter's horizontal resolution is the client's to ask for
/// (issue #172). [`sdroxide_types::SpectrumConfig`] gained `bins`: how many
/// columns the emitted frames carry, which until now was a constant 2048 on
/// both sides of the wire and so never needed saying.
///
/// It is the *client* that knows the number — its screen's pixel width and its
/// own renderer's limits — so it has to travel, and it travels in the same
/// command as the FFT size and the frame rate. Inserted after `fft_size`
/// rather than appended, because it belongs next to it: postcard numbers
/// fields by position, so a v90 peer would read `bins` as `fps` and everything
/// after it as garbage either way. The handshake's equality test is what stops
/// it trying.
///
/// **92** — the waterfall's *time* axis is the client's to ask for too, and the
/// engine now clocks it. [`sdroxide_types::SpectrumConfig`] gained
/// `rows_per_sec`, and [`sdroxide_types::SpectrumFrame`] gained `rows`: the
/// waterfall lines the engine clocked since the last frame, each of them the
/// per-bin peak over its own slice of time.
///
/// Until now a frame *was* a row, so the waterfall could not advance faster
/// than the screen redrew — a fast scroll simply wrote the same numbers two or
/// three times and the operator saw lines two or three pixels tall, while a
/// front end streaming megahertz had hundreds of transforms a second going
/// spare. The two rates are now separate all the way down the wire.
///
/// The frame also gained `rows_clocked`, which is not the same as `rows` being
/// non-empty and has to be said separately: below the frame rate most frames
/// carry no rows at all, and a client that read that as "this lane does not
/// clock rows" would scroll them on its own wall clock *as well* and run the
/// waterfall at twice the rate of its own time labels. `false` is the shape for
/// a lane that really cannot clock them — a radio's own sweep, a transmit
/// monitor — and only then does the client scroll on its own.
///
/// All three ride in the frame or the config, both of which cross the wire, and
/// `rows_per_sec` sits after `display_bins` rather than at the tail, so a v91
/// peer would desynchronise on any of them. The handshake's equality test is
/// what stops it trying.
///
/// **93** — the digital modes' transmit-audio level split in two.
/// [`sdroxide_types::DigiConfig`] lost `tx_audio_level` and gained
/// `tx_audio_level_fm` and `tx_audio_level_ssb`: the one number was doing two
/// unrelated jobs — deviation into an FM rig, drive into a sideband rig's
/// modulator — and a level set for 1200 baud packet was quietly taking 8 dB off
/// FT8 as well, which is issue #131's symptom by another road.
///
/// The configuration rides the wire in a command and comes back in an event, so
/// one field becoming two moves everything after it: postcard numbers fields by
/// position, and a v92 peer would read the new sideband level as the field that
/// used to follow and the rest as garbage. The handshake's equality test is what
/// stops it trying. `digi.json` is migrated rather than versioned — the old key
/// carries into both new ones on load, so nobody's signal changes level.
///
/// **94** — packet radio grew an operator's terminal, so the panel needs to know
/// more about the link than the callsign at the far end.
/// [`sdroxide_types::PacketStatus::link`] changes from `Option<String>` to
/// `Option<PacketLink>`: the state machine's own name for where it is, the peer,
/// the digipeater path, the sequence width, the frames outstanding, the retry
/// count against N2, and which of the two things that can drive one link is
/// driving it — the packet panel or the MAIL window. That last one is the answer
/// to "why was I refused", which an operator otherwise has to guess at.
///
/// `term` and `term_partial` are new beside it: the session's lines, and the
/// tail of one that has arrived without its terminator. The tail is carried
/// separately because it is the most important thing on the screen — a BBS
/// prompt has no carriage return after it, so a terminal that printed only whole
/// lines would sit showing nothing while the far end waited for an answer to a
/// question the operator never saw.
///
/// [`sdroxide_types::Command`] gains `PacketConnect`, `PacketSend`,
/// `PacketDisconnect` and `PacketTermClear`, appended for the usual reason, and
/// [`sdroxide_types::DigiConfig`] gains `packet_connect_text`,
/// `packet_connect_via` and `packet_ext_seq` beside the packet settings they
/// belong with rather than at the tail — postcard numbers fields by position and
/// a v93 peer desynchronises on either placement, so they go where they read.
/// The handshake's equality test is what stops it trying. `digi.json` needs no
/// migration: all three carry `#[serde(default)]`, so a config written by v93
/// loads unchanged.
///
/// **95** — the digital modes' transmit-audio level is per mode (issue #186).
/// [`sdroxide_types::DigiConfig`] gains `tx_audio_levels`, a map from
/// [`Mode`](sdroxide_types::Mode) to the level the operator set for it, placed
/// beside the two carrier levels it overrides rather than at the tail — the same
/// reasoning as v94's placement, since postcard numbers fields by position and a
/// v94 peer desynchronises either way. `DigiConfig` rides inside `DigiStatus` and
/// inside `Command::SetDigiConfig`, so this is not an append a v94 client can
/// survive; the handshake's equality test is what stops it trying.
///
/// The two carrier levels **stay**, as the level for a mode with no entry of its
/// own, which is what makes this the rare wire change that needs no config
/// migration at all: an empty map is exactly v94's behaviour, so nobody's signal
/// changes level on the update, and a mode appended in a later release inherits
/// the level the operator actually runs instead of springing back to full scale.
///
/// [`sdroxide_types::Command`] gains `SetDigiTxLevel`, appended for the usual
/// reason. It carries the mode rather than letting the engine read it off the
/// dial, because the control is a rail dragged while transmitting and a mode
/// change landing mid-drag would otherwise write one mode's level onto another's.
///
/// [`sdroxide_types::DeviceCaps`] gains `cw_audio_keyed`, appended: a client has
/// to know whether CW leaves as audio or as text over the control port before it
/// can decide whether that level reaches CW at all.
///
/// **96** — xHE-AAC decoding for DRM. Two changes to `DrmStatus`, which rides in
/// [`ServerMsg::Drm`].
///
/// [`sdroxide_types::DrmCodec`] lost its `Celp` and `Hvxc` variants. Both were
/// withdrawn from the DRM standard and neither can be signalled, and the two
/// slots they occupied now mean Opus and "reserved" — so the old table read an
/// Opus service as CELP. Postcard numbers a fieldless enum by declaration
/// order, so every remaining variant moved: a v95 peer would read xHE-AAC as
/// HVXC.
///
/// [`sdroxide_types::DrmService`] gained `codec_supported`, which is not
/// appended but sits beside `codec` where it reads. A locked receiver whose
/// codec has no decoder reports a healthy signal, a service label and silence,
/// and this is the only field that distinguishes that from an audio decode
/// that is merely failing.
/// **97** — converter overload is a reading of its own (issue #173).
/// [`sdroxide_types::Meters`] gained `adc_clip`, the fraction of converter
/// samples sitting at full scale, beside the `adc_peak_dbfs` it completes
/// rather than at the tail — postcard numbers fields by position and a v96 peer
/// desynchronises on either placement, so it goes where it reads.
///
/// `adc_peak_dbfs` was on the wire from the start and had never been measured:
/// the engine filled it with a literal `0.0`. It now carries the real peak, so
/// a v96 client shown a v97 reading would not merely mis-parse the tail — it
/// would read a field that used to be a constant. The handshake's equality test
/// is what stops it trying.
///
/// Two figures rather than one because neither answers alone. The peak cannot
/// tell a signal that fills the converter from one twice too large for it, and
/// the fraction saturates as soon as a constant-envelope signal passes √2 of
/// full scale — every sample of an FM carrier clips from there on. Together
/// they say whether the front end is into its rails and roughly how far.
///
/// `Meters` rides in an event only, never in a command, so this is a one-way
/// change: nothing a client sends carries it.
/// **98** — an IC-9700 on macOS, three ways (issue #192).
///
/// `Mode::SstvFm` — SSTV on an FM carrier, the way slow-scan is sent on VHF
/// and UHF — is appended to [`sdroxide_types::Mode`]. Appended, so every mode
/// already on the wire keeps its number; but the variant itself is new, and a
/// v97 peer handed one decodes it as nothing at all.
///
/// [`sdroxide_types::RadioState`] gained `rig_squelch`, the radio's *own*
/// squelch threshold as a `0..1` fraction of its scale, and
/// [`sdroxide_types::Command`] gained `SetRigSquelch` to move it. A rig that
/// hands sdroxide audio it has already gated has the only squelch that can
/// open — `Command::SetSquelch` is a threshold on what got through — so the
/// SQL control follows [`sdroxide_types::DeviceCaps::commands_squelch`], also
/// new here, to whichever of the two the front end actually has.
///
/// Three appended fields and one appended variant: postcard numbers both by
/// position, so a v97 peer desynchronises on the tail of every `RadioState`
/// and every `DeviceCaps` regardless. The handshake's equality test is what
/// stops it trying.
///
/// **99** — ADS-B (issue #160).
///
/// `Mode::Adsb` is appended to [`sdroxide_types::Mode`], which is on its own
/// enough to force this bump: every mode already on the wire keeps its number,
/// but a v98 peer handed the new one has no variant to decode it into and
/// desynchronises on the rest of the message. That is the same reasoning as
/// v98's `Mode::SstvFm` and v89's `Mode::Aprs`.
///
/// With it: `ServerMsg::AdsbStatus` carrying the aircraft table,
/// `Command::SetAdsbConfig` to change how the decoder behaves, and
/// [`sdroxide_types::RadioState::adsb`] holding what it was set to — all
/// appended, all at the end of their respective enums and structs.
///
/// The table is re-sent whole a couple of times a second rather than
/// incrementally, like the ISM device table and for the same reasons; the
/// per-aircraft position history is `f32` (about two metres at any latitude)
/// because it is the bulk of that message and a history dot on a map has no use
/// for more.
///
/// **100** — one appended field, `AdsbStatus::degraded` (issue #160).
///
/// A receiver can be able to run the ADS-B decoder and still be too narrow to
/// carry the waveform properly — below about 2.4 Msps a Mode S chip and a
/// sample are the same width, and the aircraft at the edge of range are lost to
/// arithmetic rather than to propagation. That is not the same statement as
/// `unavailable`, which means nothing is running at all, so it is a field of
/// its own rather than a reuse.
///
/// Appended at the end of the struct; postcard numbers fields by position, so a
/// v99 peer desynchronises on the tail of every `AdsbStatus` and the
/// handshake's equality test is what stops it trying.
///
/// **101** — the QO-100 beacon calibration decoder.
///
/// [`sdroxide_types::RadioState`] gained `qo100`
/// ([`sdroxide_types::Qo100Settings`]) holding whether the decoder runs and how
/// wide it searches, and [`sdroxide_types::Command`] gained `SetQo100Config` to
/// move it — the ISM decoder's shape, no apply step, the engine echoes the
/// setting back in the state. Both are appended at the tail of their type, so
/// every field and variant already on the wire keeps its number; a v100 peer
/// desynchronises on the tail of every `RadioState` regardless, and the
/// handshake's equality test is what stops it trying.
///
/// The live side — [`sdroxide_types::Qo100Status`] (lock, measured offset,
/// decoded telemetry text) — is a native-engine `RadioEvent` only and never
/// reaches the wire: `sdroxide-server` maps it to no `ServerMsg`, the same as
/// the spectrum lanes that travel their own way. So there is no new `ServerMsg`
/// here, and a remote client's window shows that the reading is local-only
/// rather than sitting on "starting…". If it is bridged later that is its own
/// append and its own bump.
/// **102** — RTTY on an FM carrier (issue #214).
///
/// `Mode::RttyFm` is appended to [`sdroxide_types::Mode`]: the same Baudot
/// modem, the same tone pair and the same panel as `Mode::Rtty`, into an FM
/// transmitter rather than onto a sideband. Appending a mode is on its own
/// enough to force this bump — every mode already on the wire keeps its number,
/// but a v101 peer handed the new one has no variant to decode it into and
/// desynchronises on the rest of the message. Same reasoning as v99's
/// `Mode::Adsb`, v98's `Mode::SstvFm` and v89's `Mode::Aprs`.
///
/// Nothing else changed: every table the new mode needs an answer in is
/// derived from the mode, not carried beside it.
/// **103** — recording the raw I/Q (issue #217).
///
/// [`sdroxide_types::Command`] gained `SetIqRecording`, and
/// [`sdroxide_types::RadioState`] gained `iq_recording`, `iq_recording_file`
/// and `iq_recording_mb` beside the audio recorder's three, so a remote client
/// can start a capture and watch it grow. Both appended at the tail of their
/// type, so every variant and field already on the wire keeps its number; a
/// v102 peer desynchronises on the tail of every `RadioState` regardless, and
/// the handshake's equality test is what stops it trying.
///
/// The file is written by the *engine*, on the machine the receiver is plugged
/// into — a remote client's capture lands on the station, not on the laptop
/// that asked for it, because the alternative is a gigabyte a minute over the
/// link. That is a property of the feature and not of the wire, so nothing here
/// carries the samples.
/// **104** — NAVTEX (issue #212).
///
/// `Mode::Navtex` is appended to [`sdroxide_types::Mode`] and
/// [`sdroxide_types::DigiStatus`] gains `navtex`
/// ([`sdroxide_types::NavtexStatus`]) beside the other per-mode panes, carrying
/// the messages received, the one arriving and the loose text.
/// [`sdroxide_types::DigiConfig`] gains `navtex_reverse`, the tone-sense
/// control — the mode's only setting, since there is nothing to transmit and no
/// callsign to give.
///
/// Appending a mode is on its own enough to force the bump, for the reason
/// v102's `Mode::RttyFm` and v99's `Mode::Adsb` were: a v103 peer handed the
/// new one has no variant to decode it into and desynchronises on the rest of
/// the message.
/// **105** — EU VHF contest operation (issue #223).
///
/// [`sdroxide_types::DigiConfig`] gains `contest` (a new `ContestMode` enum)
/// and `contest_serial`, and [`sdroxide_types::Command`] gains
/// `SetContestSerial` — the number's own write route, because the engine
/// advances it as each contact is logged and a client's copy of the
/// configuration is stale the moment one completes.
///
/// Both config fields are appended at the tail of the struct and the command at
/// the tail of the enum, so every field and variant already on the wire keeps
/// its number; a v104 peer desynchronises on the tail of every `DigiConfig`
/// regardless — it rides inside `Command::SetDigiConfig` and
/// `DigiStatus.config` — and the handshake's equality test is what stops it
/// trying.
///
/// The `i3 = 5` message layout the mode transmits is not on this wire at all:
/// it is packed and unpacked inside the engine, and what crosses the link is
/// the decoded text, exactly as for every other 77-bit layout.
/// **106** — browsing the public-SDR directories.
///
/// [`sdroxide_types::DeviceProbe`] gains `PublicSdrs { refresh }` and
/// [`sdroxide_types::ProbeAnswer`] gains `PublicSdrs`, carrying a
/// [`sdroxide_types::PublicSdrDirectory`] — the KiwiSDR and SpyServer listings,
/// fetched by the machine the radio is attached to.
///
/// The probe lane rather than a route of its own, and for one reason: a browser
/// client has no HTTP client and could not reach either directory across
/// origins if it had. Asking the station is the only way the web UI gets this
/// feature at all, and the station is also the end that will hold the
/// connection, so it is the end that should be reading the list.
///
/// [`sdroxide_types::Backend`] gains `KiwiSdr` and [`sdroxide_types::RadioConfig`]
/// gains `kiwi` ([`sdroxide_types::KiwiConfig`]) at its tail. Both are appended,
/// so every variant and field already on the wire keeps its number — but a v105
/// peer has no `KiwiSdr` variant to decode into and desynchronises on the rest
/// of any `RadioConfig`, which is what the handshake's equality test prevents.
/// Same shape of bump as v81's HydraSDR and v64/v65's ELAD and Lime.
/// **107** — zooming the panadapter out past the I/Q.
///
/// [`sdroxide_types::DeviceCaps`] gains `wide_span_hz` at its tail: how wide the
/// front end's full-band lane is, or zero where it has none.
///
/// On the capabilities rather than on the frame because it is what bounds the
/// client's zoom-out, and a client that had to wait for a picture to learn it
/// would spend the first frames of every session believing the passband was the
/// limit — long enough to shrink a restored window to it. The *position* of the
/// lane still rides each frame, where it belongs: that moves with the receiver.
///
/// Appended, so every field already on the wire keeps its number; `DeviceCaps`
/// is sent whole, so a v106 peer desynchronises on the tail of it regardless,
/// which is what the handshake's equality test prevents.
/// **108** — the antenna a memory channel was stored on.
///
/// [`sdroxide_types::MemoryChannel`] gains `antenna` at its tail: which socket
/// the radio was on when the channel was stored, so recalling a 2 m repeater
/// puts the beam back and recalling a 40 m net puts the wire back (issue #235).
/// `None` there means "leave the antenna alone", which is deliberately not how
/// the neighbouring `repeater` field reads an absent value — see the field.
///
/// Appended, so every field already on the wire keeps its number; the memory
/// list is sent whole, so a v107 peer desynchronises on the tail of any channel
/// regardless, which is what the handshake's equality test prevents.
/// **109** — switching the radio itself off and on again.
///
/// [`sdroxide_types::Command`] gains `SetRigPower(bool)` and
/// [`sdroxide_types::DeviceCaps`] gains `commands_rig_power` at its tail: the
/// radio's own power switch, over its control link, on a front end that has one
/// (issue #239).
///
/// The command variant is appended, so every variant already on the wire keeps
/// its number, and the caps field likewise; `DeviceCaps` is sent whole, so a
/// v108 peer desynchronises on the tail of it regardless, which is what the
/// handshake's equality test prevents.
/// **110** — importing a channel list.
///
/// [`sdroxide_types::Command`] gains `ImportMemories(Vec<MemoryChannel>)`: a
/// repeater directory or a channel table read from a CHIRP CSV file, appended
/// to the station's memories (issue #234). Appended, so every variant already
/// on the wire keeps its number.
/// **111** — the antenna a memory is stored on becomes a field the operator can
/// set, rather than only whatever the radio happened to be on when the channel
/// was stored (issue #246).
///
/// [`sdroxide_types::Command`]'s `EditMemory` gains `antenna`, beside the
/// `repeater` setup it already carried, and reads an absent one the same way
/// [`sdroxide_types::MemoryChannel::antenna`] does: recall this channel without
/// moving the antenna. A field added to an existing variant rather than a new
/// variant, so a v110 peer would decode the tail of one as garbage — which the
/// handshake's equality test catches before a frame is exchanged.
/// **112** — a memory scan can be pointed at chosen folders (issue #236).
///
/// [`sdroxide_types::ScannerConfig`] gains `folders`, the folders a
/// [`sdroxide_types::ScanKind::Memories`] scan runs over — empty for all of
/// them, which is what every existing setting means. The config rides both ways
/// (`Command::SetScannerConfig`, `RadioEvent::Scanner`), so a v111 peer would
/// read the tail of it as garbage; the handshake's equality test is what stops
/// one ever seeing it.
/// **113** — a memory scan can read its channels off the spectrum (issue #228).
///
/// [`sdroxide_types::ScannerConfig`] gains `mem_fast`, which puts every channel
/// inside one receiver window on the transform the panadapter is already made
/// from and visits only the ones something is on. Appended to the same struct
/// as 112's `folders`, and with the same consequence: it rides both ways, so
/// the handshake's equality test is what keeps a v112 peer from reading the
/// tail of it as garbage.
/// **114** — VDL Mode 2 (the datalink half of what an aeroplane broadcasts).
///
/// `Mode::Vdl2` is appended to [`sdroxide_types::Mode`], which on its own is
/// enough to force this bump for the reason v99's `Mode::Adsb` and v104's
/// `Mode::Navtex` were: a v113 peer handed the new one has no variant to decode
/// it into and desynchronises on the rest of the message.
///
/// With it: `ServerMsg::Vdl2Status` carrying the message log, the station table
/// and what each of the seven channels is doing, `Command::SetVdl2Config` to
/// change how the decoder behaves, and [`sdroxide_types::RadioState::vdl2`]
/// holding what it was set to — all appended, all at the end of their respective
/// enums and structs.
///
/// The log and the table are re-sent whole a couple of times a second rather
/// than incrementally, like the ADS-B aircraft table and for the same reasons; a
/// dropped snapshot costs nothing because the next carries the same
/// information, and a client that connects mid-session gets the whole log
/// rather than whatever happens to arrive next.
/// **115** — stated tuning ranges follow the interface they were stated for
/// (issue #254).
///
/// [`sdroxide_types::RadioConfig`] gains `freq_ranges_parked`, the RX/TX ranges
/// belonging to interfaces this radio is not on at the moment — what
/// `RadioConfig::set_backend` puts the old interface's numbers into when a tab
/// is moved to another device, so that a public receiver's published coverage
/// stops clamping the transceiver that was there before it. Appended to the end
/// of the struct, like every field above it, and the struct rides
/// `ServerMsg::RadioConfig` and `Command::SetRadioConfig` whole: a v114 peer
/// would read the tail of one as garbage, which is what the handshake's
/// equality test stops before a frame is exchanged.
/// **116** — binaural (pseudo-stereo) CW audio (issue #263).
///
/// [`sdroxide_types::RxState`] gains `binaural`, and with it
/// `Command::SetBinaural`. The field is appended to the end of the struct and
/// the command to the end of its enum, as everything above them is — but
/// `RxState` rides inside `RadioState`, which is sent whole a few times a
/// second, so a v115 peer would read the tail of every state message one byte
/// out. That is what the handshake's equality test stops before a frame is
/// exchanged.
///
/// Nothing else on the wire moves: the widener runs on the *speaker* path of
/// whichever end is doing the listening, and the audio a remote client is sent
/// is the mono downmix, which binaural leaves untouched by construction.
/// **117** — the station's external transmit/receive switch (issue #227).
///
/// A relay board, a handshake line or a sound-card GPIO pin that grounds the
/// SDR's antenna while the station transmits, and sequences an amplifier with
/// it. [`sdroxide_types::StationConfig`] gains `relay`, `Command` gains
/// `SetRelayConfig` and `TestRelay`, and `ServerMsg` gains `RelayStatus`.
///
/// The command and the message are appended to the end of their enums as
/// always, but the config field is the reason this is a bump rather than a
/// free addition: `StationConfig` rides `RadioEvent::StationConfig` and
/// `ServerMsg::StationConfig` **whole**, so a v116 peer would read the tail of
/// every station bundle one field out — and that bundle carries the band plan
/// the client draws its band edges from. The handshake's equality test stops
/// that before a frame is exchanged.
/// **118** — the RigExpert Fobos SDR backend (issue #273).
///
/// [`sdroxide_types::Backend`] gains `Fobos`, [`sdroxide_types::DeviceProbe`]
/// and [`sdroxide_types::ProbeAnswer`] each gain a `Fobos` variant that
/// enumerates one, and [`sdroxide_types::RadioConfig`] gains a `fobos` block.
///
/// The three enum variants are appended to the end of their own enums as
/// always, so no surviving discriminant moves and none of them is the reason
/// for this bump. The config field is: `RadioConfig` rides
/// `ServerMsg::RadioConfig` and `Command::SetRadioConfig` **whole**, so a v117
/// peer handed one with a `FobosConfig` on the end has nowhere to put it and
/// reads the tail of every one of those messages out of step — the same
/// not-survivable addition every backend before this one made (v64 ELAD, v65
/// LimeSDR). `#[serde(default)]` rescues a `radio.json` written before this
/// existed, because JSON is self-describing; postcard is not, so it rescues
/// nothing on the wire. The handshake's equality test is what stops that
/// before a frame is exchanged.
///
/// v119: independent sideband — [`sdroxide_types::Mode::Isb`], appended to the
/// end of `Mode` so no existing discriminant moves. Nothing in the message set
/// changed shape, and that is exactly why the bump is needed: postcard decodes
/// a discriminant it has never heard of as an error, so a v118 client handed a
/// `RadioState` whose mode is 36 fails the whole frame rather than one field.
/// The handshake refuses the pairing instead (issue #280).
///
/// v120: the transverter table — [`sdroxide_types::RadioConfig`] gains
/// `transverters`, a list with a band and an offset per box. `RadioConfig`
/// rides `ServerMsg::RadioConfig` and `Command::SetRadioConfig` **whole**, so a
/// v119 peer handed one with a list on the end reads the tail of every one of
/// those messages out of step — the same not-survivable addition every config
/// field before it made (issue #278).
///
/// v121: where the antenna is — [`sdroxide_types::RadioConfig`] gains
/// `rx_site`, which says whether this radio listens on the station's own
/// antenna or on somebody else's, and where that one is. Reception reports are
/// posted from it rather than from the operator's locator (issue #284). Same
/// not-survivable shape change as every config field before it: `RadioConfig`
/// rides `ServerMsg::RadioConfig` and `Command::SetRadioConfig` whole, so a
/// v120 peer handed one with an enum on the end reads the tail of every one of
/// those messages out of step.
///
/// v122: the VDL2 decoder's two new counters (issue #265).
/// [`sdroxide_types::Vdl2Status`] gains `hdlc_bad` and `fec_bypassed`, which
/// say whether the data field unwrapped and whether the frame came through on
/// its check sequence alone. The struct rides `ServerMsg::Vdl2Status` whole and
/// is re-sent a couple of times a second, so a v121 peer would read the tail of
/// every one of them out of step.
///
/// v123: who sent the picture — [`sdroxide_types::SstvStatus`] gains `rx_id`,
/// the callsign a station sent as an FSK ID after its transmission (issue
/// #287). The struct rides `ServerMsg::SstvStatus` whole and is re-sent several
/// times a second while the mode is up, so a v122 peer handed one with an
/// option on the end reads the tail of every one of them out of step.
///
/// v124: the operator's own frequencies —
/// [`sdroxide_types::StationConfig`] gains `digi_presets`, the list a station
/// has added to the digital modes' own tables, and
/// [`sdroxide_types::Command`] gains `SetDigiPresets` to edit it (issue #268).
/// The config rides `ServerMsg::StationConfig` whole, so a v123 peer handed one
/// with a list on the end reads the tail of it out of step; the command is
/// appended last, so no surviving discriminant moved.
///
/// v125: controlled-envelope SSB — [`sdroxide_types::TxState`] gains
/// `cessb_db`, and [`sdroxide_types::Command`] gains `SetCessb` to move it
/// (issue #283). `TxState` sits in the middle of `RadioState`, which travels
/// whole on every change, so a v124 peer desynchronises on everything after it;
/// the command is appended last, so no surviving discriminant moved.
///
/// v126: PureSignal on an HPSDR board —
/// [`sdroxide_types::HpsdrConfig`] gains `puresignal`, `ps_bins`, `ps_rate`
/// and `ps_frozen` at its tail (issue #283). `HpsdrConfig` is part of
/// `RadioConfig`, which rides `ServerMsg::RadioConfig` and
/// `Command::SetRadioConfig` whole, so a v125 peer handed one with four fields
/// on the end of that block reads the tail of every one of those messages out
/// of step.
///
/// v127: the QO-100 spectral tracker — [`sdroxide_types::Qo100Settings`] gains
/// `park_lo_hz`, `park_hi_hz`, `decode_telemetry` and `auto_apply`, the
/// parking window the tracker searches and the two things it may do with what
/// it finds (issue #291). The struct sits inside `RadioState` with `vdl2`
/// after it, and `RadioState` rides `ServerMsg::State` whole on every change,
/// so a v126 peer handed one with four fields in the middle of that block
/// reads `vdl2` — and the tail of every state update — out of step. It also
/// rides `Command::SetQo100Config`, which a remote client sends whole when the
/// operator touches any of the page's chips.
///
/// [`sdroxide_types::Qo100Status`] grows in the same commit, but it is not on
/// the wire yet: `sdroxide_server` maps `RadioEvent::Qo100Status` to `None`
/// and the QO-100 readout is local to the receiving station.
/// v128: AIS — the ship-reporting system on the two 162 MHz channels.
///
/// `Mode::Ais` is appended to [`sdroxide_types::Mode`], which on its own is
/// survivable in one direction and not the other: postcard writes an enum's
/// discriminant positionally, so a v127 peer handed a mode it has no variant
/// for fails to decode the message carrying it — and `Mode` is inside
/// `RxState`, inside `RadioState`, which travels whole on every change.
///
/// With it: `ServerMsg::AisStatus` carrying the vessel table and what each
/// channel is doing, `Command::SetAisConfig` to change how the decoder
/// behaves, and [`sdroxide_types::RadioState::ais`] holding what it was set to
/// — the settings field appended at the tail of `RadioState`, so a v127 peer
/// would read the end of every state update out of step. Both the message and
/// the command are appended last in their enums, so no surviving discriminant
/// moved.
///
/// v129: per-band transmit drive calibration —
/// [`sdroxide_types::RadioConfig`] gains `tx_drive_trim` at its tail, the
/// table that makes one Drive setting mean one output power on every band
/// (issue #295). `RadioConfig` rides `ServerMsg::RadioConfig` and
/// `Command::SetRadioConfig` whole, so a v128 peer handed one with a field on
/// the end reads the tail of every one of those messages out of step.
///
/// v130: operator-defined open-collector control words —
/// [`sdroxide_types::HpsdrConfig`] gains `oc_table` at its tail and
/// [`sdroxide_types::HpsdrFilterBoard`] gains a `Custom` variant (issue #296).
/// The block sits in the middle of `RadioConfig`, which rides
/// `ServerMsg::RadioConfig` and `Command::SetRadioConfig` whole, so a v129 peer
/// reads every field after `hpsdr` out of step; and the enum's new variant is
/// appended, so no surviving discriminant moved but a v129 peer handed one
/// fails to decode the message carrying it.
///
/// v131: the SDRplay gain controls say what they are and what they may reach.
/// [`sdroxide_types::GainElement`] gains a `unit`, because decibels are wrong
/// for one of them: an RSP's RF gain is a step into a band-dependent table and
/// a control that appended dB to it reported a number three times too small.
/// The struct rides `DeviceCaps` inside `ServerMsg::Capabilities`, which an RSP
/// now re-sends on every retune — the ladder's length belongs to the band — so
/// a v130 peer would read the tail of each one out of step rather than merely
/// missing a field.
///
/// With it, [`sdroxide_types::SdrPlayConfig`] gains `hdr_bw` (which filter the
/// RSPdx's HDR path runs, previously left at the API's default because nothing
/// ever wrote it) and `extended_if_gr` (whether the IF gain reduction may go
/// below the API's 20 dB floor). Both sit at that block's tail, and
/// `RadioConfig` rides `ServerMsg::RadioConfig` and `Command::SetRadioConfig`
/// whole, so a v130 peer handed one reads the tail of each of those out of step.
///
/// v132: [`ServerMsg::CapabilitiesUpdated`], which says the front end revised
/// what it said about itself rather than that there is a different one. The
/// distinction was free while the only thing that moved mid-session was an
/// antenna list a rig answered for once; the RSP's band-dependent LNA ladder
/// republishes on every crossing of a band edge, and a client that reads each
/// of those as a new radio throws away its wideband waterfall mid-QSY. The
/// variant is appended, so no surviving discriminant moved — but a v131 peer
/// handed one fails to decode the message carrying it.
/// v133: [`sdroxide_types::KiwiConfig`] gains `wf_zoom`, how far the
/// receiver's own waterfall is zoomed into its band (issue #303). `RadioConfig`
/// rides `ServerMsg::RadioConfig` and `Command::SetRadioConfig` whole and
/// postcard describes nothing about itself, so a v132 peer handed one reads the
/// tail of every field after it out of step — the serde default that covers a
/// stored `radio.json` does nothing on the wire.
/// v134: `Command::TuneWidebandTo`, which points the front end so a wideband
/// decoder's *window* lands on a frequency rather than its dial (issue #310).
/// The ISM band buttons send it instead of `Command::SetVfo`. Appended, so no
/// surviving discriminant moved — but a v133 peer handed one fails to decode.
/// v135: the VDL2 channel plan is the whole 25 kHz raster rather than every
/// other slot of it, so [`sdroxide_types::Vdl2Settings::channels`] is fourteen
/// bits and not seven (issue #265). A `u8` on the wire where a `u16` is
/// expected does not simply carry the wrong channels — postcard varints them,
/// so a v134 peer reads the following fields out of step.
/// v136: [`sdroxide_types::RadioConfig`] gains `rx_audio_gain_db`, a fixed trim
/// on the receive audio for a rig whose own codec is quiet — the AF rail's top
/// is unity and could only turn such a radio down (issue #315). It sits among
/// that struct's sound-card fields, near its head, and `RadioConfig` rides
/// `ServerMsg::RadioConfig` and `Command::SetRadioConfig` whole, so a v135 peer
/// handed one reads almost every field after it out of step.
/// v137: the receiving antenna. [`sdroxide_types::DeviceCaps`] gains
/// `has_rx_antenna` and [`sdroxide_types::RadioState`] gains `rx_antenna` — an
/// Icom's `0x12` reply carries that connector's in/out flag behind the socket,
/// and sdroxide had been writing the byte as zero on every antenna command,
/// switching an operator's receive aerial out of circuit on every band change
/// (issue #229). Both fields sit at their struct's tail, and both structs ride
/// whole inside `ServerMsg::Capabilities`, `ServerMsg::CapabilitiesUpdated` and
/// `ServerMsg::State`, so a v136 peer handed one reads the tail of every one of
/// those out of step rather than merely missing a field. With them,
/// [`sdroxide_types::Command`] gains `SetRxAntenna`, appended, so no surviving
/// discriminant moved — but a v136 peer handed one fails to decode the message
/// carrying it.
pub const PROTO_VERSION: u16 = 137;
const VERSION_BYTE: u8 = 0x12;

#[derive(Debug, thiserror::Error)]
pub enum ProtoError {
    #[error("empty message")]
    Empty,
    #[error("unsupported protocol version byte {0:#x}")]
    Version(u8),
    #[error("decode error: {0}")]
    Decode(#[from] postcard::Error),
}

/// Audio codec for one stream direction, negotiated at Hello time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioCodec {
    /// 20 ms Opus frames, 48 kHz mono.
    Opus48kMono,
    /// Little-endian PCM16, 48 kHz mono (fallback when WebCodecs is missing).
    Pcm16_48k,
}

/// What the client can encode/decode (browser WebCodecs availability).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioCaps {
    pub opus_decode: bool,
    pub opus_encode: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ClientMsg {
    Hello {
        proto: u16,
        audio: AudioCaps,
    },
    Command(Command),
    /// 20 ms mic frame in the codec negotiated at Hello.
    MicFrame {
        seq: u32,
        payload: Vec<u8>,
    },
    Ping(u64),
    /// Answer to [`ServerMsg::AuthRequired`]. Appended last on purpose:
    /// postcard encodes the variant as a positional discriminant, so inserting
    /// anywhere else would silently renumber every message after it.
    Auth {
        username: String,
        password: String,
    },
    /// A question about the machine the radio is attached to — which dongles
    /// are on its bus, which serial ports it has, whether an address answers
    /// from there. Answered with [`ServerMsg::ProbeAnswer`].
    ///
    /// Appended last, for the reason above. Only read from a signed-in session,
    /// like every other command: these open sockets and scan buses on the
    /// server's behalf, and the client that may do that is the one that may
    /// already point the radio at any address it likes.
    Probe(sdroxide_types::DeviceProbe),
    /// Put another radio in the station's roster. `name` is the operator's
    /// name for it, empty for the usual case of one named after whatever
    /// interface it ends up configured as.
    ///
    /// Not a [`Command`]: commands go to *a* radio's engine, and a station's
    /// roster belongs to the station. The server acts on this itself, brings
    /// the new radio up on its own address, and announces the whole roster
    /// again ([`ServerMsg::Radios`]) — which is how the client that asked, and
    /// every other client on the station, finds out.
    ///
    /// The radio arrives with no interface (`Backend::None`), exactly as one
    /// added at the station itself does: what it is comes next, from the Radio
    /// settings page, which a remote client can already drive.
    AddRadio {
        name: String,
    },
    /// Take a radio out of the station's roster. Its configuration is kept on
    /// disk, as it is when a radio is closed at the station itself — this is
    /// closing a radio, not destroying what it was set up as.
    ///
    /// The station's first radio is refused: it holds the shared network
    /// services and the legacy configuration, and `/ws` has to keep meaning
    /// something.
    RemoveRadio {
        id: u32,
    },
    /// Record the operator's name for one of the station's radios. Empty puts
    /// it back on the default — named after its interface.
    ///
    /// On the wire rather than kept on the client because the roster is a file
    /// on the *station*: a name typed here and remembered only here would be
    /// gone at the next reconnect, and invisible to everybody else on it.
    RenameRadio {
        id: u32,
        name: String,
    },
    /// Switch one of the station's radios on or off — the same switch the
    /// station's own tab strip carries, thrown from away.
    ///
    /// Not a [`Command`], for the same reason as the three above: this decides
    /// whether a radio has a front end at all, which is the station's business
    /// and not that radio engine's. The station writes its roster, has the
    /// engine rebuild its front end from it — an interface opened, or let go —
    /// and announces the roster again, which is how the client that asked and
    /// every other client on the station find out.
    SetRadioEnabled {
        id: u32,
        on: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ServerMsg {
    HelloAck {
        proto: u16,
        caps: DeviceCaps,
        state: RadioState,
        /// Codec of server→client RX audio.
        rx_codec: AudioCodec,
        /// Codec expected for client→server mic frames.
        tx_codec: AudioCodec,
    },
    State(RadioState),
    Spectrum(SpectrumFrame),
    Meters(Meters),
    Memories(Vec<MemoryChannel>),
    /// The scanner's settings, replayed on connect and re-sent on every change,
    /// exactly as `Memories` is.
    Scanner(sdroxide_types::ScannerConfig),
    RxAudio {
        seq: u32,
        payload: Vec<u8>,
    },
    Pong(u64),
    /// Another client already holds the (single) session.
    Busy,
    Error(String),
    // FT8/FT4 digital modes.
    Ft8Decodes(Vec<Decode>),
    Ft8Status(DigiStatus),
    Ft8QsoLogged(QsoRecord),
    // Skimmers (CW etc.).
    SkimmerSpots(Vec<SkimmerSpot>),
    // SSTV image mode.
    SstvLine {
        image_id: u32,
        y: u16,
        rgb: Vec<u8>,
    },
    SstvImage {
        image_id: u32,
        mode: SstvMode,
        w: u16,
        h: u16,
        png: Vec<u8>,
    },
    SstvStatus(SstvStatus),
    // Weather fax (receive only).
    WefaxLine {
        image_id: u32,
        y: u16,
        gray: Vec<u8>,
    },
    WefaxImage {
        image_id: u32,
        w: u16,
        h: u16,
        png: Vec<u8>,
    },
    WefaxStatus(sdroxide_types::WefaxStatus),
    // RIFP image mode.
    /// Reassembled raster rows of an incoming picture (grayscale, `w` per row).
    RifpRows {
        image_id: u32,
        y: u16,
        w: u16,
        h: u16,
        rows: Vec<u8>,
    },
    /// A completed, digest-verified picture (PNG bytes) and its manifest facts.
    RifpImage {
        image_id: u32,
        meta: RifpMeta,
        png: Vec<u8>,
    },
    RifpStatus(RifpStatus),
    /// FSQ image: a completed received picture (PNG bytes).
    DigiImage {
        png: Vec<u8>,
    },
    /// Hellschreiber: a batch of received dot columns, column-major, 0 = black.
    /// `seq` is the absolute column index so a client can detect a dropped
    /// batch — this lane drops rather than blocks when it backs up, and Hell has
    /// no framing of its own to resynchronise against.
    HellColumns {
        seq: u64,
        rows: u8,
        cols: Vec<u8>,
    },
    /// Voice keyer: slot contents plus what is being recorded or transmitted.
    VoiceStatus(VoiceStatus),
    // Network cockpit.
    Spots(Vec<Spot>),
    NetStatus(Option<String>),
    CallsignResult(CallsignInfo),
    Upload(UploadResult),
    Confirmations(Vec<QsoRecord>),
    /// Built-in TCI server status (listener up, bind address, client count).
    TciServerStatus {
        running: bool,
        addr: String,
        clients: usize,
        error: Option<String>,
    },
    /// Built-in rigctld server status, so the settings dialog on a remote
    /// client can show what the engine's listener is doing.
    RigctldStatus {
        running: bool,
        addr: String,
        clients: usize,
        error: Option<String>,
    },
    /// Full-band spectrum from a direct-sampling front end. Appended last on
    /// purpose: postcard encodes the variant as a positional discriminant, so
    /// inserting anywhere else would silently renumber every message after it.
    WideSpectrum(SpectrumFrame),
    /// A non-fatal operator notice from the engine, `None` to clear it — the
    /// radio refusing a tune, an interface reconnecting. Unlike
    /// [`ServerMsg::Error`] the session is intact and the client stays live.
    /// Appended last, for the reason above.
    Notice(Option<String>),
    /// The transmit-image presets, announced at startup and on every change.
    /// Cached by the server and replayed on connect, like the digi config and
    /// the voice keyer — without it a browser tab opens on five empty slots
    /// beside a console showing five full ones, and there is no second
    /// announcement to wait for.
    ImagePresets(ImagePresets),
    /// A preset's stored source picture, answering `Command::ImageGetSlot`.
    ImageSlotSource {
        slot: u8,
        version: u32,
        png: Vec<u8>,
    },
    /// One page of a received store, answering `Command::ImageList`.
    ImageListing(ImageListing),
    /// One received picture at full size, answering `Command::ImageGet`. An
    /// empty `png` means the store does not have it.
    ImageFile {
        kind: ImageKind,
        name: String,
        png: Vec<u8>,
    },
    /// A freshly received picture has been stored, as a gallery would list it.
    ImageSaved(ImageEntry),
    /// What the station is set up to do: the network cockpit, the two built-in
    /// servers, the WSJT-X broadcast and the satellite additions. Cached by the
    /// server and replayed on connect, like the digi config — these are files
    /// in the engine host's config directory, and a client on another machine
    /// has no other way to learn them.
    StationConfig(Box<StationConfig>),
    /// What each TLE subscription's cached listing holds. Replayed on connect
    /// beside the config it annotates.
    TleSubStatus(Vec<TleSubStatus>),
    /// A received picture has been deleted from the store, answering
    /// `Command::ImageDelete`. Sent to whichever client is attached, whether or
    /// not it is the one that asked.
    ImageDeleted {
        kind: ImageKind,
        name: String,
    },
    /// This server wants a username and password. Sent in place of
    /// [`ServerMsg::HelloAck`] once `Hello` has been read and its version
    /// accepted — after, so a client on the wrong protocol is told *that*
    /// rather than being asked to sign in to a server it could not talk to
    /// anyway. The client answers with [`ClientMsg::Auth`].
    ///
    /// Nothing else is sent, read or acted on until the credentials are
    /// accepted: not the capabilities, not the state, and above all not the
    /// single-client slot, which is claimed only afterwards so that a stranger
    /// cannot lock the operator out of their own radio by connecting to it.
    AuthRequired,
    /// Those were not the credentials, and why not. The socket stays open so
    /// the operator can correct a typo without redialling — but the server
    /// takes its time before it will judge another attempt.
    AuthRejected(String),
    /// What a WSPR slot decoded.
    ///
    /// Not `Ft8Decodes`: a WSPR reception is a measurement of a path, not a
    /// message addressed to anyone, and it carries the transmit power and drift
    /// that make it one. Squeezing it into `Decode` would have meant throwing
    /// both away. Appended last, so no surviving discriminant moves.
    WsprSpots(Vec<sdroxide_types::WsprSpot>),
    /// The memory folders, replayed on connect and re-sent on every change,
    /// exactly as `Memories` is. Appended last, for the usual reason.
    MemoryFolders(Vec<MemoryFolder>),
    /// What the satellite lock is doing — look angles, range, the Doppler
    /// corrections as applied. The latest one is cached by the server and
    /// replayed on connect, so a client arriving mid-pass sees the lock
    /// immediately rather than at the next tick. `None` when the lock is
    /// released. Appended last, for the usual reason.
    SatTrack(Option<Box<sdroxide_types::SatTrackStatus>>),
    /// The rotctld client's health, mirrored from the engine's
    /// `RadioEvent::RotatorStatus`. Appended last, for the usual reason.
    RotatorStatus {
        connected: bool,
        az_deg: f64,
        el_deg: f64,
        error: Option<String>,
    },
    /// Which radio interface the engine host has open, and how every backend on
    /// that machine is configured — its `radio.json`. Cached by the server and
    /// replayed on connect, like the station config, and for the same reason:
    /// it is a file in *that* machine's config directory, so a client here has
    /// no other copy, and a settings panel opened on defaults would write those
    /// defaults back over the operator's real configuration.
    ///
    /// Appended last, for the usual reason.
    RadioConfig(Box<sdroxide_types::RadioConfig>),

    // ── Winlink radio email ──
    //
    // Appended for the usual reason: postcard numbers variants by position.
    WinlinkStatus(sdroxide_types::WinlinkStatus),
    MailListing(sdroxide_types::MailListing),
    /// Boxed: a message carries its attachments, and every other variant here
    /// would otherwise be as large as the biggest mail we can hold.
    MailMessage(Box<sdroxide_types::MailMessage>),
    MailSaved(String),
    MailDeleted {
        folder: sdroxide_types::MailFolder,
        mid: String,
    },
    /// What this machine found, answering [`ClientMsg::Probe`].
    ///
    /// No request id: the answer names what it is an answer to, probes are run
    /// one at a time in the order they arrive, and the socket preserves that
    /// order — so a second Rescan cannot be overtaken by the first.
    ProbeAnswer(Box<sdroxide_types::ProbeAnswer>),
    /// The radio's capabilities changed under a live session: the engine opened
    /// a different front end.
    ///
    /// These used to ride [`ServerMsg::HelloAck`] alone, on the reasoning that
    /// a session's radio is the one it connected to. That stopped being true
    /// when the interface became changeable from here — swap a dongle for a
    /// transceiver and the gain stages, the antenna ports, the tuning ranges
    /// and whether there is a transmitter at all are different, and a client
    /// still drawing the old ones would be offering controls the radio does not
    /// have. It also covers the case that was always possible: an interface
    /// that was not there at startup and attached later.
    Capabilities(DeviceCaps),
    /// Which radio this session is on, and every radio the station serves.
    ///
    /// Sent immediately after `HelloAck`, and again to every session whenever
    /// the roster changes. A station has as many radios as its roster says,
    /// each reached at an address of its own, and nothing else on the wire
    /// would tell a client that the machine it just dialled has a second one —
    /// leaving the operator to find out by reading the server's log. With it, a
    /// client that can hold several radios at once opens the rest beside the
    /// one it asked for, and follows the station when a radio is added or
    /// taken away — by this client, by another one, or at the station itself.
    ///
    /// Appended last, like everything before it: postcard encodes the variant
    /// as a positional discriminant, so inserting anywhere else would silently
    /// renumber every message after it.
    Radios {
        /// The id of the radio this session is on, out of `radios` below.
        ///
        /// A roster that does *not* list it is how a client is told that the
        /// radio it is on has just been taken out of the station: the roster
        /// goes out on the reliable lane before the session is dropped, so the
        /// tab can close itself rather than sit on a socket that is about to
        /// shut and offer to dial an address that is now a 404.
        me: u32,
        radios: Vec<RadioInfo>,
        /// Whether this station accepts roster edits from here —
        /// [`ClientMsg::AddRadio`] and the two beside it. False for a station
        /// whose host wired none of that up (or a client's own in-process
        /// engine, which is not addressed this way at all), and the client
        /// then leaves the controls off rather than offering buttons that
        /// would be quietly ignored.
        editable: bool,
    },
    /// What the RDS/RBDS decoder has made of the WFM station on the main
    /// receiver. A snapshot, except for the group log inside it, which is a
    /// delta the client accumulates — see [`sdroxide_types::RdsData`].
    Rds(sdroxide_types::RdsData),
    /// Every ISM device heard, as a whole table. A snapshot — see
    /// [`sdroxide_types::IsmReport`].
    IsmReports(Vec<sdroxide_types::IsmReport>),
    /// Which ISM channels are live, and what the burst gate is seeing.
    IsmStatus(sdroxide_types::IsmStatus),
    /// What the DRM decoder has made of the broadcast on the main receiver.
    /// A snapshot — see [`sdroxide_types::DrmStatus`].
    Drm(sdroxide_types::DrmStatus),
    /// Every aircraft the ADS-B decoder is tracking, plus what the demodulator
    /// is seeing and why it is not running when it is not. A whole snapshot,
    /// twice a second — see [`sdroxide_types::AdsbStatus`].
    AdsbStatus(Box<sdroxide_types::AdsbStatus>),
    /// Everything the VDL Mode 2 decoder has: the message log, the station
    /// table, what each channel of the plan is doing, and why it is not running
    /// when it is not. A whole snapshot, twice a second — see
    /// [`sdroxide_types::Vdl2Status`].
    Vdl2Status(Box<sdroxide_types::Vdl2Status>),
    /// The station's external T/R switch, mirrored from the engine's
    /// `RadioEvent::RelayStatus`: whether one is configured, whether the
    /// hardware is answering, what it is, and whether the contacts are in their
    /// transmit state.
    ///
    /// Cached by the server and replayed on connect — unlike
    /// [`ServerMsg::RotatorStatus`], which is not, and which therefore leaves a
    /// client that attaches mid-session with a blank panel. This is a standing
    /// condition and a safety one: an operator opening the settings dialog
    /// remotely has to be told at once that the relay protecting the receiver
    /// is not answering, rather than after the next thing that happens to
    /// change.
    ///
    /// Appended last, for the usual reason.
    RelayStatus(Box<sdroxide_types::RelayStatus>),
    /// Every vessel the AIS decoder is tracking, plus what both channels are
    /// doing and why it is not running when it is not. A whole snapshot, twice
    /// a second — see [`sdroxide_types::AisStatus`].
    AisStatus(Box<sdroxide_types::AisStatus>),
    /// The *same* front end, revising what it said about itself — as against
    /// [`ServerMsg::Capabilities`], which is a different one.
    ///
    /// Two things a device says about itself are not constants. A rig on a
    /// control port cannot say whether it has an antenna selector until the
    /// link has been round; an RSP's LNA ladder is as long as the *band*
    /// allows, so its length moves under the dial. Both have to reach the
    /// client, and neither means the radio changed — read as a new front end,
    /// a tune across a band edge wipes the client's wideband waterfall, makes
    /// it re-read every image store and silences its announcer, in the middle
    /// of a QSY.
    ///
    /// Appended last, for the usual reason.
    CapabilitiesUpdated(DeviceCaps),
}

/// One radio in a station's roster, as a client sees it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RadioInfo {
    /// The station's own id for it, which is also what addresses it: `/ws/<id>`.
    /// Not a position in the list — deleting a radio must not renumber the
    /// others under whatever a client saved.
    pub id: u32,
    /// What to call it: the operator's name where they gave one, otherwise
    /// what its interface calls itself, and failing both its number.
    pub name: String,
    /// Whether `name` is the operator's own, rather than one the station
    /// derived from the radio's interface.
    ///
    /// The difference matters to a client that opens the radio: a derived name
    /// goes stale the moment the interface changes — and every radio added from
    /// away starts out as "No radio", because that is what a radio with no
    /// interface yet calls itself. A client that knows the name was derived
    /// leaves its tab unnamed and derives one of its own from the radio it is
    /// then connected to, which follows the interface the way the station's own
    /// tab strip does.
    pub named: bool,
    /// Whether the radio is switched on, where this station holds a switch a
    /// client may throw ([`ClientMsg::SetRadioEnabled`]). `None` where it does
    /// not — a host that wired none of it up — and the client then shows no
    /// switch rather than one that would be quietly ignored.
    ///
    /// A radio that is off keeps its engine, its scope and its address: its
    /// interface is simply not opened, which is what lets the device go and
    /// leaves the radio one button away from coming back.
    pub enabled: Option<bool>,
}

pub fn encode<T: Serialize>(msg: &T) -> Result<Vec<u8>, ProtoError> {
    Ok(postcard::to_extend(msg, vec![VERSION_BYTE])?)
}

pub fn decode<'a, T: Deserialize<'a>>(bytes: &'a [u8]) -> Result<T, ProtoError> {
    match bytes {
        [] => Err(ProtoError::Empty),
        [VERSION_BYTE, rest @ ..] => Ok(postcard::from_bytes(rest)?),
        [v, ..] => Err(ProtoError::Version(*v)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdroxide_types::ImageSlotInfo;

    #[test]
    fn roundtrip_client_and_server_msgs() {
        let msgs = [
            ClientMsg::Hello {
                proto: PROTO_VERSION,
                audio: AudioCaps { opus_decode: true, opus_encode: false },
            },
            ClientMsg::Command(Command::SetPtt(true)),
            ClientMsg::MicFrame { seq: 7, payload: vec![1, 2, 3] },
        ];
        for m in &msgs {
            let bytes = encode(m).unwrap();
            let back: ClientMsg = decode(&bytes).unwrap();
            assert_eq!(&back, m);
        }

        let m = ServerMsg::State(RadioState::default());
        let bytes = encode(&m).unwrap();
        let back: ServerMsg = decode(&bytes).unwrap();
        assert_eq!(back, m);

        // The station-roster edits, and the announcement that answers them.
        // Appended variants, so this is also where a discriminant slip in the
        // three of them would show.
        let roster = [
            ClientMsg::AddRadio { name: String::new() },
            ClientMsg::RemoveRadio { id: 3 },
            ClientMsg::RenameRadio { id: 3, name: "The Pluto".into() },
            ClientMsg::SetRadioEnabled { id: 3, on: false },
        ];
        for m in &roster {
            let bytes = encode(m).unwrap();
            let back: ClientMsg = decode(&bytes).unwrap();
            assert_eq!(&back, m);
        }
        let announced = ServerMsg::Radios {
            me: 1,
            radios: vec![
                RadioInfo {
                    id: 0,
                    name: "Signal generator".into(),
                    named: false,
                    enabled: Some(true),
                },
                RadioInfo { id: 1, name: "The Pluto".into(), named: true, enabled: None },
            ],
            editable: true,
        };
        let bytes = encode(&announced).unwrap();
        assert_eq!(decode::<ServerMsg>(&bytes).unwrap(), announced);

        // SSTV image/status messages round-trip (binary pixel payloads).
        let sstv = [
            ServerMsg::SstvLine { image_id: 3, y: 7, rgb: vec![1, 2, 3, 4, 5, 6] },
            ServerMsg::SstvImage {
                image_id: 3,
                mode: SstvMode::Martin1,
                w: 320,
                h: 256,
                png: vec![0x89, 0x50, 0x4e, 0x47],
            },
            ServerMsg::SstvStatus(SstvStatus {
                tx_mode: SstvMode::Robot36,
                detected: Some(SstvMode::Scottie2),
                // The FSK ID a station sent after its picture, which travels
                // as a whole string rather than a code — it is a callsign.
                rx_id: Some("OE1XYZ".to_string()),
                ..SstvStatus::default()
            }),
        ];
        for m in &sstv {
            let bytes = encode(m).unwrap();
            let back: ServerMsg = decode(&bytes).unwrap();
            assert_eq!(&back, m);
        }

        // The scanner's settings cross the link whole, skip lists included — a
        // remote client is where a range scan gets its SKIP pressed.
        let scanner = ServerMsg::Scanner(sdroxide_types::ScannerConfig {
            kind: sdroxide_types::ScanKind::Range,
            skip: vec![3, 9],
            skip_freq_hz: vec![145_312_500.0, 145_600_000.0],
            skip_freq_for: (144_000_000.0, 146_000_000.0, 12_500.0),
            ..Default::default()
        });
        let bytes = encode(&scanner).unwrap();
        let back: ServerMsg = decode(&bytes).unwrap();
        assert_eq!(back, scanner);

        // RIFP carries pixels, a manifest summary, and a per-chunk map.
        let rifp = [
            ServerMsg::RifpRows { image_id: 2, y: 11, w: 4, h: 20, rows: vec![9, 8, 7, 6] },
            ServerMsg::RifpImage {
                image_id: 2,
                meta: RifpMeta {
                    session: "0123456789abcdef".into(),
                    filename: "oe1test.png".into(),
                    sender: Some("OE1TEST".into()),
                    hint: None,
                    media_type: "image/png".into(),
                    content_encoding: "identity".into(),
                    width: 320,
                    height: 240,
                    bits_per_pixel: 4,
                    encoded_size: 9_000,
                    chunk_count: 47,
                    chunks_first_pass: 45,
                },
                png: vec![0x89, 0x50, 0x4e, 0x47],
            },
            ServerMsg::RifpStatus(RifpStatus {
                tx_active: true,
                tx_progress: 0.25,
                sessions: vec![sdroxide_types::RifpSession {
                    session: "0123456789abcdef".into(),
                    sender: None,
                    have_manifest: true,
                    have: 3,
                    total: 47,
                    map: vec![0b0000_0111],
                    idle_s: 2,
                }],
                ..RifpStatus::default()
            }),
        ];
        for m in &rifp {
            let bytes = encode(m).unwrap();
            let back: ServerMsg = decode(&bytes).unwrap();
            assert_eq!(&back, m);
        }

        // The picture stores: metadata one way, thumbnails and whole pictures
        // the other. Every one of these carries a binary payload, which is
        // exactly what a length-prefixed non-self-describing encoding gets
        // wrong when a field is added in the wrong place.
        let pictures = [
            ServerMsg::ImagePresets(ImagePresets {
                slots: vec![
                    ImageSlotInfo {
                        message: "CQ SSTV de OE1TEST".into(),
                        width: 1024,
                        height: 768,
                        version: 0xdead_beef,
                        thumb: vec![0x89, 0x50, 0x4e, 0x47, 0x0d],
                    },
                    ImageSlotInfo::default(),
                ],
            }),
            ServerMsg::ImageSlotSource {
                slot: 3,
                version: 0x1234_5678,
                png: vec![0x89, 0x50, 0x4e, 0x47, 1, 2, 3],
            },
            ServerMsg::ImageListing(ImageListing {
                kind: ImageKind::Wefax,
                offset: 48,
                total: 312,
                entries: vec![ImageEntry {
                    kind: ImageKind::Wefax,
                    name: "wefax-20260729-141530Z-7878.1kHz-DWD.png".into(),
                    unix: 1_785_075_330,
                    width: 1809,
                    height: 1200,
                    bytes: 1_234_567,
                    thumb: vec![0x89, 0x50, 0x4e, 0x47],
                    rifp: None,
                }],
                dir: "/home/op/Pictures/sdroxide/wefax".into(),
            }),
            ServerMsg::ImageFile {
                kind: ImageKind::Sstv,
                name: "sstv-1753795200000.png".into(),
                png: vec![0x89, 0x50, 0x4e, 0x47, 9, 9],
            },
            ServerMsg::ImageSaved(ImageEntry {
                kind: ImageKind::Sstv,
                name: "sstv-1753795200000.png".into(),
                unix: 1_753_795_200,
                width: 320,
                height: 256,
                bytes: 40_000,
                thumb: vec![0x89, 0x50],
                rifp: None,
            }),
            ServerMsg::ImageDeleted {
                kind: ImageKind::Wefax,
                name: "wefax-20260729-141530Z-7878.1kHz-DWD.png".into(),
            },
        ];
        for m in &pictures {
            let bytes = encode(m).unwrap();
            let back: ServerMsg = decode(&bytes).unwrap();
            assert_eq!(&back, m);
        }

        let cmds = [
            ClientMsg::Command(Command::ImageSetSlot { slot: 2, bytes: vec![0xff, 0xd8, 0xff] }),
            ClientMsg::Command(Command::ImageSetMessage { slot: 2, message: "73".into() }),
            ClientMsg::Command(Command::ImageGetSlot(4)),
            ClientMsg::Command(Command::ImageClearSlot(0)),
            ClientMsg::Command(Command::ImageList { kind: ImageKind::Wefax, offset: 0, count: 48 }),
            ClientMsg::Command(Command::ImageGet {
                kind: ImageKind::Sstv,
                name: "sstv-1753795200000.png".into(),
            }),
            ClientMsg::Command(Command::ImageDelete {
                kind: ImageKind::Sstv,
                name: "sstv-1753795200000.png".into(),
            }),
            ClientMsg::Command(Command::WinlinkConnect),
            ClientMsg::Command(Command::MailList {
                folder: sdroxide_types::MailFolder::Inbox,
                offset: 0,
                count: 50,
            }),
            ClientMsg::Command(Command::MailGet {
                folder: sdroxide_types::MailFolder::Sent,
                mid: "TJKYEIMMHSRB".into(),
            }),
            ClientMsg::Command(Command::MailCompose(Box::new(sdroxide_types::MailDraft {
                to: vec!["OE1XYZ".into()],
                cc: vec![],
                subject: "hello".into(),
                body: "body".into(),
                // Attachments are raw bytes, which is exactly what a
                // non-self-describing encoding is easiest to get wrong on.
                attachments: vec![sdroxide_types::MailAttachment {
                    name: "a.bin".into(),
                    data: vec![0, 1, 254, 255],
                }],
            }))),
            ClientMsg::Command(Command::MailDelete {
                folder: sdroxide_types::MailFolder::Inbox,
                mid: "TJKYEIMMHSRB".into(),
            }),
            ClientMsg::Command(Command::MailMove {
                from: sdroxide_types::MailFolder::Inbox,
                to: sdroxide_types::MailFolder::Archive,
                mid: "TJKYEIMMHSRB".into(),
            }),
        ];
        for m in &cmds {
            let bytes = encode(m).unwrap();
            let back: ClientMsg = decode(&bytes).unwrap();
            assert_eq!(&back, m);
        }

        // Winlink: a listing, a whole message with a binary attachment, and the
        // session status. The attachment is the part worth round-tripping —
        // raw bytes through a non-self-describing encoding.
        let winlink = [
            ServerMsg::WinlinkStatus(sdroxide_types::WinlinkStatus {
                busy: false,
                activity: String::new(),
                last_error: Some("peer disconnected mid-session".into()),
                last_session: Some(1_786_699_845),
                last_received: 2,
                last_sent: 1,
                counts: [3, 0, 1, 0],
                log: vec!["> FF".into(), "< FQ".into()],
            }),
            ServerMsg::MailListing(sdroxide_types::MailListing {
                folder: sdroxide_types::MailFolder::Inbox,
                offset: 0,
                total: 3,
                entries: vec![sdroxide_types::MailEntry {
                    mid: "TJKYEIMMHSRB".into(),
                    date: "2026/08/14 09:30".into(),
                    from: "OE1XYZ".into(),
                    to: "OE3JJS".into(),
                    subject: "hello".into(),
                    folder: sdroxide_types::MailFolder::Inbox,
                    bytes: 512,
                    attachments: 1,
                    unread: true,
                }],
            }),
            ServerMsg::MailMessage(Box::new(sdroxide_types::MailMessage {
                mid: "TJKYEIMMHSRB".into(),
                date: "2026/08/14 09:30".into(),
                msg_type: "Private".into(),
                from: "OE1XYZ".into(),
                to: vec!["OE3JJS".into()],
                cc: vec![],
                subject: "hello".into(),
                body: "body".into(),
                attachments: vec![sdroxide_types::MailAttachment {
                    name: "a.bin".into(),
                    data: vec![0, 1, 254, 255],
                }],
                folder: sdroxide_types::MailFolder::Inbox,
            })),
            ServerMsg::MailSaved("TJKYEIMMHSRB".into()),
            ServerMsg::MailDeleted {
                folder: sdroxide_types::MailFolder::Inbox,
                mid: "TJKYEIMMHSRB".into(),
            },
        ];
        for m in &winlink {
            let bytes = encode(m).unwrap();
            let back: ServerMsg = decode(&bytes).unwrap();
            assert_eq!(&back, m);
        }
    }

    /// The station configuration, both ways.
    ///
    /// Worth its own test because `SatConfig` reaches types that were only ever
    /// written to JSON before: `OrbitRings` deserialises tolerantly from a
    /// config file (`untagged`, so `deserialize_any`), which postcard refuses
    /// outright. It has to take a second, non-self-describing form here, and a
    /// round trip is the only thing that says so.
    #[test]
    fn roundtrip_station_config() {
        use sdroxide_types::{
            CustomTle, OrbitRings, Passband, SatConfig, SatFreqs, SatLink, StationConfig,
            TleSubStatus, TleSubscription,
        };

        let sat = SatConfig {
            tles: vec![CustomTle {
                name: "NOAA 19".into(),
                line1: "1 33591U 09005A   26031.51268519  .00000271  00000-0  16472-3 0  9992"
                    .into(),
                line2: "2 33591  99.0361 121.3384 0013431 262.5195  97.4595 14.13096410877269"
                    .into(),
                enabled: true,
            }],
            subs: vec![TleSubscription {
                name: "Weather".into(),
                url: "https://celestrak.org/NORAD/elements/gp.php?GROUP=weather&FORMAT=tle".into(),
                enabled: true,
                orbits: OrbitRings::All,
                only: vec![33_591],
            }],
            freqs: vec![SatFreqs::new(
                43_017,
                "NOAA 19",
                vec![SatLink::down("APT", "FM", Passband::at(137.1))],
            )],
            seeded: true,
        };
        let msgs = [
            ServerMsg::StationConfig(Box::new(StationConfig { sat: sat.clone(), ..no_station() })),
            ServerMsg::TleSubStatus(vec![TleSubStatus {
                url: "https://celestrak.org/NORAD/elements/gp.php?GROUP=weather&FORMAT=tle".into(),
                fetched_unix: 1_785_075_330,
                count: 8,
                curated: 0,
                error: Some("connection reset".into()),
            }]),
        ];
        for m in &msgs {
            let bytes = encode(m).unwrap();
            let back: ServerMsg = decode(&bytes).unwrap();
            assert_eq!(&back, m);
        }

        let cmd = ClientMsg::Command(Command::SetSatConfig(sat));
        let back: ClientMsg = decode(&encode(&cmd).unwrap()).unwrap();
        assert_eq!(back, cmd);
        let cmd = ClientMsg::Command(Command::RefreshTleSubs);
        let back: ClientMsg = decode(&encode(&cmd).unwrap()).unwrap();
        assert_eq!(back, cmd);
    }

    /// Every orbit-ring position survives the wire, including the one a bare
    /// index would land on by accident if the mapping ever slipped.
    #[test]
    fn orbit_rings_survive_the_wire() {
        use sdroxide_types::{OrbitRings, SatConfig, StationConfig, TleSubscription};

        for orbits in OrbitRings::ALL {
            let sat = SatConfig {
                subs: vec![TleSubscription {
                    name: "g".into(),
                    url: "https://example.invalid/tle.txt".into(),
                    enabled: true,
                    orbits,
                    only: Vec::new(),
                }],
                ..SatConfig::default()
            };
            let m = ServerMsg::StationConfig(Box::new(StationConfig { sat, ..no_station() }));
            let back: ServerMsg = decode(&encode(&m).unwrap()).unwrap();
            assert_eq!(back, m, "orbit rings {orbits:?} did not survive");
        }
    }

    fn no_station() -> sdroxide_types::StationConfig {
        sdroxide_types::StationConfig::default()
    }

    /// The interface configuration, both ways.
    ///
    /// Worth its own test for `roundtrip_station_config`'s reason: every field
    /// here has only ever been written to JSON before, and JSON forgives things
    /// postcard does not. It is also the message where a silent mismatch would
    /// be worst — a config that decodes into the wrong fields does not fail,
    /// it reconfigures somebody's radio.
    ///
    /// Filled in rather than defaulted, and across several backends at once: a
    /// field-order slip only shows where two neighbouring fields hold different
    /// values, and `Default` is mostly zeros and empty strings.
    #[test]
    fn roundtrip_radio_config() {
        use sdroxide_types::{
            Backend, LimeConfig, LimeRfeConfig, RadioConfig, RfeChannel, RfeLink, RfeModeControl,
            RfePort, RtlSdrAgc, RtlSdrConfig, RtlSdrHfMode, RtlTcpConfig,
        };

        let cfg = RadioConfig {
            backend: Backend::RtlSdr,
            // The other enum with a payload on one variant only, near the end
            // of the struct — so a slip anywhere above it lands here.
            rx_site: sdroxide_types::RxSite::Elsewhere("DO30db".into()),
            // A `Vec` in the middle of the struct, of a struct carrying an enum
            // — and the enum's own new variant, which is the pair that has to
            // survive together. Two bands, one asserting different words on
            // receive and transmit.
            hpsdr: sdroxide_types::HpsdrConfig {
                filter_board: sdroxide_types::HpsdrFilterBoard::Custom,
                oc_table: vec![
                    sdroxide_types::HpsdrOcRow {
                        band: sdroxide_types::Band::M40,
                        rx: 0x01,
                        tx: 0x41,
                    },
                    sdroxide_types::HpsdrOcRow {
                        band: sdroxide_types::Band::M2,
                        rx: 0x7F,
                        tx: 0x00,
                    },
                ],
                ..sdroxide_types::HpsdrConfig::default()
            },
            // The last field in the struct, and a `Vec` of a struct carrying an
            // enum: a length, then variant indices and floats. Two rows, on
            // bands far apart in `Band`'s declaration order, so a discriminant
            // read a byte out of step cannot land on the right one by luck.
            tx_drive_trim: vec![
                sdroxide_types::BandDriveTrim { band: sdroxide_types::Band::M160, db: -4.5 },
                sdroxide_types::BandDriveTrim { band: sdroxide_types::Band::M6, db: 2.0 },
            ],
            converter_offset_hz: 125_000_000.0,
            // The transmit converter is an enum with a payload on one variant
            // only — the shape a self-describing format forgives and postcard
            // does not, so it is carried here rather than left at its default.
            converter_tx: sdroxide_types::ConverterTx::Own(-2_256_000_000.0),
            freq_ranges_rx: vec![(0.0, 14_400_000.0), (24_000_000.0, 1_766_000_000.0)],
            rtlsdr: RtlSdrConfig {
                serial: Some("00000001".into()),
                sample_rate_hz: 1_536_000.0,
                ppm: -17,
                tuner_gain_db: 32.8,
                agc: RtlSdrAgc::Rtl,
                hf_mode: RtlSdrHfMode::DirectQ,
                bias_tee: true,
                iq_correction: false,
                transfers: 12,
                transfer_kib: 32,
            },
            rtltcp: RtlTcpConfig {
                address: "mast.local:1234".into(),
                tuner_gain_db: 49.6,
                agc: RtlSdrAgc::Both,
                ..RtlTcpConfig::default()
            },
            // Two neighbouring enums that are *not* at their defaults: the
            // scope span decides how wide the radio sweeps, and the receive
            // source decides whether sdroxide demodulates at all.
            icomnet: sdroxide_types::IcomNetConfig {
                address: "ic705.local".into(),
                username: "sdroxide".into(),
                password: "hunter2".into(),
                rx_source: sdroxide_types::IcomRxSource::If12k,
                scope_span: sdroxide_types::IcomScopeSpan::Khz500,
                ..sdroxide_types::IcomNetConfig::default()
            },
            // Both of an RSPduo's tuners, and the filter that combines them:
            // like the LimeSDR's second chain below, this block decides
            // whether a whole extra stream exists.
            sdrplay: sdroxide_types::SdrPlayConfig {
                serial: "1809014C9B".into(),
                sample_rate_hz: 1_000_000.0,
                duo_tuner: sdroxide_types::SdrPlayDuoTuner::Tuner2,
                duo: sdroxide_types::SdrPlayDuo {
                    enabled: true,
                    role: sdroxide_types::SdrPlayDuoRole::SecondRadio,
                    mode: sdroxide_types::DiversityMode::Combine,
                    lna_state: 6,
                    if_gr_db: 27,
                    taps: 24,
                    rate: 0.35,
                    frozen: true,
                },
                ..sdroxide_types::SdrPlayConfig::default()
            },
            // The last block in the struct, and the one most worth filling in:
            // every field here differs from its neighbours' defaults, because a
            // field-order slip only shows where two adjacent fields disagree.
            // The nested LimeRFE block is the part that decides whether an
            // amplifier is switched into the transmit path, so it gets
            // non-default ports, a non-default channel and a non-default mode.
            lime: LimeConfig {
                device: "LimeSDR-USB, serial=0009072C02873717".into(),
                channel: 1,
                sample_rate_hz: 15.36e6,
                oversample: 4,
                rx_gain_db: 52.0,
                tx_gain_db: 31.0,
                tx_enabled: true,
                antenna_rx: "LNAH".into(),
                antenna_tx: "BAND2".into(),
                lpf_rx_hz: 18.0e6,
                lpf_tx_hz: 20.0e6,
                calibrate: false,
                iq_correction: false,
                fifo_ksamples: 512,
                throughput_vs_latency: 0.25,
                rfe: LimeRfeConfig {
                    link: RfeLink::Board,
                    serial: sdroxide_types::SerialConfig {
                        path: "/dev/ttyUSB7".into(),
                        baud: 9600,
                        ..sdroxide_types::SerialConfig::default()
                    },
                    port_rx: RfePort::J5,
                    port_tx: RfePort::J3,
                    follow_band: false,
                    channel: RfeChannel::Ham1280,
                    mode: RfeModeControl::TxRx,
                    notch: true,
                    atten_steps: 5,
                    fan: true,
                },
                // The second receive chain, likewise nothing at its default:
                // this block decides whether a whole extra stream exists.
                aux: sdroxide_types::LimeAuxConfig {
                    role: sdroxide_types::LimeAuxRole::Diversity,
                    antenna: "LNAW".into(),
                    gain_db: 33.0,
                    mode: sdroxide_types::DiversityMode::Combine,
                    taps: 24,
                    rate: 0.35,
                    frozen: true,
                    ps_bins: 48,
                    ps_rate: 0.8,
                    ps_frozen: false,
                },
            },
            ..RadioConfig::default()
        };

        let m = ServerMsg::RadioConfig(Box::new(cfg.clone()));
        assert_eq!(decode::<ServerMsg>(&encode(&m).unwrap()).unwrap(), m);

        // And back the other way — this is the direction that writes the file.
        for reopen in [false, true] {
            let c =
                ClientMsg::Command(Command::SetRadioConfig { cfg: Box::new(cfg.clone()), reopen });
            assert_eq!(decode::<ClientMsg>(&encode(&c).unwrap()).unwrap(), c);
        }
    }

    /// The per-mode transmit-audio level, over the wire in both directions
    /// (issue #186).
    ///
    /// Filled rather than left at its default on purpose: `DigiConfig` rides
    /// whole inside `DigiStatus`, postcard numbers fields by position, and a map
    /// placed among the scalars is exactly the shape that decodes into garbage
    /// rather than failing outright if the two ends disagree. An empty map would
    /// encode as a length of zero and prove nothing.
    #[test]
    fn roundtrip_per_mode_tx_level() {
        use sdroxide_types::{DigiConfig, Mode};

        let mut cfg = DigiConfig { my_call: "OE1XYZ".into(), ..DigiConfig::default() };
        cfg.set_tx_level(Mode::Ft8, 0.25);
        cfg.set_tx_level(Mode::Rtty, 0.4);
        cfg.set_tx_level(Mode::Aprs, 0.6);

        // The command that writes one, and the configuration that carries them
        // all back.
        let m = ClientMsg::Command(Command::SetDigiTxLevel { mode: Mode::Ft8, level: 0.25 });
        assert_eq!(decode::<ClientMsg>(&encode(&m).unwrap()).unwrap(), m);

        let m = ClientMsg::Command(Command::SetDigiConfig(cfg.clone()));
        assert_eq!(decode::<ClientMsg>(&encode(&m).unwrap()).unwrap(), m);

        let status = DigiStatus::idle(cfg);
        let m = ServerMsg::Ft8Status(status);
        let back = decode::<ServerMsg>(&encode(&m).unwrap()).unwrap();
        assert_eq!(back, m);
        let ServerMsg::Ft8Status(s) = back else { panic!("not a status") };
        assert_eq!(s.config.tx_level_for(Mode::Ft8), 0.25);
        assert_eq!(s.config.tx_level_for(Mode::Rtty), 0.4);
        assert_eq!(s.config.tx_level_for(Mode::Aprs), 0.6);
        // And a mode with no entry still reaches the carrier default across the
        // wire, which is the property that makes the map need no migration.
        assert_eq!(s.config.tx_level_for(Mode::Psk), 1.0);
    }

    /// Whether CW leaves as audio is a capability, and the client needs it to
    /// decide whether the transmit-audio level reaches CW at all.
    #[test]
    fn roundtrip_cw_audio_keyed_capability() {
        let caps = DeviceCaps { cw_audio_keyed: true, ..DeviceCaps::default() };
        let m = ServerMsg::Capabilities(caps);
        let back = decode::<ServerMsg>(&encode(&m).unwrap()).unwrap();
        assert_eq!(back, m);
        let ServerMsg::Capabilities(c) = back else { panic!("not capabilities") };
        assert!(c.cw_audio_keyed);
    }

    /// Device questions and their answers, both ways.
    ///
    /// One of each shape rather than every variant: a payload-less request, one
    /// carrying an address, one carrying a whole config block, and answers
    /// holding a device list, a test outcome and a refusal. Those are the four
    /// encodings; a variant added to any of them rides one of these.
    #[test]
    fn roundtrip_device_probes() {
        use sdroxide_types::{
            DeviceProbe, IcomNetConfig, ProbeAnswer, ProbeTest, RtlSdrDevice, TestKind,
        };

        let asks = [
            DeviceProbe::RtlSdr,
            DeviceProbe::Test(ProbeTest::Tci("shack.local:40001".into())),
            DeviceProbe::Test(ProbeTest::IcomNet(Box::new(IcomNetConfig {
                address: "705.local".into(),
                username: "oe1test".into(),
                password: "pässwörd".into(),
                ..IcomNetConfig::default()
            }))),
        ];
        for a in asks {
            let m = ClientMsg::Probe(a);
            assert_eq!(decode::<ClientMsg>(&encode(&m).unwrap()).unwrap(), m);
        }

        let answers = [
            ProbeAnswer::RtlSdr(vec![RtlSdrDevice {
                serial: Some("00000001".into()),
                name: "Generic RTL2832U OEM".into(),
                vid: 0x0bda,
                pid: 0x2838,
            }]),
            ProbeAnswer::Test(TestKind::Pluto, Ok("AD9364, 70 MHz – 6 GHz".into())),
            ProbeAnswer::Test(TestKind::Tci, Err("connection refused".into())),
            ProbeAnswer::Unsupported,
        ];
        for a in answers {
            let m = ServerMsg::ProbeAnswer(Box::new(a));
            assert_eq!(decode::<ServerMsg>(&encode(&m).unwrap()).unwrap(), m);
        }
    }

    /// The sign-in exchange, both ways.
    ///
    /// These three are the only messages that cross before the handshake has
    /// finished, so a client and server that disagree about their encoding
    /// cannot recover — there is no established session to report the fault on.
    #[test]
    fn roundtrip_sign_in() {
        let ask = ServerMsg::AuthRequired;
        assert_eq!(decode::<ServerMsg>(&encode(&ask).unwrap()).unwrap(), ask);

        let no = ServerMsg::AuthRejected("username or password not accepted".into());
        assert_eq!(decode::<ServerMsg>(&encode(&no).unwrap()).unwrap(), no);

        // Non-ASCII in either field: passwords are whatever the operator typed.
        let answer =
            ClientMsg::Auth { username: "oe1test".into(), password: "pässwörd ✓".into() };
        assert_eq!(decode::<ClientMsg>(&encode(&answer).unwrap()).unwrap(), answer);
    }

    #[test]
    fn rejects_wrong_version_byte() {
        assert!(matches!(decode::<ClientMsg>(&[0x7f, 0, 0]), Err(ProtoError::Version(0x7f))));
        assert!(matches!(decode::<ClientMsg>(&[]), Err(ProtoError::Empty)));
    }
}
