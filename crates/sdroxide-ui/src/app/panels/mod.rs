//! The bottom panel: whichever operating mode is selected owns it.
//!
//! [`SdroxideApp::digi_panel`] is the dispatcher — it picks the panel for the
//! current [`Mode`] and hands it the height it may use. One submodule per
//! panel:
//!
//! - [`adsb`] — the aircraft list and the 1090 MHz radar picture
//! - [`ais`] — the vessel list and the 162 MHz marine chart
//! - [`vdl2`] — the VDL Mode 2 message log and station list
//! - [`aprs`] — the APRS station list, map and messages
//! - [`cw`] — the Morse decode/keyboard panel, the one non-digital mode here
//! - [`decodes`] — the FT8/FT4/JS8 decode list and the QSO sequencer beside it
//! - [`text_modem`] — PSK, RTTY, Olivia, THOR, Contestia and Hellschreiber
//! - [`js8`], [`fsq`] — the two keyboard modes with their own message model
//! - [`sstv`], [`wefax`], [`rf_paint`] — the image modes
//! - [`wspr`] — the propagation beacon: receptions and the beacon's own cycle
//! - [`rade`] — FreeDV / RADE digital voice
//! - [`setup`] — the digimode setup window the panels share
//! - [`widgets`] — the row and station-card widgets several panels draw

pub(in crate::app) mod adsb;
pub(in crate::app) mod ais;
pub(in crate::app) mod aprs;
pub(in crate::app) mod cw;
pub(in crate::app) mod decodes;
pub(in crate::app) mod fsq;
pub(in crate::app) mod js8;
mod navtex;
pub(in crate::app) mod packet;
pub(in crate::app) mod rade;
pub(in crate::app) mod rf_paint;
pub(in crate::app) mod setup;
pub(in crate::app) mod sstv;
pub(in crate::app) mod text_modem;
pub(in crate::app) mod vdl2;
pub(in crate::app) mod wefax;
pub(in crate::app) mod widgets;
pub(in crate::app) mod wspr;

use eframe::egui::{self, RichText};
use sdroxide_types::{Band, Command, Mode};

use crate::app::SdroxideApp;

/// The waterfall's tab. Always last, and every mode has one: on a phone the
/// panadapter is a view of its own rather than a strip above the panel, because
/// a third of that height is not enough to work a mode *and* watch a band.
pub(in crate::app) const TAB_WFALL: &str = "WFALL";

/// The panes a mode's panel splits into, in the order the tabs show them.
///
/// A phone draws one at a time — two columns want 180 and 220 points before
/// either has said anything, which is more than the screen — so this is also
/// the answer to "what is there to switch between". Modes whose panel is a
/// single column name it anyway, so the tab row reads the same everywhere and
/// the waterfall always has something to sit beside.
pub(in crate::app) fn panel_panes(mode: Mode) -> &'static [&'static str] {
    match mode {
        Mode::Ft8 | Mode::Ft4 | Mode::Ft2 => &["DECODES", "QSO"],
        Mode::Js8 => &["HEARD", "CHAT"],
        // No QSO pane, because there is no QSO: WSPR measures paths. The map
        // is its own pane rather than sharing one, so a narrow screen can show
        // it whole instead of squeezing it under something else.
        Mode::Wspr => &["SPOTS", "MAP", "STATUS"],
        Mode::Fsq => &["HEARD", "TRAFFIC"],
        Mode::Sstv | Mode::SstvFm | Mode::Rifp => &["RECEIVE", "SEND"],
        // MONITOR is every frame heard on the channel, TERMINAL is the
        // connected session — the two things a packet operator watches, and
        // they move independently, so they get a pane each rather than sharing
        // one.
        Mode::Packet | Mode::PacketHf => &["MONITOR", "TERMINAL"],
        // Three, because an APRS operator watches three things that move
        // independently: who is out there, where they are, and what they said.
        Mode::Aprs => &["STATIONS", "MESSAGES", "MAP"],
        // Two, because there is nothing to say back: this is a surveillance
        // downlink, and the aircraft are not listening.
        Mode::Adsb => &["AIRCRAFT", "MAP"],
        // Two, because a datalink is a conversation and a list of who is having
        // it. No map: VDL2 carries a position only occasionally, so one would be
        // mostly empty of the traffic this panel exists to show.
        Mode::Vdl2 => &["MESSAGES", "STATIONS"],
        // Two, for the reason ADS-B has two: a safety broadcast nobody answers,
        // and the only two questions about it are what is out there and where.
        Mode::Ais => &["VESSELS", "CHART"],
        Mode::Wefax => &["CHART", "SAVED"],
        Mode::Navtex => &["MESSAGES", "READING"],
        Mode::RfPaint => &["TEXT", "IMAGE"],
        // The keyboard modes and RADE are one column already: receive above,
        // what you are sending below it.
        _ => &["PANEL"],
    }
}

