//! The EU 868 MHz channel plan the decoder listens on, and which of those
//! channels a given receiver window can actually reach.
//!
//! # Why fixed channels rather than a wideband burst detector
//!
//! Every protocol in scope transmits on a channel fixed by ETSI EN 300 220 and
//! by the manufacturer's choice within it. Detecting a burst anywhere in the
//! band and then going back for the samples means keeping a wideband IQ history
//! long enough to re-extract from — at 2 Msps that is 16 MB a second — and the
//! answer it produces is a channel that was in this table all along. Parking a
//! decoder on each channel costs one [`sdroxide_dsp::Ddc`] per channel and
//! nothing else, and it cannot miss the start of a burst because it was already
//! listening.
//!
//! # Channel rates
//!
//! Each channel's rate is set by the widest protocol on it, not by the
//! narrowest: the same baseband feeds every decoder that shares the channel, so
//! it has to hold the largest occupied bandwidth of the group. Rates are
//! *targets* — [`sdroxide_dsp::Ddc`] snaps to an integer decimation of whatever
//! the window rate turns out to be.

use sdroxide_dsp::Ddc;
use sdroxide_types::IsmFamily;

/// One channel the decoder can park on.
pub struct Channel {
    /// Centre, in absolute RF Hz.
    pub center_hz: f64,
    /// Target baseband rate, Hz. Set by the widest protocol on the channel.
    pub rate_hz: f64,
    /// What sits here, for the panel.
    pub label: &'static str,
    /// Families with at least one protocol on this channel. A channel none of
    /// the enabled families needs is not opened at all.
    pub families: &'static [IsmFamily],
}

/// The channels, ascending.
///
/// `868.30` and `868.95` are the two that matter most in Europe: the first
/// carries nearly every sensor and the wireless M-Bus S mode, the second carries
/// M-Bus T and C, which is what most modern meters use.
pub const CHANNELS: &[Channel] = &[
    Channel {
        center_hz: 868_300_000.0,
        // Widest here is wM-Bus mode S: ±50 kHz deviation at 32.768 kcps, so
        // about 150 kHz occupied. 250 kHz leaves room for the sensors' own
        // crystal error, which on a battery-powered thermometer is not small.
        rate_hz: 250_000.0,
        label: "sensors, wM-Bus S, KNX-RF, EnOcean, Homematic",
        families: &[IsmFamily::Weather, IsmFamily::Meter, IsmFamily::HomeAuto],
    },
    Channel {
        center_hz: 868_420_000.0,
        // Z-Wave's slowest rate — the only one decoded here — is 19 200 chips a
        // second at about ±20 kHz, so it occupies well under 100 kHz.
        //
        // This was 400 kHz, sized for Z-Wave's 100 kbps rate at ±58 kHz, and that
        // width made the 9.6 kbit/s traffic *undecodable*: deviation and symbol
        // rate are measured across the whole channel, so a wideband neighbour
        // three hundred kilohertz away dominated both. The same bursts that read
        // 22.9 kHz deviation and 19 191 baud through a 150 kHz window read 160 to
        // 191 kHz and no measurable rate through a 400 kHz one.
        //
        // So the channel is as narrow as the traffic that is actually read here.
        // Adding the faster Z-Wave rates later means a second, wider channel on
        // this centre rather than widening this one back — see the note on
        // `Channel::rate_hz`.
        rate_hz: 150_000.0,
        label: "Z-Wave EU",
        families: &[IsmFamily::HomeAuto],
    },
    Channel {
        center_hz: 868_950_000.0,
        // wM-Bus T and C: 100 kcps at ±50 kHz.
        rate_hz: 500_000.0,
        label: "wM-Bus T / C",
        families: &[IsmFamily::Meter],
    },
    Channel {
        center_hz: 869_525_000.0,
        rate_hz: 125_000.0,
        label: "wM-Bus N, Homematic long range",
        families: &[IsmFamily::Meter, IsmFamily::HomeAuto],
    },
];

