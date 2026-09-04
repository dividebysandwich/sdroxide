use serde::{Deserialize, Serialize};

/// Demodulation / modulation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Mode {
    Lsb,
    Usb,
    Cw,
    Am,
    Sam,
    Nfm,
    Wfm,
    Digu,
    Digl,
    Dsb,
    Spec,
    /// FT8 digital mode — USB underneath, decoded/encoded by the digi engine.
    Ft8,
    /// FT4 digital mode — USB underneath, decoded/encoded by the digi engine.
    Ft4,
    /// PSK31 keyboard mode — USB underneath, streaming BPSK31 decode/encode.
    Psk,
    /// RTTY keyboard mode — USB underneath, streaming FSK/Baudot decode/encode.
    Rtty,
    /// SSTV image mode — a sideband underneath, image decode/encode by the digi
    /// engine. The one mode here whose sideband depends on the band: analog
    /// SSTV is a phone emission and follows phone practice, so it is LSB on
    /// 160/80/40 m and USB above (see [`Mode::is_lower_sideband_at`]).
    Sstv,
    /// Olivia MFSK keyboard mode — USB underneath, tones/bandwidth chosen in setup.
    Olivia,
    /// THOR (DominoEX-family MFSK+FEC) keyboard mode — submode chosen in setup.
    Thor,
    /// FSQ (Fast Simple QSO) IFK keyboard mode — undirected/directed/image.
    Fsq,
    /// RF Paint (Spectrum Painting) — USB underneath; paints text/images
    /// directly onto the receiver's waterfall. Transmit-only (no decode).
    RfPaint,
    /// FreeDV RADE V1 (Radio Autoencoder) digital voice — a neural codec over
    /// an OFDM waveform occupying ~1000–1900 Hz of audio. A sideband
    /// underneath, and which one follows the band the way phone does: see
    /// [`Mode::sideband_follows_band`].
    Rade,
    /// Hellschreiber — USB underneath, a facsimile mode that paints a 7×14 dot
    /// matrix per character straight onto the channel. No sync, no framing, no
    /// decoder: the receiver free-runs and the operator's eye reads the raster.
    ///
    /// Appended last on purpose. `Mode` is postcard-encoded by declaration
    /// index and serde-serialised into stored configs, so a new variant may only
    /// go at the end. Where it *appears* is set by [`Mode::ALL`] instead.
    Hell,
    /// RIFP (Radio Image Framing Protocol, draft-dulaunoy-rifp-00) — a
    /// packetised image mode. Unlike every other digital mode here it is not
    /// USB underneath: the `rifp-cpfsk-4800` profile is continuous-phase FSK
    /// straight on the carrier, ±4 kHz at 4800 baud, so the dial *is* the
    /// signal's centre and the channel is ~25 kHz wide. Appended for the same
    /// reason as [`Mode::Hell`].
    Rifp,
    /// HF weather facsimile (WEFAX / radiofax) — USB underneath, an FM
    /// subcarrier carrying a continuous raster. Receive only: the charts are
    /// broadcast by meteorological services, and an amateur station has nothing
    /// to send back. Appended for the same reason as [`Mode::Hell`].
    Wefax,
    /// JS8 — the keyboard/messaging mode built on FT8's 8-FSK waveform. Slotted
    /// like FT8 but conversational rather than a contest exchange: free text,
    /// directed commands and heartbeats, at one of four speeds chosen in setup.
    /// Appended for the same reason as [`Mode::Hell`].
    Js8,
    /// WSPR — the Weak Signal Propagation Reporter beacon mode. 4-FSK at
    /// 1.46 baud, 162 symbols filling 110.6 s of a two-minute slot, six hertz
    /// wide, carrying nothing but a callsign, a grid and a power level.
    ///
    /// Slotted like FT8 but not a QSO mode at all: there is no addressing, no
    /// exchange and nobody to answer. What comes out of it is a measurement of
    /// a path, which is why its decodes are [`crate::WsprSpot`]s rather than
    /// [`crate::Decode`]s. Appended for the same reason as [`Mode::Hell`].
    Wspr,
    /// FT2 — FT4 with the symbol rate doubled: 4-GFSK at 41.667 baud, 167 Hz
    /// wide, a 2.52 s burst in a 3.75 s slot. The LDPC(174,91) code, the
    /// CRC-14 and the 77-bit payload are FT8's and FT4's, so the sequencer,
    /// the decode list and the logbook see nothing new — only the clock runs
    /// four times faster than FT8's. Appended for the same reason as
    /// [`Mode::Hell`].
    Ft2,
    /// AX.25 packet on VHF/UHF — 1200 baud Bell 202 AFSK or 9600 baud G3RUH,
    /// chosen by `DigiConfig::packet_baud`. Like [`Mode::Rifp`] and unlike
    /// every other digital mode, this is not sideband audio: both waveforms
    /// frequency-modulate the carrier, so the dial is the channel centre.
    ///
    /// The two bauds share one variant on purpose. They differ only in what
    /// the modem puts into the modulator — tones at 1200, shaped scrambled
    /// baseband at 9600 — and agree on everything a `Mode` decides: the filter,
    /// the rig mode to command, the band plan, carrier-centring. That is what
    /// a config field is for. Appended for the same reason as [`Mode::Hell`].
    Packet,
    /// AX.25 packet on HF — 300 baud AFSK, 200 Hz shift, on single sideband.
    /// The link layer is identical to [`Mode::Packet`]'s and the same
    /// controller drives both; this is a separate variant because the *radio*
    /// underneath is different (USB, not FM), and every per-mode table in the
    /// tree — filter, CAT mode, hamlib name, band plan — needs a different
    /// answer for it. Appended for the same reason as [`Mode::Hell`].
    PacketHf,
    /// Digital Radio Mondiale — the digital shortwave broadcast system. A few
    /// hundred OFDM carriers filling 9 or 10 kHz, carrying AAC audio plus a
    /// station label, a scrolling text message and the broadcaster's clock.
    ///
    /// A broadcast mode, not an amateur one: like [`Mode::Wfm`] it is something
    /// to listen to, so it is a *demodulator* rather than one of the digital
    /// modes above — no transmit, no QSO, no transcript. Like [`Mode::Rifp`]
    /// its carrier sits on the dial rather than on a sideband, because the 9
    /// and 10 kHz occupancies are symmetric about the channel's reference
    /// frequency. Appended for the same reason as [`Mode::Hell`].
    Drm,
    /// APRS — the Automatic Packet Reporting System: 1200 baud Bell 202 AX.25
    /// UI frames on one shared FM channel per region, carrying positions,
    /// weather, telemetry, objects and short messages.
    ///
    /// The waveform, the framing and the modem are [`Mode::Packet`]'s, and a
    /// config field could in principle have covered it. It is a `Mode` of its
    /// own because everything *around* the link layer differs: APRS is a
    /// single agreed channel rather than a band segment ([`crate::Region`]
    /// picks which), the payload has a protocol above AX.25 that produces a
    /// map and a message pane rather than a monitor, and the panel an operator
    /// wants is not the packet one. Appended for the same reason as
    /// [`Mode::Hell`].
    Aprs,
    /// SSTV on an FM carrier — the way slow-scan is sent on VHF and UHF.
    ///
    /// The same seven transmission modes as [`Mode::Sstv`], the same decoder,
    /// the same panel and the same gallery: what differs is the radio
    /// underneath. Below 30 MHz an SSTV picture is a phone emission on a
    /// sideband; on 2 m and 70 cm it modulates an FM carrier on a channel, so
    /// the dial is the channel centre rather than the foot of a passband and
    /// the transmitter has to be in FM to send it at all.
    ///
    /// A `Mode` of its own for exactly the reason [`Mode::Packet`] and
    /// [`Mode::PacketHf`] are two: every per-mode table in the tree — the
    /// filter, the demodulator, the CAT mode to command, the transmit level —
    /// needs a different answer, and none of them has a dial frequency to work
    /// it out from. Making [`Mode::Sstv`] answer by band would also take the
    /// choice away from the operator, and 2 m SSTV on sideband is a thing
    /// people do (issue #192). Appended for the same reason as [`Mode::Hell`].
    SstvFm,
    /// ADS-B / Mode S on 1090 MHz — the surveillance downlink every civil
    /// aircraft transmits, carrying an ICAO address, a callsign, an altitude, a
    /// velocity and a position.
    ///
    /// Receive only, and the widest mode here by three orders of magnitude:
    /// this is 1 Mbit/s pulse-position modulation and needs at least two
    /// megasamples a second of I/Q, so it is decoded off the raw stream by its
    /// own engine lane rather than by anything downstream of the receive
    /// chain's downconverter. There is no audio at all — like [`Mode::Spec`],
    /// its demodulator is `None`.
    ///
    /// A `Mode` rather than a window because it is a thing to point the radio
    /// *at*: it owns the dial, it owns the sample rate, and nothing else can be
    /// listened to while it runs. Appended for the same reason as
    /// [`Mode::Hell`].
    Adsb,
    /// RTTY on an FM carrier — Baudot AFSK into an FM transmitter, the way a
    /// club bulletin is still sent on 2 m.
    ///
    /// The same modem, the same tone pair and the same panel as [`Mode::Rtty`];
    /// what differs is the radio underneath. On HF an RTTY signal is a pair of
    /// audio tones on a sideband, so the dial is the foot of the passband and
    /// the on-air frequency is the dial plus the tone offset. On a VHF channel
    /// the tones modulate an FM carrier, so the dial *is* the channel, nothing
    /// is offset from it, and the transmitter has to be in FM to send it at all.
    ///
    /// A `Mode` of its own for exactly the reason [`Mode::SstvFm`] is one:
    /// every per-mode table in the tree — the filter, the demodulator, the CAT
    /// mode to command, the transmit level, whether the tone offset moves the
    /// logged frequency — needs a different answer, and none of them has a dial
    /// to work it out from. Answering by band would also take the choice away
    /// from the operator, and RTTY on 2 m sideband is a thing people do.
    /// Appended for the same reason as [`Mode::Hell`] (issue #214).
    RttyFm,
    /// NAVTEX — the maritime safety broadcast on 518, 490 and 4209.5 kHz.
    ///
    /// SITOR-B (ITU-R M.625 collective B-mode): 100 baud FSK, 170 Hz shift, a
    /// seven-bit constant-ratio alphabet and time diversity instead of a
    /// checksum. Receive only, and not because sdroxide could not key it — the
    /// service is a coast station's, and an amateur transmitting on it would be
    /// putting false safety information on a distress-adjacent frequency.
    ///
    /// The channel frequencies are the *assigned* frequency, which is the
    /// centre of the two tones, so the dial sits 1700 Hz below it in USB and
    /// [`Mode::standard_tone_offset_hz`] is what does that arithmetic — the
    /// same bargain RTTY strikes with its tone pair (issue #212).
    Navtex,
    /// VDL Mode 2 — the VHF datalink airliners and ground stations exchange
    /// ACARS over, on fourteen 25 kHz channels between 136.650 and 136.975 MHz.
    ///
    /// D8PSK at 10 500 symbols a second, Reed–Solomon coded, carrying AVLC
    /// frames: company messages, position reports, link handoffs, ATC datalink.
    /// [`Mode::Adsb`] is what an aircraft *is*; this is what it *says*.
    ///
    /// Receive only, and — like ADS-B — decoded off the raw I/Q by an engine
    /// lane of its own rather than by anything downstream of the receive
    /// chain's downconverter, because the channels are a quarter of a megahertz
    /// apart and all seven are listened to at once. There is no audio, so its
    /// demodulator is `None`.
    ///
    /// A `Mode` rather than a window for the reason ADS-B is one: it is a thing
    /// to point the radio *at*. It owns the dial and nothing else can be
    /// listened to while it runs. Appended for the same reason as
    /// [`Mode::Hell`].
    Vdl2,
    /// Independent sideband — two different signals on one carrier, the lower
    /// sideband carrying one and the upper another.
    ///
    /// Still on the air: broadcasters send two language services on one
    /// transmitter, and utility stations pair voice on one sideband with a
    /// teleprinter on the other. Demodulating it as USB or LSB gets one of the
    /// two and calls the other interference; there is no single audio signal to
    /// produce, which is why this is a mode rather than a filter setting.
    ///
    /// Both sidebands are demodulated and handed to the *ears*: lower on the
    /// left, upper on the right, the way they sit on the waterfall. That rides
    /// the same mid/side path WFM stereo uses ([`crate::Mode::stereo_audio`]),
    /// so nothing downstream needs to know there are two of them.
    ///
    /// Receive only. Independent-sideband transmit is a linear-amplifier
    /// arrangement with two modulators, and no radio sdroxide drives has one.
    /// Appended for the same reason as [`Mode::Hell`] (issue #280).
    Isb,
    /// AIS — the Automatic Identification System every ship of any size
    /// transmits: its identity, its position, its course and its destination,
    /// on two 25 kHz channels either side of 162.000 MHz.
    ///
    /// GMSK at 9600 bit/s in a self-organising TDMA frame, carrying HDLC
    /// frames: position reports every two to ten seconds, the ship's name and
    /// dimensions every six minutes, buoys and base stations alongside them.
    /// [`Mode::Adsb`] is the same idea in the air.
    ///
    /// Receive only — an amateur transmitting on AIS would be putting false
    /// vessel traffic on a safety-of-life service — and, like ADS-B and VDL2,
    /// decoded off the raw I/Q by an engine lane of its own rather than by
    /// anything downstream of the receive chain's downconverter, because both
    /// channels are listened to at once. There is no audio, so its demodulator
    /// is `None`.
    ///
    /// A `Mode` rather than a window for the reason ADS-B is one: it is a thing
    /// to point the radio *at*. It owns the dial and nothing else can be
    /// listened to while it runs. Appended for the same reason as
    /// [`Mode::Hell`].
    Ais,
}