impl SdroxideApp {
    /// Fold everything this frame knows into the propagation field, and hand
    /// back the texture the map should paint under itself.
    ///
    /// One place, called by every panel that draws a map, so the flat map and
    /// the globe are never fed different things. `None` when the operator has
    /// the layer off or there is nothing to draw yet.
    pub(in crate::app) fn prop_texture(
        &mut self,
        ctx: &egui::Context,
        dial_hz: f64,
    ) -> Option<eframe::egui::TextureId> {
        let my_grid =
            self.digi_status.as_ref().map(|s| s.config.my_grid.clone()).unwrap_or_default();
        let now = crate::time::now_unix();
        let v = self.view.solar3d;
        self.prop.set_halflife_min(v.prop_halflife_min);
        self.prop.set_sources(crate::prop_map::PropSources(v.prop_sources));

        // Everything below this needs to know where "here" is: each of these
        // sources reports one end of a path and takes the other from the
        // operator's own locator. A skimmer network does not — those paths are
        // folded as they arrive and are the one thing on this map that works
        // before the locator is filled in.
        if !my_grid.trim().is_empty() {
            self.prop.set_home(&my_grid);

            // Every slotted mode's decodes are observations of a path; which
            // mode they came from only changes the decode floor they are
            // measured against.
            let mode = self.state.rx[0].mode;
            let src = match mode {
                Mode::Ft8 => Some(sdroxide_types::PropSource::Ft8),
                Mode::Ft4 => Some(sdroxide_types::PropSource::Ft4),
                Mode::Ft2 => Some(sdroxide_types::PropSource::Ft2),
                Mode::Js8 => Some(sdroxide_types::PropSource::Js8),
                _ => None,
            };
            if let Some(src) = src {
                let decodes = std::mem::take(&mut self.digi_decodes);
                self.prop.observe_decodes(&decodes, src, dial_hz, &my_grid, now);
                self.digi_decodes = decodes;
            }
            let spots = std::mem::take(&mut self.wspr_spots);
            self.prop.observe_wspr(&spots, &my_grid, now);
            self.wspr_spots = spots;
            let log = std::mem::take(&mut self.qso_log);
            self.prop.observe_log(&log, &my_grid, now);
            self.qso_log = log;
        }

        // The panel map's own display choice, from the panel's own settings —
        // `Solar3dView` belongs to the globe window and is republished over
        // every frame, so a setting the panel wrote there would not survive to
        // the next one.
        if !self.view.prop_on_map {
            return None;
        }
        let field = self.prop.field(now);
        let band = Band::ALL.get(self.view.prop_map_band as usize).copied().unwrap_or(Band::M20);
        let heat_mode = if self.view.prop_map_mode == 0 {
            crate::prop_map::PropMode::PerBand
        } else {
            crate::prop_map::PropMode::AllBands
        };
        // Every live band: the compact chip row has no space for a mask, and
        // picking bands apart is what the globe's menu is for.
        self.prop_heat.texture(ctx, &field, heat_mode, band, u32::MAX).map(|t| t.id())
    }

    /// The chip row that turns the flat map's propagation heat on and picks
    /// what it shows. Drawn just above the map by every panel that has one.
    pub(in crate::app) fn prop_map_controls(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 3.0;
            let on = self.view.prop_on_map;
            let resp = crate::chrome::chip(ui, on, RichText::new("PROP").size(9.5));
            if resp.clicked() {
                self.view.prop_on_map = !on;
            }
            resp.on_hover_text(
                "Shade the map by where signals are actually getting through, on each band. \
                 Every reception is placed at the midpoint of its path — the patch of \
                 ionosphere that bent it — not at the far station, so this is a map of the \
                 sky rather than of where radio amateurs live. Built from paths this \
                 station has taken part in, plus — when the Reverse Beacon Network is \
                 switched on under Settings → Spots — what the world's skimmers are \
                 hearing, which covers the bands this radio is not on.",
            );
            if !on {
                return;
            }
            let combined = self.view.prop_map_mode != 0;
            if crate::chrome::chip(ui, combined, RichText::new("ALL BANDS").size(9.5))
                .on_hover_text("Every band at once, one hue each — overall conditions.")
                .clicked()
            {
                self.view.prop_map_mode = 1;
            }
            if crate::chrome::chip(ui, !combined, RichText::new("ONE BAND").size(9.5))
                .on_hover_text("One band, cold to hot: blue, green, yellow, red.")
                .clicked()
            {
                self.view.prop_map_mode = 0;
            }
            if !combined {
                // Only bands with something in them: a picker full of dead
                // bands is a picker of nothing.
                for b in self.prop.peek().live_bands() {
                    let i = Band::ALL.iter().position(|x| *x == b).unwrap_or(0) as u8;
                    if crate::chrome::chip(
                        ui,
                        self.view.prop_map_band == i,
                        RichText::new(b.label()).size(9.5),
                    )
                    .clicked()
                    {
                        self.view.prop_map_band = i;
                    }
                }
            }
            // The absolute scale behind the colours. Without it the same colour
            // means different things on different evenings, and the map is
            // lying by omission.
            if self.prop_heat.peak_paths > 0.0 {
                ui.label(
                    RichText::new(format!("top ≈ {:.0} paths", self.prop_heat.peak_paths))
                        .size(9.0)
                        .color(crate::theme::gray(110)),
                );
            }
        });
    }
}

/// The full tab row for `mode`: its panes, then the waterfall.
pub(in crate::app) fn panel_tabs(mode: Mode) -> impl Iterator<Item = &'static str> {
    panel_panes(mode).iter().copied().chain(std::iter::once(TAB_WFALL))
}

/// Index of the pane named `name`, for the panels that need to switch to one of
/// their own (answering a call takes an operator to the QSO pane).
pub(in crate::app) fn pane_index(mode: Mode, name: &str) -> usize {
    panel_panes(mode).iter().position(|p| *p == name).unwrap_or(0)
}

/// The frequency of the contact, drawn next to the tone offset it is derived
/// from in the panels whose mode listens away from the dial.
///
/// Worth the room it takes: the readout at the top of the window is the *dial*,
/// and in CW that sits a sidetone-pitch below the signal, in RTTY a tone pair
/// below its mark. So the number to log, to spot and to say on the air is not
/// the big one on screen, and without this there is nowhere to read it. The
/// logbook fills a new entry in from the same place.
pub(in crate::app) fn on_air_readout(ui: &mut egui::Ui, hz: f64) {
    ui.label(
        RichText::new(format!("{:.4} MHz", hz / 1e6)).size(11.0).color(crate::theme::gray(190)),
    )
    .on_hover_text(
        "The frequency you are actually working: the dial plus the tone offset \
         beside it. This is what goes in the log — the dial alone is that much low.",
    );
}