/// Where the receiver would ideally be tuned: the centre that fits the whole
/// plan into the least bandwidth.
///
/// Not the midpoint of the channel centres — the midpoint of their *edges*, so
/// that the outermost channels' bandwidths fit too. Worth stating as code
/// rather than as a constant, because adding a channel to the table above has
/// to move it.
pub fn ideal_center_hz() -> f64 {
    let lo = CHANNELS.iter().map(|c| c.center_hz - c.rate_hz / 2.0).fold(f64::INFINITY, f64::min);
    let hi =
        CHANNELS.iter().map(|c| c.center_hz + c.rate_hz / 2.0).fold(f64::NEG_INFINITY, f64::max);
    (lo + hi) / 2.0
}

/// How much bandwidth the whole plan needs.
pub fn span_hz() -> f64 {
    let lo = CHANNELS.iter().map(|c| c.center_hz - c.rate_hz / 2.0).fold(f64::INFINITY, f64::min);
    let hi =
        CHANNELS.iter().map(|c| c.center_hz + c.rate_hz / 2.0).fold(f64::NEG_INFINITY, f64::max);
    hi - lo
}

/// Fraction of a receiver's sample rate that is actually usable.
///
/// Not a safety margin picked by feel. The RX-888's VHF path reaches 868 MHz
/// through its tuner and delivers the wideband downconverter's 2.025 Msps
/// output, whose band edges are deliberately tapered over the outermost eighth
/// at each end — see `EDGE_TAPER` in [`sdroxide_dsp::WbDdc`]. That leaves
/// exactly three quarters flat, and a channel placed in the taper is attenuated
/// by the very filter that selected it. Other front ends have an
/// anti-aliasing roll-off in roughly the same place, so the figure is right for
/// them too, if slightly conservative.
pub const USABLE_FRACTION: f64 = 0.75;

/// Whether `ch` fits inside a window of `window_rate_hz` centred on
/// `window_center_hz`, allowing for the unusable edges.
pub fn fits(ch: &Channel, window_center_hz: f64, window_rate_hz: f64) -> bool {
    let half_usable = window_rate_hz * USABLE_FRACTION / 2.0;
    let lo = window_center_hz - half_usable;
    let hi = window_center_hz + half_usable;
    ch.center_hz - ch.rate_hz / 2.0 >= lo && ch.center_hz + ch.rate_hz / 2.0 <= hi
}

/// Where to centre the decoder's window, given where the front end is tuned and
/// how much of the spectrum it hands over.
///
/// Ideally [`ideal_center_hz`], but the window has to stay inside the IQ the
/// receiver actually delivers: it is a decimation of that stream, not a second
/// tuner. So the ideal centre is clamped into what the hardware centre can
/// reach. On a receiver tuned well away from 868 MHz the result reaches no
/// channel at all, which is the honest outcome — [`fits`] then rejects
/// everything and the panel says so.
pub fn window_center_hz(hw_center_hz: f64, hw_rate_hz: f64, window_rate_hz: f64) -> f64 {
    window_center_for(ideal_center_hz(), hw_center_hz, hw_rate_hz, window_rate_hz)
}

/// As [`window_center_hz`], but aiming somewhere other than the native plan.
pub fn window_center_for(
    want_hz: f64,
    hw_center_hz: f64,
    hw_rate_hz: f64,
    window_rate_hz: f64,
) -> f64 {
    // The window is centred by an NCO offset inside the hardware span, so its
    // own edges must stay inside the usable part of that span.
    let slack = (hw_rate_hz * USABLE_FRACTION - window_rate_hz) / 2.0;
    if slack <= 0.0 {
        return hw_center_hz;
    }
    want_hz.clamp(hw_center_hz - slack, hw_center_hz + slack)
}