/// The bands on which a mode that keeps phone practice rides the lower
/// sideband, as (low, high) Hz — see [`Mode::sideband_follows_band`].
///
/// Written as frequency ranges rather than [`crate::Band`] values on purpose:
/// the edges differ by region (80 m runs to 4.0 MHz in Region 2, 40 m to 7.3),
/// and the widest edges are the right answer here — a station tuned to 3.845
/// from Europe is still on 80 m as far as which sideband to use is concerned.
///
/// 160, 80 and 40 m and nothing else. 60 m is deliberately absent though it is
/// a low band: the 5 MHz channels are worked upper sideband the world over,
/// which is what the licence says in most of it. Everything from 30 m up is
/// USB by the same convention.
const PHONE_LSB_BANDS: [(f64, f64); 3] =
    [(1_800_000.0, 2_000_000.0), (3_500_000.0, 4_000_000.0), (7_000_000.0, 7_300_000.0)];

impl Mode {
    /// Every mode, in the order they cycle and appear in the picker — which is
    /// deliberately *not* the enum's declaration order (see [`Mode::Hell`]).
    pub const ALL: [Mode; 38] = [
        Mode::Lsb,
        Mode::Usb,
        Mode::Cw,
        Mode::Am,
        Mode::Sam,
        Mode::Nfm,
        Mode::Wfm,
        Mode::Drm,
        Mode::Adsb,
        Mode::Vdl2,
        Mode::Ais,
        Mode::Digu,
        Mode::Digl,
        Mode::Dsb,
        Mode::Isb,
        Mode::Spec,
        Mode::Ft8,
        Mode::Ft4,
        Mode::Ft2,
        Mode::Js8,
        Mode::Wspr,
        Mode::Psk,
        Mode::Rtty,
        Mode::RttyFm,
        Mode::Packet,
        Mode::PacketHf,
        Mode::Aprs,
        Mode::Sstv,
        Mode::SstvFm,
        Mode::Rifp,
        Mode::Wefax,
        Mode::Navtex,
        Mode::Olivia,
        Mode::Thor,
        Mode::Fsq,
        Mode::Hell,
        Mode::RfPaint,
        Mode::Rade,
    ];

    /// The digital modes handled by a dedicated decode/encode engine (the
    /// slotted FT8/FT4 modes, the continuous keyboard modes, Hell, SSTV, RIFP,
    /// packet, RF Paint). All are USB underneath except RIFP, VHF packet and
    /// VHF SSTV, which frequency-modulate the carrier.
    pub const DIGITAL: [Mode; 22] = [
        Mode::Ft8,
        Mode::Ft4,
        Mode::Ft2,
        Mode::Js8,
        Mode::Wspr,
        Mode::Psk,
        Mode::Rtty,
        Mode::RttyFm,
        Mode::Olivia,
        Mode::Thor,
        Mode::Fsq,
        Mode::Hell,
        Mode::Sstv,
        Mode::SstvFm,
        Mode::Rifp,
        Mode::Wefax,
        Mode::Navtex,
        Mode::RfPaint,
        Mode::Rade,
        Mode::Packet,
        Mode::PacketHf,
        Mode::Aprs,
    ];

    /// True for modes that use a dedicated decode/QSO layer over USB.
    pub fn is_digital(self) -> bool {
        matches!(
            self,
            Mode::Ft8
                | Mode::Ft4
                | Mode::Ft2
                | Mode::Js8
                | Mode::Wspr
                | Mode::Psk
                | Mode::Rtty
                | Mode::RttyFm
                | Mode::Sstv
                | Mode::SstvFm
                | Mode::Rifp
                | Mode::Olivia
                | Mode::Thor
                | Mode::Fsq
                | Mode::Hell
                | Mode::RfPaint
                | Mode::Rade
                | Mode::Wefax
                | Mode::Navtex
                | Mode::Packet
                | Mode::PacketHf
                | Mode::Aprs
        )
    }

    /// True for AX.25 packet, on either band. Both variants run the same link
    /// layer and the same controller; what differs is the radio underneath.
    ///
    /// Deliberately not true for [`Mode::Aprs`], which is AX.25 over the same
    /// modem but reaches its own controller and its own panel: every caller of
    /// this wants the connected-mode station — the KISS server, the Winlink
    /// route, the packet monitor — and APRS is none of those.
    pub fn is_packet(self) -> bool {
        matches!(self, Mode::Packet | Mode::PacketHf)
    }

    /// True for APRS.
    pub fn is_aprs(self) -> bool {
        matches!(self, Mode::Aprs)
    }

    /// True for ADS-B / Mode S on 1090 MHz.
    ///
    /// Deliberately not [`Mode::is_digital`], even though it has a decoder and
    /// a panel: every caller of that one means "the digi engine drives this",
    /// and the digi engine works in 48 kHz audio. ADS-B is decoded from the raw
    /// I/Q by an engine lane of its own, transmits nothing, and shares none of
    /// the digital modes' configuration.
    pub fn is_adsb(self) -> bool {
        matches!(self, Mode::Adsb)
    }

    /// True for VDL Mode 2 on the 136.7–137.0 MHz datalink channels.
    ///
    /// Not [`Mode::is_digital`], for the reason [`Mode::is_adsb`] is not: every
    /// caller of that one means "the digi engine drives this", and the digi
    /// engine works in 48 kHz audio. VDL2 is decoded from the raw I/Q by an
    /// engine lane of its own, transmits nothing, and shares none of the
    /// digital modes' configuration.
    pub fn is_vdl2(self) -> bool {
        matches!(self, Mode::Vdl2)
    }

    /// True for AIS on the two 162 MHz ship-reporting channels.
    ///
    /// Not [`Mode::is_digital`], for the reason [`Mode::is_adsb`] is not: every
    /// caller of that one means "the digi engine drives this", and the digi
    /// engine works in 48 kHz audio. AIS is decoded from the raw I/Q by an
    /// engine lane of its own, transmits nothing, and shares none of the
    /// digital modes' configuration.
    pub fn is_ais(self) -> bool {
        matches!(self, Mode::Ais)
    }

    /// True for the modes that own the bottom panel.
    ///
    /// [`Mode::is_digital`] used to answer this on its own, which was true
    /// until a mode arrived with a panel and no digi engine behind it. The two
    /// questions are separate: this one decides whether the panadapter shares
    /// the window, and that one decides who is being handed audio.
    pub fn has_bottom_panel(self) -> bool {
        self.is_digital() || self.is_adsb() || self.is_vdl2() || self.is_ais()
    }

    /// True for the modes decoded by a wideband engine lane off the raw I/Q
    /// rather than by the receive chain — ADS-B, VDL2 and AIS.
    ///
    /// What they have in common is everything the rest of the receiver assumes
    /// and they break: no audio, no transmitter, no receive filter, and a
    /// bandwidth set by the decoder rather than by the operator.
    pub fn is_wideband_lane(self) -> bool {
        self.is_adsb() || self.is_vdl2() || self.is_ais()
    }

    /// True for the modes whose transmit waveform is not single-sideband audio
    /// on the carrier, so the dial is the signal's centre rather than its lower
    /// edge: RIFP's CPFSK profile keys the carrier itself, and VHF packet
    /// frequency-modulates it. HF packet is *not* one of these — 300 baud is
    /// audio on a sideband like any other keyboard mode. APRS is VHF packet
    /// under another name, so it is.
    pub fn is_carrier_centered(self) -> bool {
        matches!(self, Mode::Rifp | Mode::Packet | Mode::Aprs | Mode::SstvFm | Mode::RttyFm)
    }