/// The standard dial frequency for `band`, if one exists for `mode`
/// (matched by which band's edges the frequency falls within).
///
/// The shared table wins where it has an answer: those frequencies follow the
/// station's IARU region, and they are what the waterfall's band strip and the
/// frequency chip already show — a band button that jumped somewhere else would
/// be contradicting the rest of the screen. The list below covers the modes the
/// shared table has no convention for, and the VHF/UHF entries it stops short
/// of.
pub(in crate::app) fn digi_freq_for_band(mode: Mode, band: Band) -> Option<f64> {
    let shared = sdroxide_types::digi_channels_in(mode, band);
    // Published conventions only. A frequency this station saved for itself is
    // offered in the ⇵ picker and honoured when the dial is already on it, but
    // a band button is a jump to where the *band* is worked, and answering it
    // with one station's own note would be a surprise (issue #268).
    let shared: Vec<_> = shared.into_iter().filter(|c| !c.mine).collect();
    if !shared.is_empty() {
        // The plain calling frequency, which is the one with no note. Not
        // simply the lowest: FT8's DXpedition frequency is *below* the calling
        // one on five bands, and a band button that dropped the operator into a
        // Fox/Hound window would be a trap.
        let c = shared.iter().find(|c| c.note.is_empty()).unwrap_or(&shared[0]);
        return Some(c.dial_hz);
    }
    let (lo, hi) = band.edges()?;
    digi_dial_freqs(mode).iter().find(|&&(_, hz)| (lo..=hi).contains(&hz)).map(|&(_, hz)| hz)
}