/// The narrowest window this front end can deliver that still covers `need_hz`:
/// what to ask a [`sdroxide_dsp::Ddc`] for, and what it will hand back.
///
/// [`sdroxide_dsp::Ddc`] decimates by an integer factor and picks it by
/// *rounding*, so a request can come back several per cent under: 1.365 MHz out
/// of a 3.84 Msps stream arrives as 1.28 MHz. For a *channel* that is
/// immaterial — the rate there is a target and the traffic is far narrower than
/// the channel — but for the window it is not, because [`fits`] has already
/// vetted the plan against the width that was asked for. A window built
/// narrower than the width it was chosen for can end up reaching none of the
/// lanes it was chosen to serve.
///
/// That is issue #310. With rtl_433's 868 MHz band on AUTO, a 1.365 MHz window
/// was chosen because it covered that band, built at 1.28 MHz, and then covered
/// neither the band nor two of the four native channels — having dragged the
/// window off the native plan to get there.
///
/// So the widths on offer are walked from narrow to wide, and each one is put
/// to `Ddc` rather than predicted, until one comes back wide enough. `None`
/// when even the undecimated stream is too narrow, which is the honest answer
/// and leaves the caller to say so.
pub(crate) fn width_at_least(in_rate_hz: f64, need_hz: f64) -> Option<(f64, f64)> {
    if !need_hz.is_finite() || need_hz <= 0.0 || need_hz > in_rate_hz {
        return None;
    }
    // Start at the deepest decimation that could conceivably be wide enough and
    // widen from there, so the answer is the cheapest one that works.
    let mut dec = (in_rate_hz / need_hz).floor().max(1.0);
    while dec >= 1.0 {
        let target = in_rate_hz / dec;
        let got = Ddc::rate_for(in_rate_hz, target);
        if got >= need_hz {
            return Some((target, got));
        }
        dec -= 1.0;
    }
    None
}

/// Where the decoder's one window should sit and how wide it should be.
pub struct WindowPlan {
    /// What to aim the window at, before the hardware span clamps it.
    pub want_center_hz: f64,
    /// The bandwidth to ask the down-converter for.
    pub target_rate_hz: f64,
    /// The bandwidth it will hand back for that request — see
    /// [`width_at_least`], which is why the two are separate numbers. This is
    /// the width the plan was scored against, so it is the one [`fits`] has to
    /// be asked about once the window exists.
    pub rate_hz: f64,
}

/// Whether anything switched on could decode a frame arriving on this channel.
///
/// Not the same question as "is the family switched on": 868.95 and 869.525 are
/// in the table so the panel can list them and the diagnostic tools can listen
/// there, but no protocol is implemented on either. A channel that cannot
/// produce a decode should not outvote a band that can when the window has to
/// choose — which is what it was doing.
fn decodes_here(cfg: &sdroxide_types::IsmSettings, ch: &Channel) -> bool {
    crate::proto::PROTOCOLS
        .iter()
        .any(|p| (p.channel_hz - ch.center_hz).abs() < 10_000.0 && cfg.protocol_enabled(p.id))
}