    /// True for the modes that go out on a *frequency-modulated* carrier.
    ///
    /// Asked when the answer changes what a level means rather than where the
    /// dial sits, which is why it is not [`Mode::is_carrier_centered`] even
    /// though the data modes in it are the same three: audio into an FM
    /// transmitter is deviation, and audio into a sideband one is drive into
    /// the modulator. The two want different numbers and neither is a sensible
    /// default for the other — see [`crate::DigiConfig::tx_audio_level_fm`],
    /// which this picks between.
    ///
    /// HF packet is deliberately not here: 300 baud is audio on a sideband like
    /// any other keyboard mode.
    pub fn is_fm_carrier(self) -> bool {
        matches!(
            self,
            Mode::Nfm
                | Mode::Wfm
                | Mode::Rifp
                | Mode::Packet
                | Mode::Aprs
                | Mode::SstvFm
                | Mode::RttyFm
        )
    }

    /// True for the modes whose transmit audio is levelled by
    /// [`crate::DigiConfig::tx_audio_levels`] — everything the digi engine
    /// transmits.
    ///
    /// CW is in it for the same reason it joins the digi engine at all (see
    /// `Engine::sync_digi_mode`): it is not a digital mode, but audio keying
    /// puts its sidetone through the same transmit-block seam, so the level
    /// reaches it. Whether it *applies* is a second question the mode cannot
    /// answer — a rig sending from its own keyer takes text over CAT and never
    /// hears our audio — and that one is `DeviceCaps::cw_audio_keyed`.
    ///
    /// Wefax is excluded because it never transmits: the charts are broadcast
    /// by meteorological services and an amateur station has nothing to send
    /// back.
    pub fn takes_digi_tx_audio(self) -> bool {
        (self.is_digital() && !self.is_rx_only()) || self == Mode::Cw
    }

    /// True for the continuous keyboard text modes (PSK31 / RTTY / Olivia / Thor
    /// / FSQ), as opposed to the slotted FT8/FT4 modes. Drives which decode
    /// engine + panel is used.
    pub fn is_text_modem(self) -> bool {
        matches!(
            self,
            Mode::Psk | Mode::Rtty | Mode::RttyFm | Mode::Olivia | Mode::Thor | Mode::Fsq
        )
    }

    /// True for the slotted FT8/FT4 modes, as opposed to the continuous
    /// keyboard modems and the image modes. Drives the decode-list / callsign
    /// overlays that only make sense for a slot-based decoder.
    ///
    /// WSPR is slotted too and is deliberately *not* here: those overlays are
    /// built from [`crate::Decode`]s, and WSPR produces [`crate::WsprSpot`]s.
    /// Including it would buy an overlay that is always empty and a transmit
    /// frequency picker for a mode whose tone offset does not move.
    pub fn is_slotted(self) -> bool {
        matches!(self, Mode::Ft8 | Mode::Ft4 | Mode::Ft2 | Mode::Js8)
    }

    /// How much spectrum this mode's signal occupies, in Hz, for the modes whose
    /// answer is a property of the waveform rather than of a filter setting.
    ///
    /// `None` means "ask something else": SSB and NFM are as wide as the filter
    /// the operator chose, and CW as wide as the fist sending it.
    ///
    /// Used by the transmit lockout, which needs to know how far above the
    /// carrier the emission actually reaches. The figures are tones times tone
    /// spacing: FT8 is 8 × 6.25, FT4 is 4 × 20.833, FT2 is 4 × 41.667 (see
    /// `Ft2::TONE_SPACING_HZ`), WSPR is 4 × 1.4648.
    ///
    /// JS8 is given its WIDEST speed, Turbo's 160 Hz, because a `Mode` does not
    /// know which speed is set — [`crate::Js8Speed::bandwidth_hz`] does, and is
    /// the figure to prefer wherever the speed is in hand. Erring wide is the
    /// right direction for a band-edge check and the wrong one for a display,
    /// so do not reuse this for drawing.
    pub fn occupied_bw_hz(self) -> Option<f32> {
        match self {
            Mode::Ft8 => Some(50.0),
            Mode::Ft4 => Some(83.3),
            Mode::Ft2 => Some(166.7),
            Mode::Js8 => Some(crate::Js8Speed::Turbo.bandwidth_hz()),
            Mode::Wspr => Some(6.0),
            _ => None,
        }
    }

    /// True for WSPR. Its own controller and panel: it is slotted like FT8, but
    /// there is no QSO to sequence and what it decodes is a list of paths rather
    /// than a conversation.
    pub fn is_wspr(self) -> bool {
        matches!(self, Mode::Wspr)
    }

    /// The clock this mode keeps, for the modes that keep one by themselves.
    ///
    /// `None` for JS8 — its slot length is an operator setting, so the answer
    /// depends on a [`crate::Js8Speed`] this enum does not carry; ask
    /// [`crate::Js8Speed::slot_timing`] instead. `None` too for every mode with
    /// no slots at all, which is what makes this the test for "is there a turn
    /// to show progress through".
    pub fn slot_timing(self) -> Option<SlotTiming> {
        match self {
            // Symbol 0 is nominally half a second into the slot, which is the
            // dt reference WSJT-X and mfsk-core both measure against.
            Mode::Ft8 => Some(SlotTiming { slot_s: 15.0, tx_offset_s: 0.5, burst_s: 12.64 }),
            Mode::Ft4 => Some(SlotTiming { slot_s: 7.5, tx_offset_s: 0.5, burst_s: 4.48 }),
            // FT2 is FT4 at double the symbol rate: 105 × 288 / 12000 = 2.52 s
            // of signal in a 3.75 s slot, keyed 0.1 s in (Decodium's
            // `Modulator.cpp` sets `delay_ms=100` for FT2 against FT4's 300).
            Mode::Ft2 => Some(SlotTiming { slot_s: 3.75, tx_offset_s: 0.1, burst_s: 2.52 }),
            // 162 symbols of 8192/12000 s, starting one second into a two-minute
            // slot. The burst fills all but nine seconds of it, which is why
            // there is almost no latitude in when to key.
            Mode::Wspr => Some(SlotTiming {
                slot_s: crate::WSPR_SLOT_S,
                tx_offset_s: crate::WSPR_TX_OFFSET_S,
                burst_s: crate::WSPR_BURST_S,
            }),
            _ => None,
        }
    }

    /// True for JS8. Forks the digi panel to the conversation UI and uses its
    /// own controller: it is slotted like FT8 but carries a chat rather than a
    /// contest exchange, so the Tx1–Tx6 sequencer has nothing to say about it.
    pub fn is_js8(self) -> bool {
        matches!(self, Mode::Js8)
    }

    /// True for the FSQ mode (adds a directed-message / contacts / image layer
    /// on top of the plain keyboard-modem panel).
    pub fn is_fsq(self) -> bool {
        matches!(self, Mode::Fsq)
    }

    /// True for the SSTV image mode. Forks the digi panel to the image UI and
    /// skips the FT8/text-modem overlays.
    pub fn is_sstv(self) -> bool {
        matches!(self, Mode::Sstv | Mode::SstvFm)
    }

    /// True for the RIFP image mode. Shares SSTV's image panel (compose,
    /// transmit, gallery) over a packetised protocol and its own modem.
    pub fn is_rifp(self) -> bool {
        matches!(self, Mode::Rifp)
    }

    /// True for the modes that drive the image panel — a picture compositor on
    /// transmit, a live picture and a gallery on receive.
    pub fn is_image(self) -> bool {
        matches!(self, Mode::Sstv | Mode::SstvFm | Mode::Rifp)
    }

    /// True for HF weather fax. Its own panel rather than the image one: there
    /// is nothing to compose and nothing to transmit, and what it needs instead
    /// — line rate, index of cooperation, phasing and slant — has no counterpart
    /// in SSTV.
    pub fn is_wefax(self) -> bool {
        matches!(self, Mode::Wefax)
    }

    /// True for the receive-only modes, so the UI can leave the transmit
    /// controls out rather than showing ones that refuse.
    pub fn is_rx_only(self) -> bool {
        // NAVTEX is receive-only by choice rather than by capability: the
        // service belongs to coast stations, and an amateur transmitting on it
        // would be putting false safety information on a distress-adjacent
        // channel.
        // ISB is receive-only by capability: transmitting it wants two
        // modulators driving one linear amplifier, and no radio sdroxide
        // drives is wired that way.
        matches!(self, Mode::Wefax | Mode::Adsb | Mode::Navtex | Mode::Vdl2 | Mode::Isb | Mode::Ais)
    }

    /// True for Hellschreiber. Forks the digi panel to the scrolling raster UI:
    /// unlike the keyboard modems there is nothing to decode into text, so it
    /// gets its own controller and panel rather than joining `is_text_modem`.
    pub fn is_hell(self) -> bool {
        matches!(self, Mode::Hell)
    }

    /// True for the RF Paint (Spectrum Painting) mode. Forks the digi panel to
    /// the text/image painting UI and uses its own transmit-only controller.
    pub fn is_rf_paint(self) -> bool {
        matches!(self, Mode::RfPaint)
    }

    /// True for FreeDV RADE V1 digital voice. Unlike the other digital modes it
    /// carries speech rather than text or images, so it both replaces the
    /// receive audio and consumes the microphone on transmit.
    pub fn is_rade(self) -> bool {
        matches!(self, Mode::Rade)
    }

    /// Whether the voice keyer may transmit in this mode.
    ///
    /// The digital modes synthesise their own transmit audio, so a recorded
    /// message has nowhere to go — RADE excepted: it carries speech, and takes
    /// the playback as its microphone input exactly like a live over.
    pub fn allows_voice_keyer(self) -> bool {
        !self.is_digital() || self.is_rade()
    }