fn digi_dial_freqs(mode: Mode) -> &'static [(&'static str, f64)] {
    match mode {
        // JS8 conventional dials; signals sit in the ~3 kHz above each.
        Mode::Js8 => &[
            ("160m", 1_842_000.0),
            ("80m", 3_578_000.0),
            ("40m", 7_078_000.0),
            ("30m", 10_130_000.0),
            ("20m", 14_078_000.0),
            ("17m", 18_104_000.0),
            ("15m", 21_078_000.0),
            ("12m", 24_922_000.0),
            ("10m", 28_078_000.0),
        ],
        // PSK31 activity centres (USB dial; signals sit ~1 kHz above).
        Mode::Psk => &[
            ("80m", 3_580_000.0),
            ("40m", 7_040_000.0),
            ("30m", 10_142_000.0),
            ("20m", 14_070_000.0),
            ("17m", 18_097_000.0),
            ("15m", 21_070_000.0),
            ("12m", 24_920_000.0),
            ("10m", 28_120_000.0),
        ],
        // RTTY sub-band starts (USB dial).
        Mode::Rtty => &[
            ("80m", 3_580_000.0),
            ("40m", 7_040_000.0),
            ("30m", 10_140_000.0),
            ("20m", 14_080_000.0),
            ("17m", 18_100_000.0),
            ("15m", 21_080_000.0),
            ("12m", 24_920_000.0),
            ("10m", 28_080_000.0),
        ],
        Mode::Ft4 => &[
            ("80m", 3_575_000.0),
            ("40m", 7_047_500.0),
            ("30m", 10_140_000.0),
            ("20m", 14_080_000.0),
            ("17m", 18_104_000.0),
            ("15m", 21_140_000.0),
            ("12m", 24_919_000.0),
            ("10m", 28_180_000.0),
        ],
        // FT2 dials, per the reference client. 60m is in the mode's own list
        // but not in this row, which carries one button per contest band.
        Mode::Ft2 => &[
            ("80m", 3_578_000.0),
            ("40m", 7_062_000.0),
            ("30m", 10_144_000.0),
            ("20m", 14_084_000.0),
            ("17m", 18_108_000.0),
            ("15m", 21_144_000.0),
            ("12m", 24_923_000.0),
            ("10m", 28_184_000.0),
        ],
        // FreeDV calling frequencies. Dial, on whichever sideband the band is
        // worked: 80 m and 40 m are LSB, everything above USB — the switch is
        // sdroxide's to make, not the operator's (`Mode::sideband_follows_band`).
        Mode::Rade => &[
            ("80m", 3_625_000.0),
            ("40m", 7_177_000.0),
            ("20m", 14_236_000.0),
            ("15m", 21_313_000.0),
            ("10m", 28_330_000.0),
            // No published 2 m calling frequency exists — FreeDV's own
            // suggested-frequency list stops at HF — but there is real RADE
            // activity on the band; 144.200 is where it gathers.
            ("2m", 144_200_000.0),
        ],
        // SSTV calling frequencies (USB).
        Mode::Sstv => &[
            ("80m", 3_730_000.0),
            ("40m", 7_171_000.0),
            ("20m", 14_230_000.0),
            ("15m", 21_340_000.0),
            ("10m", 28_680_000.0),
            // The Region 1 image-mode calling frequency and the narrow-band
            // SSTV activity centre. Deliberately not the FM SSTV channel
            // (433.400): this mode demodulates a USB passband.
            ("2m", 144_500_000.0),
            ("70cm", 432_500_000.0),
        ],
        // The VHF/UHF image channels, where the picture rides an FM carrier —
        // see `SSTV_FM_DIALS` for where each comes from and why the list is
        // this short.
        Mode::SstvFm => &[("6m", 50_510_000.0), ("2m", 144_500_000.0), ("70cm", 433_400_000.0)],
        // RTTY on an FM carrier has no worldwide convention at all: where it is
        // still used it is a national society's own bulletin on a local
        // channel, and the frequency comes from that society rather than from
        // a band plan. These are the FM calling channels — somewhere to start
        // from and a band to be on, not a place anybody is transmitting RTTY
        // (issue #214). Store the real one in a memory.
        Mode::RttyFm => &[("6m", 50_150_000.0), ("2m", 145_500_000.0), ("70cm", 433_500_000.0)],
        // The three NAVTEX channels, as *dial* frequencies: the service quotes
        // the assigned frequency (518, 490, 4209.5 kHz), which for an F1B
        // emission is the centre of the two tones, so upper sideband sits
        // 1700 Hz below it — see `NAVTEX_TONE_HZ`. Labelled by the channel,
        // because that is what a schedule names.
        Mode::Navtex => &[("518", 516_300.0), ("490", 488_300.0), ("4209.5", 4_207_800.0)],
        // Olivia activity centres (USB dial).
        Mode::Olivia => &[
            ("80m", 3_581_000.0),
            ("40m", 7_073_000.0),
            ("30m", 10_142_000.0),
            ("20m", 14_076_000.0),
            ("17m", 18_103_000.0),
            ("15m", 21_076_000.0),
            ("10m", 28_076_000.0),
        ],
        // THOR / DominoEX activity centres (USB dial).
        Mode::Thor => &[
            ("80m", 3_580_000.0),
            ("40m", 7_070_000.0),
            ("30m", 10_147_000.0),
            ("20m", 14_073_000.0),
            ("17m", 18_103_000.0),
            ("15m", 21_073_000.0),
            ("10m", 28_073_000.0),
        ],
        // FSQCALL calling frequencies (USB dial; signals ~1500 Hz above).
        Mode::Fsq => &[
            ("80m", 3_575_000.0),
            ("40m", 7_105_000.0),
            ("30m", 10_144_000.0),
            ("20m", 14_105_000.0),
            ("17m", 18_104_000.0),
            ("15m", 21_105_000.0),
            ("10m", 28_105_000.0),
        ],
        // Hellschreiber (USB dial), from hellschreiber.com's narrow-band digimode
        // band plan of 18 March 2019 — its "common calling & operating" column,
        // taking IARU Region 1 where that column is split, to match the Region 1
        // defaults `Band::edges` already uses.
        //
        // Two deliberate departures, on 15 m and 10 m: that table's own calling
        // frequencies there (21074 / 28074) fall *outside* the operating ranges
        // it lists in the same cell, and both sit exactly on the FT8 sub-band.
        // The range starts are used instead — internally consistent, clear of
        // FT8, and what the Feld Hell Club lists. 6 m is not in that table at
        // all, so it keeps the club's figure.
        Mode::Hell => &[
            ("160m", 1_840_000.0),
            ("80m", 3_574_000.0),
            ("60m", 5_351_500.0),
            ("40m", 7_040_000.0),
            ("30m", 10_144_000.0),
            ("20m", 14_073_000.0),
            ("17m", 18_104_000.0),
            ("15m", 21_063_000.0),
            ("12m", 24_924_000.0),
            ("10m", 28_063_000.0),
            ("6m", 50_286_000.0),
        ],
        // RF Paint has no defined calling frequency — offer no band presets.
        Mode::RfPaint => &[],
        // RIFP assigns no frequency at all: 433.92 MHz is the deployment
        // example the draft names, and the others are the middle of the
        // segments where a 25 kHz channel is a realistic thing to ask for
        // (10 m FM and the 6 m all-modes part). The dial is the signal's
        // *centre* here, not its lower edge as in every mode above.
        Mode::Rifp => &[
            ("10m", 29_600_000.0),
            ("6m", 51_250_000.0),
            // The 2 m image/facsimile corner: 144.700 is the FAX calling
            // frequency, inside the all-modes segment.
            ("2m", 144_700_000.0),
            ("70cm", sdroxide_types::RIFP_CALLING_HZ),
        ],
        // FT8 (and default).
        _ => &[
            ("160m", 1_840_000.0),
            ("80m", 3_573_000.0),
            ("40m", 7_074_000.0),
            ("30m", 10_136_000.0),
            ("20m", 14_074_000.0),
            ("17m", 18_100_000.0),
            ("15m", 21_074_000.0),
            ("12m", 24_915_000.0),
            ("10m", 28_074_000.0),
            ("6m", 50_313_000.0),
            ("4m", 70_174_000.0),
            ("2m", 144_174_000.0),
            // The European convention, mirroring 144.174; North American
            // activity sits elsewhere (432.065 among others).
            ("70cm", 432_174_000.0),
        ],
    }
}

impl SdroxideApp {
    /// Drop everything derived from the previous mode's decodes: the waterfall
    /// callsign boxes, the decode list, the world-map station dots and the
    /// clicked-decode preview. Called on every RX mode change so leaving FT8/FT4
    /// (for SSTV, a keyboard mode, or plain SSB) doesn't carry its labels over.
    ///
    /// The transmit buffer is *not* part of this — the one caller that has a
    /// reason to empty it says so itself.
    pub(in crate::app) fn clear_digi_rx(&mut self) {
        self.clear_digi_band_rx();
        // WSPR receptions belong to the band and the slot they came
        // from; carried into another mode they would be a list of
        // things the receiver is no longer listening for. Not a band question,
        // though: each spot carries the absolute frequency it was heard on and
        // the panel prints the band from it, so a hopping beacon's multi-band
        // list is right as it stands.
        self.wspr_spots.clear();
        // The read-along stream anchors on the buffer it was last reading; a
        // new mode fills that buffer with something unrelated, and without a
        // re-anchor the first snapshot after the change would be read out from
        // the beginning. On this side of the split with the buffer it is about:
        // a QSY does not refill it, so re-anchoring would only skip past copy
        // the operator is part way through hearing.
        self.speech.announcer.reset_text();
    }