/// Choose the window from what is switched on.
///
/// There is one window and two lanes that may want different parts of the
/// spectrum — the native decoders are fixed on the European 868 MHz channels,
/// while rtl_433 can be pointed at any of five bands hundreds of megahertz
/// apart. A window aimed at 868 would never reach a 433 MHz band, so the aim
/// point cannot be a constant.
///
/// Each lane contributes the span it needs covered *whole*, and the candidates
/// are those spans plus, ahead of them, the span that holds the native plan and
/// one band **together**. On 868 MHz the two overlap, so a front end with room
/// for both should be asked for both rather than made to pick — which is the
/// other half of issue #310: a 3.84 Msps PlutoSDR has room for the whole native
/// plan *and* rtl_433's 868 MHz band, and was being handed a window sized to
/// the narrower of the two.
///
/// Candidates are ranked by what they would actually decode, then by cost:
///
/// 1. how many channels and bands that can produce a decode are covered,
/// 2. then the narrower window, so a channel nothing reads is never worth
///    doubling the down-converter's work for,
/// 3. then how much of the plan is in view at all, so the panel says "not
///    decoded yet" rather than "outside the receiver's window" when the two
///    cost the same,
/// 4. then candidate order.
///
/// Whatever is left uncovered is reported as such, with somewhere to tune —
/// which is the honest outcome when no front end spans both bands.
pub fn window_plan(
    cfg: &sdroxide_types::IsmSettings,
    hw_center_hz: f64,
    hw_rate_hz: f64,
) -> Option<WindowPlan> {
    // The span each switched-on lane needs inside the usable window, whole.
    let native = cfg.native_enabled().then(|| {
        let half = span_hz() / 2.0;
        (ideal_center_hz() - half, ideal_center_hz() + half)
    });

    #[allow(unused_mut)]
    let mut bands: Vec<(f64, f64)> = Vec::new();
    #[cfg(feature = "rtl433")]
    if cfg.rtl433_enabled() {
        for b in crate::rtl433::bands::all() {
            if cfg.rtl433.band_enabled(b.bit) {
                let r = crate::rtl433::bands::rate_for(b, &cfg.rtl433);
                bands.push((b.center_hz - r / 2.0, b.center_hz + r / 2.0));
            }
        }
    }

    let mut wants: Vec<(f64, f64)> = Vec::new();
    if let Some(n) = native {
        // Both lanes first, so a tie goes to the window that serves both.
        wants.extend(bands.iter().map(|b| (n.0.min(b.0), n.1.max(b.1))));
        wants.push(n);
    }
    wants.extend(bands.iter().copied());

    if wants.is_empty() {
        return None;
    }

    let score = |center: f64, rate: f64| -> (u32, u32) {
        let (mut decodable, mut in_view) = (0, 0);
        if cfg.native_enabled() {
            for ch in CHANNELS {
                if fits(ch, center, rate) {
                    in_view += 1;
                    decodable += u32::from(decodes_here(cfg, ch));
                }
            }
        }
        #[cfg(feature = "rtl433")]
        if cfg.rtl433_enabled() {
            for b in crate::rtl433::bands::all() {
                if cfg.rtl433.band_enabled(b.bit)
                    && crate::rtl433::bands::fits(b, &cfg.rtl433, center, rate)
                {
                    in_view += 1;
                    decodable += 1;
                }
            }
        }
        (decodable, in_view)
    };

    let mut best: Option<((u32, u32), WindowPlan)> = None;
    for (lo, hi) in wants {
        // A receiver too narrow for the span still gets the widest window it
        // has, so the lane runs and `fits` reports what is missing, rather than
        // the whole decoder falling back to a width nothing chose.
        let (target, rate) = width_at_least(hw_rate_hz, (hi - lo) / USABLE_FRACTION)
            .unwrap_or((hw_rate_hz, hw_rate_hz));
        let want_center_hz = (lo + hi) / 2.0;
        let center = window_center_for(want_center_hz, hw_center_hz, hw_rate_hz, rate);
        let (decodable, in_view) = score(center, rate);
        let better = match &best {
            None => true,
            // Decodes first, then the cheaper window, then coverage. Written
            // out rather than folded into one key because the middle term is
            // the only one that wants the *smaller* number.
            Some(((best_decodable, best_in_view), best_plan)) => {
                match decodable.cmp(best_decodable) {
                    std::cmp::Ordering::Greater => true,
                    std::cmp::Ordering::Less => false,
                    std::cmp::Ordering::Equal => match rate.total_cmp(&best_plan.rate_hz) {
                        std::cmp::Ordering::Less => true,
                        std::cmp::Ordering::Greater => false,
                        std::cmp::Ordering::Equal => in_view > *best_in_view,
                    },
                }
            }
        };
        if better {
            best = Some((
                (decodable, in_view),
                WindowPlan { want_center_hz, target_rate_hz: target, rate_hz: rate },
            ));
        }
    }
    best.map(|(_, p)| p)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The RX-888 reaching 868 MHz through its tuner, which is the receiver
    /// this was developed against: the wideband downconverter's output rate.
    const RX888_VHF_RATE: f64 = 2_025_000.0;

    #[test]
    fn the_channels_are_ascending_and_have_sane_rates() {
        for w in CHANNELS.windows(2) {
            assert!(w[0].center_hz < w[1].center_hz, "channel table is not sorted");
        }
        for c in CHANNELS {
            assert!(c.rate_hz >= 100_000.0 && c.rate_hz <= 1_000_000.0, "{} Hz", c.rate_hz);
            assert!(!c.families.is_empty(), "a channel nothing listens on");
        }
    }

    /// The plan has to fit what the RX-888's VHF path can deliver, or the
    /// receiver this was written for reaches only part of its own channel
    /// table. This is the constraint that set the rates above.
    #[test]
    fn the_whole_plan_fits_the_rx888_vhf_window() {
        let usable = RX888_VHF_RATE * USABLE_FRACTION;
        assert!(
            span_hz() <= usable,
            "the plan needs {:.0} kHz but only {:.0} kHz is usable",
            span_hz() / 1e3,
            usable / 1e3
        );

        // And with the window placed where it belongs, every channel is in.
        let center = ideal_center_hz();
        for c in CHANNELS {
            assert!(fits(c, center, RX888_VHF_RATE), "{:.3} MHz missed", c.center_hz / 1e6);
        }
    }

    /// On a receiver whose whole IQ stream is barely wider than the plan there is
    /// no room to slide a window inside it, so the window *is* the stream and the
    /// operator has to tune to the right place. Saying so is the point: the panel
    /// turns this into "tune to 868.88 MHz" rather than "no devices found".
    #[test]
    fn a_receiver_with_no_slack_pins_the_window_to_its_own_centre() {
        // The window can be no narrower than the plan, so on the RX-888 the Ddc
        // decimates by one and the window rate is the hardware rate.
        let c = window_center_hz(868_900_000.0, RX888_VHF_RATE, RX888_VHF_RATE);
        assert_eq!(c, 868_900_000.0, "there is no slack to slide into");

        // Tuned close enough, the plan still fits — that is what matters.
        for ch in CHANNELS {
            assert!(fits(ch, c, RX888_VHF_RATE), "{:.3} MHz missed", ch.center_hz / 1e6);
        }

        // Tuned 2 MHz low, it cannot: the channels are simply not in the stream.
        let c = window_center_hz(866_900_000.0, RX888_VHF_RATE, RX888_VHF_RATE);
        assert_eq!(c, 866_900_000.0);
        assert!(
            !CHANNELS.iter().any(|ch| fits(ch, c, RX888_VHF_RATE)),
            "a receiver 2 MHz away should reach none of the plan"
        );
    }

    /// A front end with room to spare puts the window where the plan wants it,
    /// wherever the operator has tuned within reach.
    #[test]
    fn a_wide_front_end_slides_the_window_onto_the_plan() {
        let c = window_center_hz(866_900_000.0, 8_000_000.0, 2_000_000.0);
        assert!((c - ideal_center_hz()).abs() < 1.0, "got {c}, wanted the ideal centre");
        for ch in CHANNELS {
            assert!(fits(ch, c, 2_000_000.0), "{:.3} MHz missed", ch.center_hz / 1e6);
        }
    }

    /// A wide front end can hold the plan wherever it is tuned within reach,
    /// and the clamp must not push the window outside the hardware span.
    #[test]
    fn a_wide_front_end_keeps_the_window_inside_its_own_span() {
        let hw_rate = 8_000_000.0;
        for hw_center in [866_000_000.0, 868_900_000.0, 871_000_000.0] {
            let w = window_center_hz(hw_center, hw_rate, 2_000_000.0);
            let slack = (hw_rate * USABLE_FRACTION - 2_000_000.0) / 2.0;
            assert!(
                (w - hw_center).abs() <= slack + 1.0,
                "window {w} is {} Hz off a centre with only {slack} Hz of slack",
                (w - hw_center).abs()
            );
        }
    }

    /// A narrow receiver — an RTL-SDR at 2.4 Msps has 1.8 MHz usable, which is
    /// enough; at 1 Msps it is not, and `fits` has to say so rather than let a
    /// channel be decoded through the anti-alias roll-off.
    #[test]
    fn a_window_narrower_than_a_channel_admits_nothing() {
        let ch = &CHANNELS[2]; // 868.95 MHz at 500 kHz
        assert!(!fits(ch, ch.center_hz, 500_000.0), "500 kHz has only 375 kHz usable");
        assert!(fits(ch, ch.center_hz, 700_000.0));
    }

    /// The PlutoSDR of issue #310: 3.840 Msps with its LO parked where the
    /// operator left it.
    const PLUTO_RATE: f64 = 3_840_000.0;
    const PLUTO_CENTER: f64 = 868_957_858.0;

    fn all_on() -> sdroxide_types::IsmSettings {
        sdroxide_types::IsmSettings {
            enabled: true,
            threshold_db: 12,
            families: u32::MAX,
            max_devices: 200,
            rtl433: sdroxide_types::Rtl433Settings::OFF,
        }
    }

    /// The width a window is built at has to be the width it was chosen for.
    ///
    /// [`sdroxide_dsp::Ddc`] rounds its decimation to the *nearest* integer, so
    /// a plain `Ddc::new(in, want)` can come back several per cent narrower than
    /// `want` — which for a window means one chosen because it covered a band
    /// no longer covering it. This is the arithmetic that must not regress.
    #[test]
    fn a_window_is_never_narrower_than_the_width_it_was_asked_for() {
        for &in_rate in
            &[2_025_000.0, 2_400_000.0, 3_840_000.0, 8_000_000.0, 10_000_000.0, 32_400_000.0]
        {
            let mut need = 100_000.0;
            while need <= in_rate {
                let (target, got) = width_at_least(in_rate, need).expect("need is inside the span");
                assert!(got >= need, "{in_rate} Hz gave {got} for a need of {need}");
                assert_eq!(
                    Ddc::new(in_rate, target).out_rate(),
                    got,
                    "in {in_rate}, need {need}: the width promised is not the one built"
                );
                need *= 1.07;
            }
        }
        // And it says so rather than guessing when the stream is simply too
        // narrow for what was asked.
        assert_eq!(width_at_least(1_000_000.0, 2_000_000.0), None);
    }

    /// Issue #310, the half that made the ISM panel worse the more of it was
    /// switched on: with rtl_433's 868 MHz band on AUTO, the window was sized
    /// 1.365 MHz because that covered the band, built at 1.28 MHz because a
    /// `Ddc` rounds its decimation, and then covered neither that band nor two
    /// of the four native channels — having pulled the window off the native
    /// plan to get there.
    #[cfg(feature = "rtl433")]
    #[test]
    fn the_window_covers_the_band_it_was_chosen_for() {
        let mut cfg = all_on();
        cfg.rtl433.bands = crate::rtl433::bands::BANDS[1].bit; // 868 MHz EU
        cfg.rtl433.bandwidth_hz = sdroxide_types::RTL433_BANDWIDTH_AUTO;

        let p = window_plan(&cfg, PLUTO_CENTER, PLUTO_RATE).expect("a plan");
        assert_eq!(
            Ddc::new(PLUTO_RATE, p.target_rate_hz).out_rate(),
            p.rate_hz,
            "the plan was scored against a window that will not be built"
        );
        let center = window_center_for(p.want_center_hz, PLUTO_CENTER, PLUTO_RATE, p.rate_hz);
        assert!(
            crate::rtl433::bands::fits(
                &crate::rtl433::bands::BANDS[1],
                &cfg.rtl433,
                center,
                p.rate_hz
            ),
            "the window was chosen for the 868 MHz band and does not reach it"
        );
    }

    /// The other half of #310: a 3.840 Msps front end has room for the whole
    /// native plan *and* rtl_433's 868 MHz band, so it should be given a window
    /// that holds both rather than one sized to the narrower of the two.
    ///
    /// The operator's own settings from the issue: every family on, rtl_433 on
    /// the 868 MHz band, bandwidth chip at 2048 kHz.
    #[cfg(feature = "rtl433")]
    #[test]
    fn a_front_end_with_room_for_both_lanes_is_given_both() {
        let mut cfg = all_on();
        cfg.rtl433.bands = crate::rtl433::bands::BANDS[1].bit;
        cfg.rtl433.bandwidth_hz = 2_048_000;

        let p = window_plan(&cfg, PLUTO_CENTER, PLUTO_RATE).expect("a plan");
        let center = window_center_for(p.want_center_hz, PLUTO_CENTER, PLUTO_RATE, p.rate_hz);
        for ch in CHANNELS {
            assert!(
                fits(ch, center, p.rate_hz),
                "{:.3} MHz is outside a window this receiver had room for",
                ch.center_hz / 1e6
            );
        }
        assert!(
            crate::rtl433::bands::pick(&cfg.rtl433, center, p.rate_hz).is_some(),
            "rtl_433 got no band out of a window wide enough for one"
        );
        // And the band it gets is fed at least the width it asked for — 960 kHz
        // for a 1024 kHz band is both narrower than the band and under the
        // 1.2 Msps rtl_433's own wM-Bus mode C&T decoder wants.
        let want = crate::rtl433::bands::rate_for(&crate::rtl433::bands::BANDS[1], &cfg.rtl433);
        let (target, got) = width_at_least(p.rate_hz, want).expect("the band fits the window");
        assert!(got >= want);
        assert_eq!(Ddc::new(p.rate_hz, target).out_rate(), got);
    }

    /// A channel nothing can decode must not outvote a band that can.
    ///
    /// 868.95 and 869.525 are in the table so the panel can list them and the
    /// diagnostic tools can listen there; no protocol is implemented on either.
    /// Scoring a window by how many *channels* it covers therefore bought empty
    /// rows with the window's own placement.
    #[cfg(feature = "rtl433")]
    #[test]
    fn a_channel_that_decodes_nothing_does_not_outvote_a_band_that_does() {
        // A receiver with only the metering families on has no native decoder
        // at all, so the window belongs to rtl_433 whatever the channel plan
        // would have preferred.
        let mut cfg = all_on();
        cfg.families = sdroxide_types::IsmFamily::Meter.bit();
        cfg.rtl433.bands = crate::rtl433::bands::BANDS[1].bit;
        cfg.rtl433.bandwidth_hz = sdroxide_types::RTL433_BANDWIDTH_AUTO;

        let p = window_plan(&cfg, PLUTO_CENTER, PLUTO_RATE).expect("a plan");
        let center = window_center_for(p.want_center_hz, PLUTO_CENTER, PLUTO_RATE, p.rate_hz);
        assert!(
            crate::rtl433::bands::pick(&cfg.rtl433, center, p.rate_hz).is_some(),
            "the only lane that can decode anything here did not get the window"
        );
    }

    /// Two lanes that cannot share a window still get the better of the two,
    /// and the window is not stretched across 435 MHz trying to hold both.
    #[cfg(feature = "rtl433")]
    #[test]
    fn lanes_on_different_bands_do_not_produce_an_impossible_window() {
        let mut cfg = all_on();
        cfg.rtl433.bands = crate::rtl433::bands::BANDS[0].bit; // 433.92 MHz
        cfg.rtl433.bandwidth_hz = sdroxide_types::RTL433_BANDWIDTH_AUTO;

        let p = window_plan(&cfg, PLUTO_CENTER, PLUTO_RATE).expect("a plan");
        assert!(p.rate_hz <= PLUTO_RATE);
        let center = window_center_for(p.want_center_hz, PLUTO_CENTER, PLUTO_RATE, p.rate_hz);
        assert!(
            CHANNELS.iter().any(|ch| fits(ch, center, p.rate_hz)),
            "the reachable lane should have got the window"
        );
    }

    /// The native lane on its own is unchanged by any of the above: the plan's
    /// own span, placed on the plan, on the receiver it was written for.
    #[test]
    fn the_native_lane_alone_still_gets_the_plan_it_always_had() {
        let cfg = all_on();
        for &(hw_center, hw_rate) in &[(PLUTO_CENTER, PLUTO_RATE), (868_900_000.0, RX888_VHF_RATE)]
        {
            let p = window_plan(&cfg, hw_center, hw_rate).expect("a plan");
            assert_eq!(Ddc::new(hw_rate, p.target_rate_hz).out_rate(), p.rate_hz);
            let center = window_center_for(p.want_center_hz, hw_center, hw_rate, p.rate_hz);
            for ch in CHANNELS {
                assert!(
                    fits(ch, center, p.rate_hz),
                    "{:.3} MHz missed at {:.3} MHz / {:.3} Msps",
                    ch.center_hz / 1e6,
                    hw_center / 1e6,
                    hw_rate / 1e6
                );
            }
        }
    }
}