    pub fn label(self) -> &'static str {
        match self {
            Mode::Lsb => "LSB",
            Mode::Usb => "USB",
            Mode::Cw => "CW",
            Mode::Am => "AM",
            Mode::Sam => "SAM",
            Mode::Nfm => "NFM",
            Mode::Wfm => "WFM",
            Mode::Digu => "DIGU",
            Mode::Digl => "DIGL",
            Mode::Dsb => "DSB",
            Mode::Spec => "SPEC",
            Mode::Ft8 => "FT8",
            Mode::Ft4 => "FT4",
            Mode::Ft2 => "FT2",
            Mode::Psk => "PSK",
            Mode::Rtty => "RTTY",
            Mode::RttyFm => "RTTY-FM",
            Mode::Navtex => "NAVTEX",
            Mode::Sstv => "SSTV",
            Mode::SstvFm => "SSTV-FM",
            Mode::Olivia => "OLIVIA",
            Mode::Thor => "THOR",
            Mode::Fsq => "FSQ",
            Mode::Hell => "HELL",
            Mode::RfPaint => "RFPAINT",
            Mode::Rade => "RADE",
            Mode::Rifp => "RIFP",
            Mode::Packet => "PACKET",
            Mode::PacketHf => "PACKET-HF",
            Mode::Aprs => "APRS",
            Mode::Wefax => "WEFAX",
            Mode::Js8 => "JS8",
            Mode::Wspr => "WSPR",
            Mode::Drm => "DRM",
            Mode::Adsb => "ADS-B",
            Mode::Vdl2 => "VDL2",
            Mode::Isb => "ISB",
            Mode::Ais => "AIS",
        }
    }

    /// Default audio passband edges in Hz relative to the carrier/VFO.
    /// Negative frequencies are below the carrier (LSB side).
    pub fn default_filter(self) -> (f32, f32) {
        match self {
            Mode::Lsb => (-2850.0, -150.0),
            Mode::Usb => (150.0, 2850.0),
            // CW passband is centered on the sidetone pitch (default 700 Hz).
            Mode::Cw => (450.0, 950.0),
            Mode::Am | Mode::Sam => (-5000.0, 5000.0),
            // DRM's carrier set is symmetric about the dial and 10 kHz
            // wide in the occupancy almost every broadcast uses. This
            // does not gate the decode — the transmission says how wide
            // it is and the decoder reads that — only what the
            // panadapter shades and what the S-meter measures.
            Mode::Drm => (-5000.0, 5000.0),
            Mode::Nfm => (-8000.0, 8000.0),
            Mode::Wfm => (-96_000.0, 96_000.0),
            // Not a receive filter — nothing narrows this stream, and the
            // decoder reads all of it. The +/-1 MHz is the demodulator's own
            // bandwidth, drawn on the panadapter so an operator can see that
            // the whole channel is being read rather than some slice of it.
            Mode::Adsb => (-1_000_000.0, 1_000_000.0),
            // Likewise not a receive filter: the decoder reads seven
            // channels spread over 325 kHz, and this is the whole plan
            // drawn on the panadapter so an operator can see that all of
            // it is being listened to.
            Mode::Vdl2 => (-162_500.0, 162_500.0),
            // Likewise: the two AIS channels sit 25 kHz either side of the
            // dial, and this is the pair of slots drawn on the panadapter so
            // an operator can see that both are being listened to.
            Mode::Ais => (-37_500.0, 37_500.0),
            Mode::Digu => (200.0, 3200.0),
            Mode::Digl => (-3200.0, -200.0),
            Mode::Dsb => (-2850.0, 2850.0),
            // Both sidebands, drawn as one passband: the edges are the outer
            // ones, and the demodulator keeps its own gap either side of the
            // carrier (see `IsbDemod`).
            Mode::Isb => (-2850.0, 2850.0),
            Mode::Spec => (-5000.0, 5000.0),
            // FT8/FT4 occupy the whole USB audio passband (tones 0..~3500 Hz).
            // PSK/RTTY/Olivia/Thor/FSQ/Hell do the same (the modem filters
            // narrowly around audio_hz — and Hell X9 needs nearly all of it).
            // SSTV occupies the full sideband audio passband (mirrored onto
            // the lower sideband by `default_filter_at` where it rides one).
            Mode::Ft8
            | Mode::Ft4
            | Mode::Ft2
            | Mode::Js8
            | Mode::Psk
            | Mode::Rtty
            | Mode::Sstv
            | Mode::Olivia
            | Mode::Thor
            | Mode::Fsq
            | Mode::Hell
            | Mode::RfPaint => (100.0, 3300.0),
            // The fax subcarrier is 1900 Hz ± 400; the wider passband leaves
            // room for a receiver tuned a few hundred hertz off, which is the
            // normal state of affairs on a chart found by ear.
            Mode::Wefax => (500.0, 3300.0),
            // The two NAVTEX tones are 1615 and 1785 Hz; a few hundred hertz
            // either side leaves room for a receiver that is not exactly on the
            // channel, which is the usual state of a signal found by ear.
            Mode::Navtex => (1300.0, 2100.0),
            // WSPR lives in one 200 Hz window, 1400–1600 Hz above the dial, and
            // the decoder searches nowhere else. Narrow rather than the usual
            // digital 100–3300 on purpose: the QRSS beacons just below the
            // window and anything else in a 3 kHz passband would work the AGC
            // against signals ten dB under the noise, which is the whole range
            // this mode operates in. 1200–1800 leaves room for a dial a few
            // hundred hertz out without letting the neighbours in.
            Mode::Wspr => (1200.0, 1800.0),
            // RIFP is not a sideband mode: the CPFSK carrier sits *on* the
            // dial and swings ±4 kHz, so the passband straddles it. 25 kHz is
            // the profile's recommended occupied bandwidth.
            Mode::Rifp => (-12_500.0, 12_500.0),
            // RADE V1's OFDM carriers sit between roughly 1060 and 1880 Hz;
            // the wider passband leaves room for the acquisition search to
            // track a signal that is off frequency.
            Mode::Rade => (300.0, 2700.0),
            // VHF packet frequency-modulates the carrier, so like RIFP the
            // passband straddles the dial. ±8 kHz is the NFM channel: 1200
            // Bell 202 at ±3 kHz deviation occupies about 10 kHz by Carson,
            // 9600 G3RUH about 16 kHz, and both fit inside a 25 kHz channel
            // with the usual margin for a rig a little off frequency.
            // APRS shares the channel and therefore the passband; it is
            // 1200 Bell 202 on FM whatever the region. VHF SSTV is the same
            // shape of thing — an FM channel, not a sideband — and its video
            // subcarrier runs to 2300 Hz, well inside it.
            Mode::Packet | Mode::Aprs | Mode::SstvFm | Mode::RttyFm => (-8_000.0, 8_000.0),
            // HF packet is 300 baud AFSK on a sideband, tones around
            // 1600/1800 Hz — an ordinary keyboard-mode passband.
            Mode::PacketHf => (150.0, 2850.0),
        }
    }

    /// True for modes that place the displayed carrier below the passband.
    ///
    /// Answers for the mode alone, so it cannot see a band-dependent sideband
    /// ([`Self::sideband_follows_band`]) — prefer [`Self::is_lower_sideband_at`]
    /// wherever a dial frequency is at hand.
    pub fn is_lower_sideband(self) -> bool {
        matches!(self, Mode::Lsb | Mode::Digl)
    }

    /// True for the modes whose sideband is a property of the *band* rather
    /// than of the mode, so it cannot be answered without a dial.
    ///
    /// Both are phone emissions and keep phone practice rather than the other
    /// digital modes' fixed USB: analog SSTV, and FreeDV RADE, which carries
    /// speech and is worked on the phone segments alongside it. On 160, 80 and
    /// 40 m both ride the lower sideband — a picture or an over sent on USB
    /// there arrives at everybody else's receiver inverted, and an inverted
    /// RADE signal does not decode at all — and USB on every band above.
    ///
    /// `Mode::Sstv` by name rather than [`Self::is_sstv`]: sideband is a
    /// question about a sideband emission, and [`Mode::SstvFm`] is not one.
    pub fn sideband_follows_band(self) -> bool {
        matches!(self, Mode::Sstv | Mode::Rade)
    }

    /// True for modes that place the displayed carrier below the passband at
    /// `dial_hz`.
    ///
    /// Sideband is a fixed property of every mode but the two
    /// [`Self::sideband_follows_band`] names, which take theirs from the band
    /// they are being worked on.
    pub fn is_lower_sideband_at(self, dial_hz: f64) -> bool {
        self.is_lower_sideband()
            || (self.sideband_follows_band()
                && PHONE_LSB_BANDS.iter().any(|&(lo, hi)| dial_hz >= lo && dial_hz <= hi))
    }

    /// [`Self::default_filter`] at a dial frequency: the same passband, mirrored
    /// onto the lower sideband where the mode rides one there (SSTV and RADE on
    /// 160/80/40 m). Sideband is carried entirely in the sign of the edges, so
    /// this is what actually puts the demodulator on the right side.
    pub fn default_filter_at(self, dial_hz: f64) -> (f32, f32) {
        let (lo, hi) = self.default_filter();
        if self.is_lower_sideband_at(dial_hz) && lo >= 0.0 { (-hi, -lo) } else { (lo, hi) }
    }

    /// True where the signal being worked sits at an audio offset from the dial
    /// rather than on it, so the dial alone is not the frequency of the contact.
    ///
    /// CW is the surprising one: the dial sits a sidetone-pitch below what is
    /// being copied — and keyed — so a 700 Hz pitch puts every contact 700 Hz
    /// above the number in the readout. The digital modes are the familiar
    /// case, transmitting at the dial plus their tone offset. The
    /// carrier-centred modes ([`Self::is_carrier_centered`]) are digital and
    /// deliberately not here: they key the carrier itself, so the dial *is* the
    /// frequency.
    pub fn tunes_off_dial(self) -> bool {
        (self.is_digital() || matches!(self, Mode::Cw)) && !self.is_carrier_centered()
    }

    /// Where the signal actually is, given the dial and the mode's audio cursor
    /// (`DigiStatus::audio_hz`, the CW pitch or the digital tone offset).
    ///
    /// This is the frequency of the contact: what goes in the log, what is
    /// quoted on the air, and what another station reads off their own dial.
    /// The analog modes ignore `audio_hz` and answer with the dial, which for
    /// them is the same thing.
    pub fn on_air_hz(self, dial_hz: f64, audio_hz: f32) -> f64 {
        if !self.tunes_off_dial() {
            return dial_hz;
        }
        if self.is_lower_sideband_at(dial_hz) {
            dial_hz - f64::from(audio_hz)
        } else {
            dial_hz + f64::from(audio_hz)
        }
    }

    /// The audio offset this mode's tones stand at where the offset is a
    /// standard the whole band keeps rather than a slot the operator picks
    /// inside a sub-band.
    ///
    /// RTTY is the one that has one: mark and space are 2125 and 2295 Hz
    /// wherever you are, and stations are worked by moving the dial onto them,
    /// not by dragging the tone pair across the passband. The slotted modes are
    /// the opposite — one agreed dial per band, and the choice of where in the
    /// sub-band to transmit is worth remembering — which is what
    /// `DigiConfig::tx_audio_hz` is for, and why RTTY stays out of it.
    pub fn standard_tone_offset_hz(self) -> Option<f32> {
        match self {
            Mode::Rtty => Some(crate::RTTY_CENTER_HZ),
            // The channel frequencies (518, 490, 4209.5 kHz) are the assigned
            // frequency, which is the *centre* of the two tones — so the dial
            // is 1700 Hz below the channel, and a decode logged at the dial
            // would be logged 1.7 kHz low.
            Mode::Navtex => Some(crate::NAVTEX_TONE_HZ),
            _ => None,
        }
    }

    /// True where [`Self::standard_tone_offset_hz`] has an answer: the mode's
    /// tone offset is fixed by convention, so a click tunes the dial onto the
    /// signal rather than moving the tones onto it.
    pub fn holds_standard_tones(self) -> bool {
        self.standard_tone_offset_hz().is_some()
    }

    /// Which carrier position a transceiver puts this mode at, for the per-mode
    /// I.F. offsets of [`crate::PanadapterConfig`].
    ///
    /// Not the same folding as the engine's `rig_mode_class`, which exists to
    /// recognise a rig reporting the plain sideband a data mode rides on. Here
    /// the data modes have to stay apart from it: a rig's DATA setting commonly
    /// sits at a different carrier offset from plain SSB, and that difference
    /// is exactly what this table is for.
    pub fn if_class(self) -> crate::IfModeClass {
        use crate::IfModeClass as C;
        match self {
            Mode::Lsb => C::Lsb,
            Mode::Usb | Mode::Spec => C::Usb,
            Mode::Cw => C::Cw,
            // DRM sits on the dial like AM does, and a receiver with an
            // I.F. output offers no separate DRM setting to differ from.
            // ISB joins them for the same reason DSB does: the carrier is on
            // the dial and a rig with an I.F. output has no separate setting
            // for it.
            Mode::Am | Mode::Sam | Mode::Dsb | Mode::Drm | Mode::Isb => C::Am,
            // WFM is FM's carrier position too; a rig with an I.F. output has
            // no such mode, so nothing here is lost by grouping them.
            // ADS-B joins them for the same reason WFM does: no radio with an
            // I.F. output has this mode, so there is no separate offset for it
            // to have, and FM's is the one a wideband receiver already uses.
            Mode::Nfm | Mode::Wfm | Mode::Adsb | Mode::Vdl2 | Mode::Ais => C::Fm,
            // Everything a rig would be put into DATA (or DIGI) for, on either
            // sideband — including RIFP and VHF packet, which the rig carries
            // as FM data rather than SSB but still through its data input.
            Mode::Digu
            | Mode::Digl
            | Mode::Ft8
            | Mode::Ft4
            | Mode::Ft2
            | Mode::Js8
            | Mode::Wspr
            | Mode::Psk
            | Mode::Rtty
            | Mode::RttyFm
            | Mode::Sstv
            | Mode::SstvFm
            | Mode::Rifp
            | Mode::Wefax
            | Mode::Navtex
            | Mode::Olivia
            | Mode::Thor
            | Mode::Fsq
            | Mode::Hell
            | Mode::RfPaint
            | Mode::Rade
            | Mode::Packet
            | Mode::PacketHf
            | Mode::Aprs => C::Data,
        }
    }

    /// Whether the audio-chain AGC runs in this mode. An FM discriminator's
    /// output level is set by deviation, not signal strength, so there is
    /// nothing for an AGC to level — it can only pump on the noise between
    /// overs and breathe with the modulation. FM chains pass audio at unity
    /// gain instead, and the UI draws no AGC control for them.
    pub fn audio_agc(self) -> bool {
        // DRM joins FM in bypassing the AGC, for the same reason: what comes
        // out is not a demodulated signal whose level follows the carrier's,
        // but the audio codec's own output, already at the level the
        // broadcaster mixed it to. Levelling it again would ride the programme.
        // ADS-B is here because it produces no audio at all — its receive
        // chain has no demodulator, so there is nothing for an AGC to be in
        // front of.
        !matches!(self, Mode::Nfm | Mode::Wfm | Mode::Drm | Mode::Adsb | Mode::Vdl2 | Mode::Ais)
    }

    /// Whether this mode offers binaural (pseudo-stereo) audio — the receive
    /// passband spread across the stereo image, so that pitch becomes
    /// direction (issue #263).
    ///
    /// CW is the mode the pan law fits exactly: a CW signal *is* a tone, so
    /// placing it by pitch places the signal, and two stations a couple of
    /// hundred hertz apart in a pile-up become two sources rather than two
    /// notes.
    ///
    /// SSB is here because operators asked for it, and what it buys there is a
    /// different thing worth having: a voice occupies the whole passband rather
    /// than a point in it, so two stations do not separate the way two notes
    /// do, but the *noise* still decorrelates across the image while the voice
    /// stays coherent in the middle of it — which is the spaciousness that
    /// makes a long listen on a noisy band less tiring. The cost is that one
    /// speaker's own spectrum is spread across the head, low formants to one
    /// side and sibilance to the other, and not everybody likes it. It is
    /// opt-in and off by default, so that is the operator's call to make.
    ///
    /// Everything else is left out. The data modes have no listener to place
    /// anything for; AM and FM broadcast would get an effect rather than a
    /// receiving aid; WFM already has a second ear of its own. None of them is
    /// worth another permanent button on a row that has to fit a 1366-pixel
    /// screen.
    pub fn binaural_audio(self) -> bool {
        matches!(self, Mode::Cw | Mode::Lsb | Mode::Usb)
    }

    /// Furthest a filter edge may be dragged from the carrier — bounded by
    /// the mode's DSP channel bandwidth.
    pub fn max_filter_hz(self) -> f32 {
        match self {
            Mode::Wfm => 120_000.0,
            // Not a filter in the sense the others are — there is no channel
            // being carved out of anything, because the decoder reads the whole
            // stream. What the number does is let the panadapter shade the
            // 2 MHz the demodulator actually looks at, which on a receiver
            // whose span is wider than that is worth seeing.
            Mode::Adsb => 1_200_000.0,
            // Room to shade the whole seven-channel plan, and a little
            // past it — for the same reason ADS-B has one: the number
            // does not narrow anything, it only says what is being read.
            Mode::Vdl2 => 250_000.0,
            // Room to shade both AIS channels and a little past them, for the
            // same reason: the number does not narrow anything, it only says
            // what is being read.
            Mode::Ais => 60_000.0,
            _ => 24_000.0,
        }
    }

    /// Whether the passband is a channel *centred* on the carrier, so that
    /// dragging one edge has to carry the other with it.
    ///
    /// AM and its relatives detect both sidebands together and FM detects a
    /// channel about the carrier: in either the two halves of the passband
    /// carry the same signal, and narrowing one alone throws away half of it
    /// while letting the interference on the other side straight through.
    /// Every preset these modes have is symmetric for that reason, and a hand
    /// drag ought not to be able to reach a shape the presets deliberately
    /// cannot (issue #256).
    ///
    /// SSB, CW and the data modes are the other case and keep both edges to
    /// themselves: their passband sits to one side of the carrier by
    /// definition, and its two edges do different jobs.
    ///
    /// ADS-B and VDL2 are left out on purpose: their edges shade what the
    /// decoder is reading rather than filter anything, so there is no signal
    /// to lose by moving one.
    pub fn filter_symmetric(self) -> bool {
        matches!(
            self,
            Mode::Am
                | Mode::Sam
                | Mode::Dsb
                // Both sidebands, always the same width: dragging one edge
                // has to move the other, or one ear ends up wider than the
                // other with nothing on screen saying so.
                | Mode::Isb
                | Mode::Nfm
                | Mode::Wfm
                | Mode::SstvFm
                | Mode::RttyFm
                | Mode::Packet
                | Mode::Aprs
        )
    }

    /// Filter width presets: (label, lo, hi) relative to the carrier.
    pub fn filter_presets(self) -> &'static [(&'static str, f32, f32)] {
        match self {
            Mode::Usb | Mode::Digu => &[
                ("1.8k", 200.0, 2000.0),
                ("2.4k", 200.0, 2600.0),
                ("2.7k", 150.0, 2850.0),
                ("3.3k", 100.0, 3400.0),
            ],
            Mode::Lsb | Mode::Digl => &[
                ("1.8k", -2000.0, -200.0),
                ("2.4k", -2600.0, -200.0),
                ("2.7k", -2850.0, -150.0),
                ("3.3k", -3400.0, -100.0),
            ],
            Mode::Cw => &[
                ("100", 650.0, 750.0),
                ("250", 575.0, 825.0),
                ("500", 450.0, 950.0),
                ("1k", 200.0, 1200.0),
            ],
            Mode::Am | Mode::Sam => {
                &[("6k", -3000.0, 3000.0), ("10k", -5000.0, 5000.0), ("16k", -8000.0, 8000.0)]
            }
            // Both sidebands at once, so the label is the width of *each* one
            // — an ISB channel described as "2.7 kHz per sideband" is 5.4 kHz
            // of spectrum, and calling it 5.4k would read as half of what the
            // operator gets in either ear.
            Mode::Isb => &[
                ("1.8k", -2000.0, 2000.0),
                ("2.4k", -2600.0, 2600.0),
                ("2.7k", -2850.0, 2850.0),
                ("3.3k", -3400.0, 3400.0),
            ],
            // VHF SSTV joins NFM rather than packet's wider pair: it is a voice
            // channel with a picture on it, and the deviation is a voice
            // channel's.
            Mode::Nfm | Mode::SstvFm | Mode::RttyFm => {
                &[("8k", -4000.0, 4000.0), ("16k", -8000.0, 8000.0)]
            }
            Mode::Dsb => &[("5k", -2500.0, 2500.0), ("6k", -3000.0, 3000.0)],
            // Both wider than any filter would be: a Mode S reply reaches its
            // first nulls about 6 MHz out and is read by a slicer rather than
            // by a passband. These are here so the panadapter's shading can be
            // made to match the window the receiver is actually delivering,
            // which is the only thing that limits the decode.
            Mode::Adsb => &[("2M", -1_000_000.0, 1_000_000.0), ("2.4M", -1_200_000.0, 1_200_000.0)],
            // One channel, or the whole plan. A receiver too narrow for
            // the group can still take the Common Signalling Channel, and
            // this is how the shading says which of the two it is doing.
            Mode::Vdl2 => &[("25k", -12_500.0, 12_500.0), ("325k", -162_500.0, 162_500.0)],
            // One channel, or both. A receiver too narrow for the pair still
            // hears whichever it is over, and this is how the shading says
            // which of the two it is doing.
            Mode::Ais => &[("25k", -12_500.0, 12_500.0), ("75k", -37_500.0, 37_500.0)],
            // The one digital mode with a real filter choice: 1200 Bell 202
            // occupies about 10 kHz and 9600 G3RUH about 16 kHz, so the
            // operator wants the narrower one when running 1200 on a busy
            // channel. Labelled by occupied bandwidth, like NFM's.
            Mode::Packet | Mode::Aprs => &[("12k", -6000.0, 6000.0), ("20k", -10_000.0, 10_000.0)],
            // The six spectrum occupancies of the DRM standard, as the
            // carrier tables actually place them: the two half-channel
            // modes sit entirely above the reference, 9 and 10 kHz are
            // symmetric about it, and the two wide ones are neither.
            Mode::Drm => &[
                ("4.5k", 0.0, 4300.0),
                ("5k", 0.0, 4900.0),
                ("9k", -4300.0, 4300.0),
                ("10k", -4900.0, 4900.0),
                ("18k", -4100.0, 13_100.0),
                ("20k", -4700.0, 14_600.0),
            ],
            // Digital modes have a fixed wide passband; no presets.
            Mode::Wfm
            | Mode::Spec
            | Mode::Ft8
            | Mode::Ft4
            | Mode::Ft2
            | Mode::Js8
            | Mode::Wspr
            | Mode::Psk
            | Mode::Rtty
            | Mode::Sstv
            | Mode::Olivia
            | Mode::Thor
            | Mode::Fsq
            | Mode::Hell
            | Mode::RfPaint
            | Mode::Rifp
            | Mode::Wefax
            | Mode::Navtex
            | Mode::PacketHf
            | Mode::Rade => &[],
        }
    }
}