    /// Drop what belongs to the band we have just left: the decode list, the
    /// waterfall callsign boxes it draws, the world-map station dots, the
    /// clicked-decode preview and the Hell strip.
    ///
    /// A decode records the audio tone it was heard on and nothing about the
    /// dial, so a row from the previous band is indistinguishable from one on
    /// this band — and clicking it moves the transmit offset, queues a call, and
    /// is scored for novelty, all against a dial that has moved out from under
    /// it. Nothing on screen could tell the operator which rows those are, so
    /// they go.
    ///
    /// Narrower than [`Self::clear_digi_rx`] at both ends. The transmit buffer
    /// stays: a QSY rebuilds no controller, so a half-typed over is still going
    /// out exactly as typed and throwing it away would be a surprise. The WSPR
    /// spots stay too — each carries its own RF frequency and the panel prints
    /// the band beside it, so unlike a decode it is never ambiguous about where
    /// it was heard, and WSPR band hopping crosses a band edge every slot on
    /// purpose.
    pub(in crate::app) fn clear_digi_band_rx(&mut self) {
        self.digi_decodes.clear();
        self.digi_stations = Default::default();
        self.digi_preview = None;
        // The Hell raster is a continuous strip with no frame boundary, so
        // leaving it up across the change would splice unrelated text.
        self.hell.clear();
    }