/// The clock a slotted mode keeps: how long one turn lasts, when the
/// transmitter keys inside it, and how long it stays keyed.
///
/// Slots are counted from the Unix epoch, so the slot containing a given
/// instant is `floor(unix / slot_s)` and needs no other reference. That is what
/// lets a receiver and a transmitter on opposite sides of the world agree on
/// when a turn starts with nothing but their clocks. Every slot length here is
/// commensurate with a minute — it either divides one, as FT8's 15 s does, or
/// is a whole number of them, as WSPR's two — so the boundaries also land where
/// an operator reading a clock expects them to.
///
/// Interface facts, not protocol internals: the engine schedules against these,
/// and the panels draw the turn's progress from them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SlotTiming {
    /// Slot period in seconds — how often a transmission may start.
    pub slot_s: f64,
    /// Delay from the slot boundary to the first symbol, in seconds.
    pub tx_offset_s: f64,
    /// On-air duration of one transmission, in seconds.
    pub burst_s: f64,
}

impl SlotTiming {
    /// How far into a slot the burst stops, as a fraction of the slot.
    ///
    /// What is left after it is the mode's turnaround: decode time at the far
    /// end, and the margin that keeps a clock a little out from overrunning the
    /// next slot.
    pub fn burst_end_frac(&self) -> f64 {
        ((self.tx_offset_s + self.burst_s) / self.slot_s).clamp(0.0, 1.0)
    }
}

impl std::str::FromStr for Mode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Mode::ALL
            .into_iter()
            .find(|m| m.label().eq_ignore_ascii_case(s))
            .ok_or_else(|| format!("unknown mode {s:?} (try USB, LSB, CW, AM, SAM, NFM, WFM…)"))
    }
}

/// Which denoiser is running behind the NR chip.
///
/// Derived from [`NrLevel`] rather than stored: the wire carries the level, so a
/// fifth engine would cost three appended `NrLevel` variants and nothing else.
/// This type is never serialised.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NrEngine {
    /// RNNoise (`nnnoiseless`) — a recurrent per-band gain estimator.
    Rnn,
    /// DeepFilterNet3 (`deep_filter` / tract) — ERB gains plus a deep filter.
    DeepFilter,
    /// The spectral-bleach algorithm, ported to Rust in `sdroxide-dsp`.
    SpecBleach,
    /// The hand-written MCRA + log-MMSE spectral NR this program started with.
    Spectral,
}

impl NrEngine {
    /// Engine-row order in the NR picker: neural first, classical last.
    pub const ALL: [NrEngine; 4] =
        [NrEngine::Rnn, NrEngine::DeepFilter, NrEngine::SpecBleach, NrEngine::Spectral];

    /// The tag the chips wear. The original spectral NR keeps the bare "NR" it
    /// has always had, so an operator who never opens the picker sees exactly
    /// the chip they saw before.
    pub fn tag(self) -> &'static str {
        match self {
            NrEngine::Rnn => "RNN",
            NrEngine::DeepFilter => "DFNR",
            NrEngine::SpecBleach => "SPEC",
            NrEngine::Spectral => "NR",
        }
    }

    /// What the hover text calls it.
    pub fn name(self) -> &'static str {
        match self {
            NrEngine::Rnn => "RNNoise — neural, speech-trained, cheap",
            NrEngine::DeepFilter => "DeepFilterNet3 — neural, strongest, costliest",
            NrEngine::SpecBleach => "Spectral bleach — adaptive spectral, masked",
            NrEngine::Spectral => "Spectral NR — MCRA + log-MMSE",
        }
    }

    /// This engine at `strength`.
    pub fn at(self, s: NrStrength) -> NrLevel {
        use NrEngine::*;
        use NrStrength::*;
        match (self, s) {
            (Spectral, Low) => NrLevel::Low,
            (Spectral, Med) => NrLevel::Medium,
            (Spectral, High) => NrLevel::High,
            (Rnn, Low) => NrLevel::RnnLow,
            (Rnn, Med) => NrLevel::RnnMed,
            (Rnn, High) => NrLevel::RnnHigh,
            (SpecBleach, Low) => NrLevel::SpecLow,
            (SpecBleach, Med) => NrLevel::SpecMed,
            (SpecBleach, High) => NrLevel::SpecHigh,
            (DeepFilter, Low) => NrLevel::DfLow,
            (DeepFilter, Med) => NrLevel::DfMed,
            (DeepFilter, High) => NrLevel::DfHigh,
        }
    }

    /// The next engine in picker order, wrapping.
    pub fn next(self) -> NrEngine {
        let i = NrEngine::ALL.iter().position(|e| *e == self).unwrap_or(0);
        NrEngine::ALL[(i + 1) % NrEngine::ALL.len()]
    }
}

/// How hard whichever engine is selected is pushed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NrStrength {
    Low,
    Med,
    High,
}

impl NrStrength {
    pub const ALL: [NrStrength; 3] = [NrStrength::Low, NrStrength::Med, NrStrength::High];

    pub fn label(self) -> &'static str {
        match self {
            NrStrength::Low => "Low",
            NrStrength::Med => "Med",
            NrStrength::High => "High",
        }
    }
}

/// Audio noise-reduction setting for the demodulated audio: one of four engines
/// at one of three intensities, or off. See [`NrEngine`].
///
/// **The declaration order is the wire format.** postcard encodes the
/// discriminant positionally, so variants are only ever appended — the spectral
/// group sits where it always did (1..3), the RNNoise group where proto v10 put
/// it (4..6), and the two engines added in v43 follow. Nothing reads the
/// declaration order but the wire: [`NrLevel::ALL`] and the picker impose the
/// display order instead.
///
/// The RNNoise variants were called `Ai*` until v43, when renaming them still
/// cost nothing. It would cost something now: the operator's setting is kept in
/// `session.json`, which is JSON and so names its variants. A rename has to
/// keep loading the old spelling (serde `alias`) or every operator on that
/// engine silently comes back up on NR off.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum NrLevel {
    #[default]
    Off,
    // Spectral NR (`SpectralNr`) — the original three, discriminants 1..3.
    Low,
    Medium,
    High,
    // RNNoise (`NeuralNr`) — appended in proto v10, discriminants 4..6.
    RnnLow,
    RnnMed,
    RnnHigh,
    // Spectral bleach (`SpecBleachNr`) — appended in v43, discriminants 7..9.
    SpecLow,
    SpecMed,
    SpecHigh,
    // DeepFilterNet3 (`DeepFilterNr`) — appended in v43, discriminants 10..12.
    DfLow,
    DfMed,
    DfHigh,
}

impl NrLevel {
    /// Every setting, in the order the picker lists them.
    pub const ALL: [NrLevel; 13] = [
        NrLevel::Off,
        NrLevel::RnnLow,
        NrLevel::RnnMed,
        NrLevel::RnnHigh,
        NrLevel::DfLow,
        NrLevel::DfMed,
        NrLevel::DfHigh,
        NrLevel::SpecLow,
        NrLevel::SpecMed,
        NrLevel::SpecHigh,
        NrLevel::Low,
        NrLevel::Medium,
        NrLevel::High,
    ];