    /// The chips a phone switches the panel's views with, plus the one reading
    /// worth carrying across all of them: how many stations the last slot
    /// decoded, which is the answer to "is the band open".
    ///
    /// Wrapped, so a mode with more panes than the width holds takes a second
    /// row rather than losing the last one — which on most modes is the
    /// waterfall.
    pub(in crate::app) fn digi_tabs(&mut self, ui: &mut egui::Ui, mode: Mode) {
        let selected = self.digi_pane(mode);
        // Pin the content to the width left *inside* the margins. A `Frame`
        // reports an outer rect of content-plus-margins, and a content row that
        // has taken all the width there was makes that outer rect wider than
        // the window — which the parent then expands to include, leaving every
        // row drawn after this one wrapping against a width the screen has not
        // got and clipping whatever crosses the edge.
        const MARGIN: f32 = 8.0;
        let inner_w = (ui.available_width() - 2.0 * MARGIN).max(80.0);
        egui::Frame::new()
            .inner_margin(egui::Margin { left: 8, right: 8, top: 6, bottom: 2 })
            .show(ui, |ui| {
                ui.set_max_width(inner_w);
                ui.horizontal_wrapped(|ui| {
                    for (i, label) in panel_tabs(mode).enumerate() {
                        if crate::chrome::chip(ui, selected == i, label).clicked() {
                            self.view.digi_pane = i;
                        }
                    }
                    // Only the modes whose panel is the decode list have a
                    // count to put here. JS8 is slotted too, but it keeps its
                    // own heard list and never fills `digi_decodes` — a "0 rx"
                    // beside a busy band would be a lie.
                    if matches!(mode, Mode::Ft8 | Mode::Ft4 | Mode::Ft2) {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                RichText::new(format!("{} rx", self.digi_decodes.len()))
                                    .size(10.0)
                                    .color(crate::theme::gray(120)),
                            );
                        });
                    }
                });
            });
    }

    /// The pane showing this frame, clamped to what `mode` actually has. A
    /// stored index only means anything against the mode it was stored in, and
    /// switching from a two-pane mode to a one-pane one must not leave the
    /// panel pointing at a pane that does not exist.
    pub(in crate::app) fn digi_pane(&self, mode: Mode) -> usize {
        self.view.digi_pane.min(panel_panes(mode).len())
    }

    /// Which pane a panel should draw, or `None` where it draws them all.
    ///
    /// `None` on every layout but a phone, and on a phone whenever the
    /// waterfall tab is up — the waterfall replaces the panel rather than
    /// living inside it, so the panel is not drawn at all.
    pub(in crate::app) fn phone_pane(&self, ui: &egui::Ui, mode: Mode) -> Option<usize> {
        if crate::layout::tier(ui.ctx()) != crate::layout::Tier::Phone {
            return None;
        }
        let pane = self.digi_pane(mode);
        (pane < panel_panes(mode).len()).then_some(pane)
    }

    /// The FT8/FT4 operating panel: decode list on the left, QSO area on the
    /// right. Sits below the (zoomed) waterfall in digital modes.
    ///
    /// A phone shows one column at a time instead — see [`Self::digi_tabs`].
    /// Two columns need 180 + 7 + 220 points before either has said anything,
    /// which is more than the whole of the screen.
    pub(in crate::app) fn digi_panel(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        // Full width, above the split: the turn is as much the decode list's
        // clock as the sequencer's, and a phone showing one pane at a time must
        // not lose it with the other.
        self.slot_progress(ui);
        ui.add_space(4.0);
        if let Some(pane) = self.phone_pane(ui, self.state.rx[0].mode) {
            match pane {
                0 => self.decode_list(ui, cmds),
                _ => self.qso_area(ui, cmds),
            }
            return;
        }
        let avail = ui.available_size();
        let handle_w = 7.0;
        // Decode list takes a user-draggable fraction of the width; the QSO area
        // gets the rest (each keeps a usable minimum).
        let left_w = (avail.x * self.view.digi_split_fraction)
            .clamp(180.0, (avail.x - handle_w - 220.0).max(180.0));
        ui.horizontal_top(|ui| {
            // Force a top-down layout: `allocate_ui` would otherwise inherit the
            // parent `horizontal_top` (left-to-right) and lay the rows out
            // sideways, overflowing and shoving the QSO column off-screen.
            ui.allocate_ui_with_layout(
                egui::vec2(left_w, avail.y),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    self.decode_list(ui, cmds);
                },
            );
            // Draggable vertical divider between the decode table and the QSO area.
            let hresp = crate::chrome::split_handle(ui, egui::vec2(handle_w, avail.y), None);
            if hresp.dragged() {
                let d = hresp.drag_delta().x / avail.x.max(1.0);
                self.view.digi_split_fraction =
                    (self.view.digi_split_fraction + d).clamp(0.28, 0.72);
            }
            ui.vertical(|ui| {
                self.qso_area(ui, cmds);
            });
        });
    }

    /// The chip that chooses between putting characters on the air as they are
    /// typed and holding the line back until Return commits it.
    ///
    /// Shared by every panel that types onto the air, because it is one habit
    /// rather than one per mode: an operator who composes a line before sending
    /// it does that in CW and in PSK alike. `hover` is the mode's own reason
    /// for wanting it, which is not the same reason everywhere.
    pub(in crate::app) fn send_on_return_chip(
        &mut self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
        hover: &str,
    ) {
        let on = self.digi_cfg_edit.send_on_enter;
        if crate::chrome::chip(ui, on, RichText::new("SEND ON RETURN").size(10.5))
            .on_hover_text(hover)
            .clicked()
        {
            self.digi_cfg_edit.send_on_enter = !on;
            if self.digi_cfg_seeded {
                cmds.push(Command::SetDigiConfig(self.digi_cfg_edit.clone()));
            }
        }
    }

    /// Empty the received-text window.
    ///
    /// Shared by every panel that copies text, because it is the same wish
    /// everywhere: the page is full of the last hour and the QSO starting now
    /// wants a clean one. Deliberately not the same button as the transmit
    /// box's CLEAR — one throws away what was received, the other stops what is
    /// being sent, and confusing the two mid-over is expensive.
    pub(in crate::app) fn clear_rx_chip(&self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        if crate::chrome::chip(ui, false, RichText::new(" CLEAR RX ").size(10.5))
            .on_hover_text("Empty the receive window. Nothing that is on the air stops.")
            .clicked()
        {
            cmds.push(Command::DigiClearRx);
        }
    }

    /// Commit the transmit box: hand the whole buffer over and start the over.
    ///
    /// What Return does under [`Self::send_on_return_chip`], in every panel
    /// that types onto the air. The break goes in the buffer rather than being
    /// swallowed: the keyboard modes carry it as a line break, CW keys it as a
    /// word space, and Hell has no glyph for it so it costs one blank cell —
    /// which is the gap you want between one committed line and the next
    /// anyway.
    pub(in crate::app) fn commit_tx_line(&mut self, cmds: &mut Vec<Command>) {
        if self.text_tx.trim().is_empty() {
            return;
        }
        if !self.text_tx.ends_with('\n') {
            self.text_tx.push('\n');
        }
        cmds.push(Command::DigiTxText(self.text_tx.clone()));
        cmds.push(Command::DigiTxActive(true));
    }

    /// The mode's conventional operating frequencies, as a chip that opens a
    /// picker.
    ///
    /// Every band the mode has a convention on, not only the one the dial is
    /// in: an operator changing band for FT8 wants 14.074 from a list, and
    /// having to remember it — or to reach for the band buttons and then a
    /// separate number — is the trip to a web page this exists to remove
    /// (issue #210). The band the dial is in leads, so the nearest useful
    /// entries are under the cursor when the popup opens, and the rest follow
    /// in frequency order.
    ///
    /// Absent only for a mode with no convention of its own, and nothing the
    /// operator has saved for it — CW, SSB, and the ones whose frequency is a
    /// property of what they are pointed at rather than of the mode (RF Paint,
    /// RADE, WEFAX, which has its own station picker instead).
    ///
    /// The list is also the operator's own. The net that meets on 3.585 every
    /// Tuesday is not in anybody's global table and never will be, so the
    /// picker saves the dial you are on and offers it back under this mode
    /// afterwards (issue #268). Those entries are marked, and they are the only
    /// ones that can be removed — a published convention is not this station's
    /// to edit.
    ///
    /// The dial is what moves. These are dial frequencies, and the audio
    /// offset within the passband is a separate control that must not be
    /// disturbed by changing band segment.
    pub(in crate::app) fn digi_freq_chip(&self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        let mode = self.state.rx[0].mode;
        let dial = self.state.active_freq_hz();
        let here_band = sdroxide_types::Band::containing(dial);
        let channels = sdroxide_types::digi_channels(mode);
        // No early return for an empty list any more: a mode the tables have no
        // convention for is exactly the one an operator needs to save a
        // frequency *under*, and a chip that only appears once there is
        // something in it can never be the place they save the first one.
        // "On" when the dial is already sitting on one of them, so the chip
        // doubles as a readout of whether you are where the mode expects.
        let here = channels.iter().find(|c| (c.dial_hz - dial).abs() < 1.0);
        let face = match here {
            Some(c) => format!("⇵ {:.3}", c.dial_hz / 1e6),
            None => "⇵ FREQ".to_string(),
        };
        let btn = crate::chrome::chip(ui, here.is_some(), RichText::new(face).size(11.0))
            .on_hover_text(if channels.is_empty() {
                format!(
                    "No frequency is agreed for {} — open this to save the one you are on",
                    mode.label()
                )
            } else {
                format!(
                    "The {} frequencies for {} — picking one tunes the dial, and you can save \
                     your own",
                    channels.len(),
                    mode.label()
                )
            });

        // Grouped by band, the dial's own band first: everything else is a
        // band change, and the entries next to where the operator already is
        // are the ones they are most likely to want.
        let mut groups: Vec<(sdroxide_types::Band, Vec<sdroxide_types::DigiChannel>)> = Vec::new();
        for c in &channels {
            let b = sdroxide_types::Band::containing(c.dial_hz);
            match groups.iter_mut().find(|(gb, _)| *gb == b) {
                Some((_, v)) => v.push(*c),
                None => groups.push((b, vec![*c])),
            }
        }
        if let Some(i) = groups.iter().position(|(b, _)| *b == here_band) {
            groups.swap(0, i);
        }
        let flagged = channels.iter().any(|c| c.outside_data_segment(mode));

        let mut pick = None;
        // Set inside the popup and acted on after it, because both rewrite the
        // whole list and neither may borrow it while the rows are being drawn
        // from it.
        let mut save = false;
        let mut forget: Option<f64> = None;
        let resp = egui::Popup::from_toggle_button_response(&btn)
            .frame(crate::chrome::window_frame())
            .close_behavior(egui::PopupCloseBehavior::CloseOnClick)
            .show(|ui| {
                crate::chrome::window_body_bg(ui);
                ui.set_max_width(300.0);
                ui.label(
                    RichText::new(format!("{} · frequencies", mode.label()))
                        .color(crate::theme::CYAN_DIM())
                        .size(9.5)
                        .strong(),
                );
                ui.add_space(2.0);
                if groups.is_empty() {
                    ui.label(
                        RichText::new("Nothing agreed for this mode — save your own below.")
                            .size(10.0)
                            .weak(),
                    );
                }
                // Scrolled rather than sized to the list: a mode with a
                // convention on every band has more entries than a popup can
                // be tall on a laptop screen.
                egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
                    for (band, chans) in &groups {
                        ui.label(
                            RichText::new(band.label())
                                .color(crate::theme::LINE_LIT())
                                .size(9.5)
                                .strong(),
                        );
                        for c in chans {
                            let on = here.map(|h| h.dial_hz) == Some(c.dial_hz);
                            // A star for the operator's own, so a frequency
                            // this station saved is never mistaken for one the
                            // rest of the world is listening on.
                            let mark = if c.mine { " ★" } else { "  " };
                            let mut text = format!(" {mark} {:.3} MHz", c.dial_hz / 1e6);
                            if !c.note.is_empty() {
                                text.push_str(&format!("   {}", c.note));
                            }
                            let mut rich = RichText::new(text).size(12.0);
                            if c.outside_data_segment(mode) {
                                rich = rich.color(crate::theme::YELLOW());
                            }
                            ui.horizontal(|ui| {
                                let row = ui.selectable_label(on, rich);
                                if c.outside_data_segment(mode) {
                                    row.clone().on_hover_text(format!(
                                        "A global convention that the IARU Region {} band plan \
                                         does not put narrow data on — check your own band plan \
                                         before transmitting here.",
                                        sdroxide_types::region().number()
                                    ));
                                }
                                if row.clicked() {
                                    pick = Some(c.dial_hz);
                                }
                                // Only this station's own entries can go. A
                                // published convention is not ours to delete —
                                // and an operator who removed 14.074 by
                                // accident would have no way to get it back.
                                if c.mine
                                    && ui
                                        .small_button("✕")
                                        .on_hover_text("Forget this saved frequency")
                                        .clicked()
                                {
                                    forget = Some(c.dial_hz);
                                }
                            });
                        }
                    }
                });
                if flagged {
                    ui.add_space(2.0);
                    ui.label(
                        RichText::new(format!(
                            "Amber: outside the Region {} data segment.",
                            sdroxide_types::region().number()
                        ))
                        .color(crate::theme::LINE_LIT())
                        .size(10.0),
                    );
                }
                // The dial you are on, saved under this mode. The whole of what
                // issue #268 asked for beyond the shipped tables: a club net or
                // a local calling frequency is not in anybody's global list, and
                // having tuned it once you should not have to find it again.
                ui.add_space(4.0);
                ui.separator();
                let saved = here.is_some_and(|h| h.mine);
                let can_save = !saved && here.is_none();
                ui.add_enabled_ui(can_save, |ui| {
                    if ui
                        .button(RichText::new(format!("＋ Save {:.3} MHz", dial / 1e6)).size(11.0))
                        .on_hover_text(if saved {
                            "This dial is already one of your saved frequencies.".to_string()
                        } else if !can_save {
                            "This dial is already in the agreed list for this mode.".to_string()
                        } else {
                            format!(
                                "Remember this dial under {} and offer it here from now on. \
                                 Saved on the station, so every screen attached to this radio \
                                 has it.",
                                mode.label()
                            )
                        })
                        .clicked()
                    {
                        save = true;
                    }
                });
            });
        if let Some(r) = &resp {
            crate::chrome::paint_popup_cut_border(ui.ctx(), &r.response, 1.0);
        }
        if let Some(hz) = pick {
            cmds.push(Command::SetVfo { vfo: self.state.active_vfo, hz });
        }
        // The list is edited whole and latest-wins, exactly as the station's
        // other configuration is: the engine sorts, de-duplicates, saves it and
        // announces it back, so a second screen catches up on the next message
        // rather than holding a list of its own.
        if save || forget.is_some() {
            let mut presets = sdroxide_types::digi_presets().to_vec();
            if let Some(hz) = forget {
                presets.retain(|p| p.mode != mode || (p.dial_hz - hz).abs() >= 1.0);
            }
            if save {
                presets.push(sdroxide_types::DigiPreset {
                    mode,
                    dial_hz: dial,
                    note: String::new(),
                });
            }
            cmds.push(Command::SetDigiPresets(presets));
        }
    }

    /// A compact decode-squelch (sensitivity) slider for the keyboard panels:
    /// higher = require a stronger signal, so pure noise stops decoding.
    pub(in crate::app) fn digi_squelch_slider(
        &mut self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
    ) {
        let mut sq = self.digi_cfg_edit.digi_squelch;
        ui.spacing_mut().slider_width = 84.0;
        let resp =
            crate::chrome::slider(ui, egui::Slider::new(&mut sq, 0.0..=1.0).show_value(false))
                .on_hover_text("Decode squelch — raise to stop decoding noise");
        ui.label(RichText::new("SQL").size(10.0).color(crate::theme::CYAN_DIM()));
        if resp.changed() && self.digi_cfg_seeded {
            self.digi_cfg_edit.digi_squelch = sq;
            cmds.push(Command::SetDigiConfig(self.digi_cfg_edit.clone()));
        }
    }

    /// The clock the current mode keeps, or `None` where it keeps none.
    ///
    /// FT8, FT4, FT2 and WSPR have theirs fixed by the mode; JS8's slot length
    /// is an operator setting, so it has to come from the engine's status.
    pub(in crate::app) fn slot_timing(&self) -> Option<sdroxide_types::SlotTiming> {
        let mode = self.state.rx[0].mode;
        match mode {
            Mode::Js8 => Some(
                self.digi_status
                    .as_ref()
                    .and_then(|s| s.js8.as_ref())
                    .map_or(sdroxide_types::Js8Speed::default(), |j| j.speed)
                    .slot_timing(),
            ),
            _ => mode.slot_timing(),
        }
    }

    /// The slot length of the current mode, in seconds.
    ///
    /// The decode list groups rows into turns by this, and getting it wrong for
    /// JS8 Turbo would draw one "EVEN/ODD" header per two and a half turns. A
    /// mode with no slots at all answers with FT8's, because the callers are
    /// dividing decodes into turns and a zero there would divide by nothing.
    pub(in crate::app) fn slot_period_s(&self) -> f64 {
        self.slot_timing().map_or(15.0, |t| t.slot_s)
    }

    /// The slot clock, above the panel of every mode that has one.
    ///
    /// One bar in one place for FT8, FT4, FT2 and JS8: the turn is the unit all
    /// four are operated in — when the decoder speaks, when the sequencer may
    /// key — and it is worth the five pixels in every pane rather than only
    /// beside the one control that mentions it. WSPR draws its own inside the
    /// beacon's status card, where the countdown and the duty cycle it belongs
    /// with already are.
    ///
    /// Nothing at all in a mode with no slots: there is no turn to be part-way
    /// through, and a bar that crept across a PSK panel would be measuring
    /// something that does not exist.
    pub(in crate::app) fn slot_progress(&self, ui: &mut egui::Ui) {
        let Some(t) = self.slot_timing() else { return };
        // No engine anchor: only WSPR's status carries the slot it is working
        // on. A slot is a fixed block of UTC counted from the epoch, so this
        // window's own clock is the same answer whenever the two agree — and
        // when they do not, the DT readout in the QSO area is the thing that
        // says so.
        let into = widgets::slot_phase_s(crate::time::now_unix_f64(), t.slot_s, 0);
        let transmitting = self.digi_status.as_ref().map(|s| s.transmitting).unwrap_or(false);
        let state = if transmitting {
            widgets::SlotState::Transmitting
        } else {
            widgets::SlotState::Listening
        };
        widgets::slot_bar(ui, t, into, state).on_hover_text(format!(
            "This turn: {:.1} s of {:.0}. Decodes land at the end of it, and that is when the \
             next transmission may start.{}",
            into,
            t.slot_s,
            if transmitting {
                format!(
                    " The mark is where the burst stops, {:.1} s in.",
                    t.tx_offset_s + t.burst_s
                )
            } else {
                String::new()
            }
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The waterfall is the tab one past the mode's own panes — that is how
    /// `App::ui` tells "show the panadapter" from "show the panel", so a mode
    /// with no panes at all would make the two indistinguishable.
    #[test]
    fn every_mode_has_at_least_one_pane_and_a_waterfall_after_it() {
        for mode in Mode::ALL {
            let panes = panel_panes(mode);
            assert!(!panes.is_empty(), "{mode:?} has no panes");
            let tabs: Vec<_> = panel_tabs(mode).collect();
            assert_eq!(tabs.len(), panes.len() + 1, "{mode:?} tab count");
            assert_eq!(tabs.last().copied(), Some(TAB_WFALL), "{mode:?} waterfall is not last");
            assert!(!panes.contains(&TAB_WFALL), "{mode:?} names a pane after the waterfall tab");
        }
    }

    /// A stored pane index only means anything against the mode it was stored
    /// in. Clamping it to the pane count is what stops a switch from FT8 (two
    /// panes) to PSK (one) leaving the panel pointing past the end — the value
    /// the clamp yields is always either a real pane or the waterfall.
    #[test]
    fn a_stored_pane_index_is_valid_in_every_other_mode() {
        for from in Mode::ALL {
            // The furthest tab the mode it was stored in could select.
            let stored = panel_panes(from).len();
            for to in Mode::ALL {
                let panes = panel_panes(to).len();
                let clamped = stored.min(panes);
                assert!(clamped <= panes, "{from:?} → {to:?}: {clamped} past {panes}");
            }
        }
    }

    #[test]
    fn the_slotted_modes_can_find_their_qso_pane() {
        for mode in [Mode::Ft8, Mode::Ft4, Mode::Ft2] {
            let i = pane_index(mode, "QSO");
            assert_eq!(panel_panes(mode)[i], "QSO", "{mode:?}");
            assert_ne!(i, pane_index(mode, "DECODES"), "{mode:?} panes collapsed");
        }
        // A mode with no such pane falls back to its first, rather than to an
        // index that is not there.
        assert_eq!(pane_index(Mode::Psk, "QSO"), 0);
    }
}