    /// Suffix shown after "NR" on the chip (Off shows just "NR"). The original
    /// spectral NR keeps its bare "Low"/"Mid"/"High" — it is the one an operator
    /// may already have muscle memory for.
    pub fn label(self) -> &'static str {
        match self {
            NrLevel::Off => "Off",
            NrLevel::Low => "Low",
            NrLevel::Medium => "Mid",
            NrLevel::High => "High",
            NrLevel::RnnLow => "RNN Low",
            NrLevel::RnnMed => "RNN Med",
            NrLevel::RnnHigh => "RNN High",
            NrLevel::SpecLow => "SPEC Low",
            NrLevel::SpecMed => "SPEC Med",
            NrLevel::SpecHigh => "SPEC High",
            NrLevel::DfLow => "DFNR Low",
            NrLevel::DfMed => "DFNR Med",
            NrLevel::DfHigh => "DFNR High",
        }
    }

    pub fn is_on(self) -> bool {
        !matches!(self, NrLevel::Off)
    }

    /// Which denoiser this runs, or `None` when NR is off.
    pub fn engine(self) -> Option<NrEngine> {
        Some(match self {
            NrLevel::Off => return None,
            NrLevel::Low | NrLevel::Medium | NrLevel::High => NrEngine::Spectral,
            NrLevel::RnnLow | NrLevel::RnnMed | NrLevel::RnnHigh => NrEngine::Rnn,
            NrLevel::SpecLow | NrLevel::SpecMed | NrLevel::SpecHigh => NrEngine::SpecBleach,
            NrLevel::DfLow | NrLevel::DfMed | NrLevel::DfHigh => NrEngine::DeepFilter,
        })
    }

    /// How hard it is pushed, or `None` when NR is off.
    pub fn strength(self) -> Option<NrStrength> {
        Some(match self {
            NrLevel::Off => return None,
            NrLevel::Low | NrLevel::RnnLow | NrLevel::SpecLow | NrLevel::DfLow => NrStrength::Low,
            NrLevel::Medium | NrLevel::RnnMed | NrLevel::SpecMed | NrLevel::DfMed => {
                NrStrength::Med
            }
            NrLevel::High | NrLevel::RnnHigh | NrLevel::SpecHigh | NrLevel::DfHigh => {
                NrStrength::High
            }
        })
    }

    /// The same strength on a different engine. From `Off`, starts at Med — the
    /// level worth trying first on every one of them.
    pub fn with_engine(self, e: NrEngine) -> NrLevel {
        e.at(self.strength().unwrap_or(NrStrength::Med))
    }

    /// The same engine at a different strength. From `Off`, picks RNNoise: the
    /// cheapest engine that works on voice, and the one that needs no model.
    pub fn with_strength(self, s: NrStrength) -> NrLevel {
        self.engine().unwrap_or(NrEngine::Rnn).at(s)
    }

    /// One step for a button or a knob: Off → Low → Med → High → Off, *within
    /// the engine already selected*. A single control cannot usefully walk
    /// thirteen states, and the engine is a considered choice — something set
    /// once from the picker, not something a footswitch changes underfoot.
    ///
    /// From Off this starts on RNNoise; the level is the whole state, so
    /// switching off forgets which engine was on rather than adding a second
    /// field to the wire to remember it.
    pub fn next(self) -> NrLevel {
        match self.strength() {
            None => NrLevel::RnnLow,
            Some(NrStrength::Low) => self.with_strength(NrStrength::Med),
            Some(NrStrength::Med) => self.with_strength(NrStrength::High),
            Some(NrStrength::High) => NrLevel::Off,
        }
    }

    /// The next engine at the same strength, wrapping — the other half of what
    /// the picker offers, for a control surface with a button to spare.
    pub fn next_engine(self) -> NrLevel {
        match self.engine() {
            None => NrEngine::Rnn.at(NrStrength::Med),
            Some(e) => self.with_engine(e.next()),
        }
    }

    /// Spectral-NR tuning: `(noise over-estimation factor, minimum gain floor)`.
    /// A larger over-estimate removes more of the noise; a lower floor lets weak
    /// bins be attenuated further — more aggressive, at more risk of artefacts.
    /// The over-factors are modest because the MCRA estimator is unbiased (it
    /// tracks the noise mean, not an under-estimated minimum), so ~1.0 already
    /// removes stationary noise; higher values are pure over-subtraction.
    /// Neutral (unused) for Off and for every other engine.
    pub fn params(self) -> (f32, f32) {
        match self {
            NrLevel::Low => (1.0, 0.30),
            NrLevel::Medium => (1.4, 0.14),
            NrLevel::High => (2.0, 0.07),
            _ => (1.0, 1.0),
        }
    }

    /// Spectral-bleach tuning: `(reduction in dB, residue whitening 0..=1)`.
    /// Whitening flattens what is left so the residue reads as even hiss rather
    /// than as musical noise, which matters more the harder the reduction is
    /// pushed. 20 dB is the top: the algorithm will take 40, and on a radio
    /// signal that sounds like a swimming pool.
    pub fn spec_params(self) -> (f32, f32) {
        match self {
            NrLevel::SpecLow => (6.0, 0.00),
            NrLevel::SpecMed => (12.0, 0.15),
            NrLevel::SpecHigh => (20.0, 0.30),
            _ => (0.0, 0.0),
        }
    }

    /// RNNoise wet/dry depth (0 = bypass, 1 = full RNNoise). Only meaningful for
    /// the `Rnn*` variants.
    pub fn rnn_mix(self) -> f32 {
        match self {
            NrLevel::RnnLow => 0.55,
            NrLevel::RnnMed => 0.8,
            NrLevel::RnnHigh => 1.0,
            _ => 0.0,
        }
    }

    /// DeepFilterNet's attenuation limit, in dB — the most it may take out of
    /// any band. A limit rather than a wet/dry blend because it is the knob the
    /// network itself exposes: a capped mask still tracks the speech, where a
    /// dry blend at 45 % puts 45 % of the noise back with it.
    pub fn df_atten_db(self) -> f32 {
        match self {
            NrLevel::DfLow => 6.0,
            NrLevel::DfMed => 12.0,
            NrLevel::DfHigh => 24.0,
            _ => 0.0,
        }
    }

    /// Make-up gain applied to the listener audio after noise reduction:
    /// suppression lowers the overall level (more so at higher settings), so a
    /// progressively larger boost keeps the perceived loudness roughly constant.
    /// The neural engines preserve speech level far better than spectral
    /// subtraction, so their make-up is gentle — DeepFilterNet is trained to
    /// leave the speech where it found it and needs least of all.
    pub fn makeup_gain(self) -> f32 {
        match self {
            NrLevel::Off => 1.0,
            NrLevel::RnnLow => 1.0,
            NrLevel::RnnMed => 1.1,
            NrLevel::RnnHigh => 1.2,
            NrLevel::DfLow => 1.0,
            NrLevel::DfMed => 1.05,
            NrLevel::DfHigh => 1.15,
            NrLevel::SpecLow => 1.15,
            NrLevel::SpecMed => 1.4,
            NrLevel::SpecHigh => 1.7,
            NrLevel::Low => 1.3,
            NrLevel::Medium => 1.7,
            NrLevel::High => 2.1,
        }
    }
}

/// AGC behavior for a receiver channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgcMode {
    Off,
    Slow,
    Med,
    Fast,
}

impl AgcMode {
    pub const ALL: [AgcMode; 4] = [AgcMode::Off, AgcMode::Slow, AgcMode::Med, AgcMode::Fast];

    pub fn label(self) -> &'static str {
        match self {
            AgcMode::Off => "Off",
            AgcMode::Slow => "Slow",
            AgcMode::Med => "Med",
            AgcMode::Fast => "Fast",
        }
    }

    /// Cycle to the next setting: Off → Slow → Med → Fast → Off.
    pub fn next(self) -> AgcMode {
        match self {
            AgcMode::Off => AgcMode::Slow,
            AgcMode::Slow => AgcMode::Med,
            AgcMode::Med => AgcMode::Fast,
            AgcMode::Fast => AgcMode::Off,
        }
    }

    /// Hang time in milliseconds; `None` means AGC disabled.
    pub fn hang_ms(self) -> Option<f32> {
        match self {
            AgcMode::Off => None,
            AgcMode::Slow => Some(1000.0),
            AgcMode::Med => Some(500.0),
            AgcMode::Fast => Some(100.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A mode whose passband is a channel about the carrier must have nothing
    /// but symmetric presets, and a mode with an asymmetric preset must not
    /// claim to be one: the drag rule and the preset buttons are two routes to
    /// the same passband, and an operator who reaches one shape by clicking
    /// and cannot reach it by dragging has found a bug, not a policy.
    #[test]
    fn symmetric_modes_have_symmetric_presets() {
        for m in Mode::ALL {
            let all_symmetric = m.filter_presets().iter().all(|(_, lo, hi)| lo == &-hi);
            if m.filter_symmetric() {
                assert!(all_symmetric, "{m:?} mirrors its edges but has an off-centre preset");
            } else if !m.filter_presets().is_empty() {
                assert!(
                    !all_symmetric || m == Mode::Adsb || m == Mode::Vdl2 || m == Mode::Ais,
                    "{m:?} has only symmetric presets — should its edges mirror?"
                );
            }
        }
    }

    /// `Mode` carries the same postcard-by-declaration-index contract as
    /// [`NrLevel`] below, and is serialised into far more: every stored config,
    /// every `RadioState` on the wire, every remote client's idea of what the
    /// radio is doing. Inserting a variant rather than appending one renames
    /// every mode after it, silently, on disk and on the wire alike.
    ///
    /// The whole enum is pinned rather than a sample: a sample only catches an
    /// insertion before the samples it happens to name.
    #[test]
    fn mode_discriminants_are_stable() {
        let pinned = [
            (Mode::Lsb, 0),
            (Mode::Usb, 1),
            (Mode::Cw, 2),
            (Mode::Am, 3),
            (Mode::Sam, 4),
            (Mode::Nfm, 5),
            (Mode::Wfm, 6),
            (Mode::Digu, 7),
            (Mode::Digl, 8),
            (Mode::Dsb, 9),
            (Mode::Spec, 10),
            (Mode::Ft8, 11),
            (Mode::Ft4, 12),
            (Mode::Psk, 13),
            (Mode::Rtty, 14),
            (Mode::Sstv, 15),
            (Mode::Olivia, 16),
            (Mode::Thor, 17),
            (Mode::Fsq, 18),
            (Mode::RfPaint, 19),
            (Mode::Rade, 20),
            (Mode::Hell, 21),
            (Mode::Rifp, 22),
            (Mode::Wefax, 23),
            (Mode::Js8, 24),
            (Mode::Wspr, 25),
            (Mode::Ft2, 26),
            (Mode::Packet, 27),
            (Mode::PacketHf, 28),
            (Mode::Drm, 29),
            (Mode::Aprs, 30),
            (Mode::SstvFm, 31),
            (Mode::Adsb, 32),
            (Mode::RttyFm, 33),
            (Mode::Navtex, 34),
            (Mode::Vdl2, 35),
            (Mode::Isb, 36),
            (Mode::Ais, 37),
        ];
        for (mode, index) in pinned {
            assert_eq!(mode as u8, index, "{} moved", mode.label());
        }
    }

    /// The frequency of a contact is the dial only where the mode listens on
    /// it. CW listens — and keys — a sidetone-pitch up, and RTTY a tone pair
    /// up, so logging the dial logs both of them low; that was issue #143.
    #[test]
    fn on_air_frequency_takes_the_tone_offset_into_account() {
        // CW at a 700 Hz pitch: the signal is 700 Hz above the readout.
        assert_eq!(Mode::Cw.on_air_hz(14_050_000.0, 700.0), 14_050_700.0);
        // RTTY on the standard pair, and the pair is where it starts.
        assert_eq!(Mode::Rtty.standard_tone_offset_hz(), Some(crate::RTTY_CENTER_HZ));
        assert_eq!(Mode::Rtty.on_air_hz(14_080_000.0, crate::RTTY_CENTER_HZ), 14_082_210.0);
        // The analog modes are on the dial and ignore the cursor, whatever a
        // stale one from the last digital mode happens to hold.
        assert_eq!(Mode::Usb.on_air_hz(14_200_000.0, 1500.0), 14_200_000.0);
        assert_eq!(Mode::Lsb.on_air_hz(3_700_000.0, 1500.0), 3_700_000.0);
        // A mode on the lower sideband works *below* its dial. SSTV is LSB on
        // 40 m and USB above, and the answer follows the band.
        assert_eq!(Mode::Sstv.on_air_hz(7_171_000.0, 1500.0), 7_169_500.0);
        assert_eq!(Mode::Sstv.on_air_hz(14_230_000.0, 1500.0), 14_231_500.0);
        // The carrier-centred modes key the carrier itself: no offset to take.
        assert_eq!(Mode::Packet.on_air_hz(144_800_000.0, 1500.0), 144_800_000.0);
        assert_eq!(Mode::Rifp.on_air_hz(144_800_000.0, 1500.0), 144_800_000.0);
        // Only RTTY holds its tones; every other keyboard mode picks an offset.
        assert!(Mode::Rtty.holds_standard_tones());
        assert!(!Mode::Psk.holds_standard_tones());
        assert!(!Mode::Ft8.holds_standard_tones());
    }

    /// A mode missing from [`Mode::ALL`] compiles, persists and decodes — and
    /// is simply unreachable, because `ALL` is what the picker and the mode
    /// cycle are built from. Nothing else notices.
    #[test]
    fn every_mode_is_reachable_from_all() {
        for (mode, index) in Mode::ALL.iter().zip(0u8..) {
            let _ = (mode, index);
        }
        // `Mode::ALL`'s length is checked by the array type; what needs
        // checking is that it is a permutation of the enum, with nothing
        // dropped and nothing listed twice.
        // The last variant *by discriminant*, which is the one appended most
        // recently — not the one that reads last in the picker.
        let last = Mode::Ais as u8;
        for i in 0..=last {
            let present = Mode::ALL.iter().filter(|m| **m as u8 == i).count();
            assert_eq!(present, 1, "discriminant {i} appears {present} times in Mode::ALL");
        }
        assert_eq!(Mode::ALL.len(), last as usize + 1);
    }

    /// RTTY on an FM carrier is the same modem on a different radio, and every
    /// table that decides *where the signal is* has to say so (issue #214).
    ///
    /// The tone offset is the one that bites. On a sideband the mark/space pair
    /// sits 2210 Hz above the dial, so the contact is logged there; on a channel
    /// the tones are inside the FM carrier and the dial *is* the frequency.
    /// Copying `Mode::Rtty`'s answer would log every VHF bulletin 2.2 kHz high.
    #[test]
    fn rtty_on_fm_is_a_channel_not_a_sideband() {
        assert!(Mode::RttyFm.is_text_modem(), "it is the RTTY modem and wants the RTTY panel");
        assert!(Mode::RttyFm.is_digital());
        assert!(Mode::RttyFm.is_fm_carrier(), "the level is deviation, not drive");
        assert!(Mode::RttyFm.is_carrier_centered(), "the dial is the channel centre");

        // No tone offset: the dial is where the contact was.
        assert_eq!(Mode::RttyFm.standard_tone_offset_hz(), None);
        assert_eq!(Mode::RttyFm.on_air_hz(145_500_000.0, crate::RTTY_CENTER_HZ), 145_500_000.0);
        // …where the HF twin's is not.
        assert_eq!(Mode::Rtty.on_air_hz(14_080_000.0, crate::RTTY_CENTER_HZ), 14_082_210.0);

        // An FM channel's passband, whatever the band, where the sideband twin
        // has a sideband's.
        assert_eq!(Mode::RttyFm.default_filter(), (-8000.0, 8000.0));
        for dial in [50_150_000.0, 145_500_000.0, 433_500_000.0, 7_040_000.0] {
            assert_eq!(
                Mode::RttyFm.default_filter_at(dial),
                (-8000.0, 8000.0),
                "at {:.3} MHz",
                dial / 1e6
            );
        }
        assert_ne!(Mode::Rtty.default_filter(), Mode::RttyFm.default_filter());
    }

    /// The wire is the declaration order. Every discriminant that has ever been
    /// on the wire is pinned here, so a variant inserted rather than appended
    /// fails the build instead of silently renaming everyone's noise reduction.
    #[test]
    fn nr_discriminants_are_stable() {
        assert_eq!(NrLevel::Off as u8, 0);
        assert_eq!(NrLevel::Low as u8, 1);
        assert_eq!(NrLevel::Medium as u8, 2);
        assert_eq!(NrLevel::High as u8, 3);
        // Were AiLow/AiMed/AiHigh before proto v43; the rename is invisible to
        // postcard, the positions are not.
        assert_eq!(NrLevel::RnnLow as u8, 4);
        assert_eq!(NrLevel::RnnMed as u8, 5);
        assert_eq!(NrLevel::RnnHigh as u8, 6);
        assert_eq!(NrLevel::SpecLow as u8, 7);
        assert_eq!(NrLevel::SpecMed as u8, 8);
        assert_eq!(NrLevel::SpecHigh as u8, 9);
        assert_eq!(NrLevel::DfLow as u8, 10);
        assert_eq!(NrLevel::DfMed as u8, 11);
        assert_eq!(NrLevel::DfHigh as u8, 12);
    }

    #[test]
    fn nr_engine_and_strength_round_trip() {
        for l in NrLevel::ALL {
            let (Some(e), Some(s)) = (l.engine(), l.strength()) else {
                assert_eq!(l, NrLevel::Off);
                continue;
            };
            assert_eq!(e.at(s), l, "{l:?} did not round-trip through {e:?}/{s:?}");
        }
    }

    #[test]
    fn nr_all_lists_every_engine_at_every_strength_exactly_once() {
        assert_eq!(NrLevel::ALL.len(), 1 + NrEngine::ALL.len() * NrStrength::ALL.len());
        for e in NrEngine::ALL {
            for s in NrStrength::ALL {
                let want = e.at(s);
                assert_eq!(
                    NrLevel::ALL.iter().filter(|l| **l == want).count(),
                    1,
                    "{want:?} is not in ALL exactly once"
                );
            }
        }
    }

    /// A button walks four states and comes back, without leaving its engine.
    #[test]
    fn nr_next_stays_on_its_engine() {
        for e in NrEngine::ALL {
            let mut l = e.at(NrStrength::Low);
            for _ in 0..2 {
                l = l.next();
                assert_eq!(l.engine(), Some(e));
            }
            assert_eq!(l.next(), NrLevel::Off);
        }
    }

    #[test]
    fn nr_next_engine_holds_the_strength_and_wraps() {
        let mut l = NrEngine::ALL[0].at(NrStrength::High);
        for _ in 0..NrEngine::ALL.len() {
            assert_eq!(l.strength(), Some(NrStrength::High));
            l = l.next_engine();
        }
        assert_eq!(l, NrEngine::ALL[0].at(NrStrength::High), "next_engine did not wrap");
    }

    /// Reaching for a strength with NR off switches it on rather than doing
    /// nothing, and reaching for an engine keeps the strength you had.
    #[test]
    fn nr_from_off_picks_sensible_defaults() {
        assert_eq!(NrLevel::Off.with_strength(NrStrength::High), NrLevel::RnnHigh);
        assert_eq!(NrLevel::Off.with_engine(NrEngine::DeepFilter), NrLevel::DfMed);
        assert_eq!(NrLevel::SpecHigh.with_engine(NrEngine::Rnn), NrLevel::RnnHigh);
    }

    /// A burst that ran past its slot would key over the next one, and a slot
    /// that was not a whole number of minutes or an even part of one would put
    /// the boundaries somewhere no operator's clock shows.
    #[test]
    fn every_slotted_modes_burst_fits_a_slot_the_minute_agrees_with() {
        for mode in Mode::ALL {
            let Some(t) = mode.slot_timing() else { continue };
            assert!(
                t.tx_offset_s + t.burst_s < t.slot_s,
                "{mode:?}: a {}s burst keyed {}s in overruns a {}s slot",
                t.burst_s,
                t.tx_offset_s,
                t.slot_s
            );
            let (long, short) = if t.slot_s >= 60.0 { (t.slot_s, 60.0) } else { (60.0, t.slot_s) };
            assert!(
                (long / short).fract() < 1e-9,
                "{mode:?}: a {}s slot is not a minute's worth of one",
                t.slot_s
            );
            assert!(t.burst_end_frac() > 0.0 && t.burst_end_frac() < 1.0, "{mode:?}");
        }
    }

    /// The modes with a turn to show, and only those: JS8 answers through its
    /// speed instead, and a mode with no slots must not be given FT8's.
    #[test]
    fn only_the_slotted_modes_have_a_slot_clock() {
        for mode in Mode::ALL {
            let expected = matches!(mode, Mode::Ft8 | Mode::Ft4 | Mode::Ft2 | Mode::Wspr);
            assert_eq!(mode.slot_timing().is_some(), expected, "{mode:?}");
        }
        assert_eq!(Mode::Js8.slot_timing(), None);
    }

    /// FT2 runs FT4's exchange four times faster than FT8 does, which is the
    /// whole mode: the slot lengths are what say so.
    #[test]
    fn the_ft_family_slots_stand_in_the_published_ratio() {
        let ft8 = Mode::Ft8.slot_timing().unwrap();
        let ft4 = Mode::Ft4.slot_timing().unwrap();
        let ft2 = Mode::Ft2.slot_timing().unwrap();
        assert_eq!(ft8.slot_s, 2.0 * ft4.slot_s);
        assert_eq!(ft4.slot_s, 2.0 * ft2.slot_s);
    }

    /// SSTV and RADE follow phone practice: the low bands are LSB, everything
    /// above is USB, and no other mode's sideband moves with the dial.
    #[test]
    fn the_phone_modes_take_the_low_bands_lower_sideband() {
        for mode in [Mode::Sstv, Mode::Rade] {
            for dial in [1_890_000.0, 3_730_000.0, 3_845_000.0, 7_165_000.0, 7_177_000.0] {
                assert!(mode.is_lower_sideband_at(dial), "{mode:?} at {dial} should be LSB");
                let (lo, hi) = mode.default_filter_at(dial);
                assert!(
                    lo < 0.0 && hi <= 0.0,
                    "{mode:?} at {dial}: passband {lo}..{hi} is not below the dial"
                );
                // Mirrored, not redesigned: the same occupied bandwidth either
                // way — a picture's, or the autoencoder's carrier set.
                let (ulo, uhi) = mode.default_filter();
                assert_eq!((hi - lo), (uhi - ulo));
            }
            // 60 m is a low band worked upper sideband, and 30 m is the first
            // of the ones nothing argues about.
            for dial in [5_357_000.0, 10_130_000.0, 14_236_000.0, 21_340_000.0, 144_500_000.0] {
                assert!(!mode.is_lower_sideband_at(dial), "{mode:?} at {dial} should be USB");
                assert_eq!(mode.default_filter_at(dial), mode.default_filter());
            }
        }
    }

    /// The band-aware answer differs from the mode-only one for those two alone
    /// — 40 m does not turn FT8 or PSK31 upside down. [`Mode::SstvFm`] is not in
    /// it either: an FM carrier has no sideband to be on the wrong side of.
    #[test]
    fn only_the_phone_modes_change_sideband_with_the_band() {
        for mode in Mode::ALL {
            for dial in [1_890_000.0, 3_730_000.0, 7_171_000.0, 14_230_000.0, 145_500_000.0] {
                let differs = mode.is_lower_sideband_at(dial) != mode.is_lower_sideband();
                let want = mode.sideband_follows_band() && dial < 10_000_000.0;
                assert_eq!(differs, want, "{mode:?} at {dial}");
            }
        }
        assert!(Mode::Sstv.sideband_follows_band() && Mode::Rade.sideband_follows_band());
        assert!(!Mode::SstvFm.sideband_follows_band());
    }
}
