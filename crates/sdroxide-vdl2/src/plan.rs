//! The channel plan, and which of it a given receiver window can reach.
//!
//! # Why fixed channels rather than a wideband burst detector
//!
//! Every VDL2 transmission in the world is on one of fourteen frequencies.
//! Watching
//! the whole 300 kHz for a burst and then going back for the samples means
//! keeping a wideband history long enough to re-extract from, and the answer it
//! produces is a channel that was in this table all along. Parking a
//! downconverter on each channel costs one [`sdroxide_dsp::Ddc`] apiece and
//! nothing else, and it cannot miss the start of a transmission because it was
//! already listening — which for a burst whose first sixteen symbols are the
//! only synchronisation there is, is the whole game.
//!
//! # One window, fourteen channels
//!
//! The plan spans 350 kHz including the outer channels' own bandwidth, so a
//! single half-megahertz window covers the lot on any receiver in this tree —
//! an RTL-SDR at 2.4 Msps lands on a 480 kHz window and clears the outermost
//! channel by 5 kHz. A narrower one reaches fewer, and which ones it reaches
//! depends on where the hardware centre happens to be — so the window slides
//! inside the front end's span to take in as much of the plan as it can, and the
//! panel is told what it got.
//!
//! # Neighbours, and what keeps them out
//!
//! The channels are the 25 kHz raster itself, not every other slot of it, so two
//! that are both in use can be adjacent — 136.900 and 136.925 are, and one of
//! them is somebody's local airport. Nothing in front of the demodulator
//! separates them: each channel's downconverter is designed against its own
//! output rate, which is ten times a channel wide. Two things do the separating.
//! [`crate::demod::ChannelFilter`] keeps a neighbour out of the *decision*, and
//! [`crate::gate::Gate`]'s own detection filter keeps it from opening the gate
//! in the first place — without which a strong station 25 kHz away is counted as
//! a burst on a channel it was never on.
//!
//! Source: the European ICAO Frequency Management Manual (EUR Doc 011) for
//! 136.700–136.975 and the air/airport split below; the Aeronautical Frequency
//! Committee's USA plan for 136.650. 136.975 is the worldwide Common Signalling
//! Channel.

use sdroxide_types::{
    VDL2_CHANNEL_LABELS, VDL2_CHANNEL_SPACING_HZ, VDL2_CHANNELS_HZ, VDL2_CSC_HZ,
    VDL2_PLAN_CENTER_HZ,
};

/// One channel of the plan.
pub struct Channel {
    pub center_hz: f64,
    /// What the assignment is for — see [`VDL2_CHANNEL_LABELS`], which the
    /// panel reads too.
    pub label: &'static str,
    /// The Common Signalling Channel: in use worldwide, and where every link
    /// starts.
    pub csc: bool,
}

/// The channels, ascending.
pub const CHANNELS: [Channel; 14] = {
    // Built from the two tables rather than written out a third time: a channel
    // whose label belonged to its neighbour is exactly the sort of mistake this
    // module exists to have stopped making.
    const BLANK: Channel = Channel { center_hz: 0.0, label: "", csc: false };
    let mut out = [BLANK; 14];
    let mut i = 0;
    while i < VDL2_CHANNELS_HZ.len() {
        out[i] = Channel {
            center_hz: VDL2_CHANNELS_HZ[i],
            label: VDL2_CHANNEL_LABELS[i],
            csc: VDL2_CHANNELS_HZ[i] == VDL2_CSC_HZ,
        };
        i += 1;
    }
    out
};

/// Fraction of a front end's span the window may claim.
///
/// The outer edges of any receiver's window are where its own anti-alias filter
/// is already rolling off, and a channel sitting in the roll-off decodes badly
/// or not at all. The same three quarters the ISM and ADS-B lanes use, for the
/// same reason.
pub const USABLE_FRACTION: f64 = 0.75;

/// What the lane asks its window downconverter for.
///
/// Not simply the plan's span divided by [`USABLE_FRACTION`]: a
/// [`sdroxide_dsp::Ddc`] decimates by a whole number and rounds to the *nearest*
/// one, so a target sitting exactly on the requirement can round the wrong way
/// and land under it. On an RTL-SDR at 2.4 Msps that is the difference between a
/// 480 kHz window that holds all fourteen channels and a 400 kHz one that holds
/// nine. Half a megahertz leaves room for the rounding on every front end here.
pub const WINDOW_TARGET_RATE_HZ: f64 = 500_000.0;

/// What each channel's own downconverter asks for.
///
/// Ten samples a symbol nominally; an integer decimation lands somewhere near
/// it, between about nine and twelve on the front ends in this tree. The floor
/// that matters is not the symbol timing — that is interpolated — but the rest
/// of the plan, which at this rate is partly inside the downconverter's own
/// Nyquist band and partly folded back into it. Neither is fatal on its own; a
/// channel folding *on top of the signal* while still inside the decimator's
/// passband would be, and
/// [`tests::no_channel_of_the_plan_reaches_another_channels_decision`] is what
/// says none does.
pub const CHANNEL_TARGET_RATE_HZ: f64 = 105_000.0;

/// Distance from the plan's outermost channel centres to its edges.
pub fn span_hz() -> f64 {
    let lo = CHANNELS[0].center_hz - VDL2_CHANNEL_SPACING_HZ / 2.0;
    let hi = CHANNELS[CHANNELS.len() - 1].center_hz + VDL2_CHANNEL_SPACING_HZ / 2.0;
    hi - lo
}

/// Where the window wants to sit to reach the whole plan.
pub fn ideal_center_hz() -> f64 {
    VDL2_PLAN_CENTER_HZ
}

/// Whether a channel is inside a window, with its own bandwidth and the front
/// end's roll-off allowed for.
pub fn fits(center_hz: f64, window_center_hz: f64, window_rate_hz: f64) -> bool {
    let half = window_rate_hz * USABLE_FRACTION / 2.0;
    (center_hz - window_center_hz).abs() + VDL2_CHANNEL_SPACING_HZ / 2.0 <= half
}

/// Indices of the channels a window reaches, ascending.
pub fn channels_in_window(window_center_hz: f64, window_rate_hz: f64) -> Vec<usize> {
    (0..CHANNELS.len())
        .filter(|&i| fits(CHANNELS[i].center_hz, window_center_hz, window_rate_hz))
        .collect()
}

/// Where to put a window of `window_rate_hz` inside a front end's span.
///
/// The ideal centre where the span reaches it, and as close as the span allows
/// otherwise — a receiver that cannot hold the whole plan should still hold as
/// much of it as it can rather than refusing.
pub fn window_center_for(hw_center_hz: f64, hw_rate_hz: f64, window_rate_hz: f64) -> f64 {
    let slack = (hw_rate_hz * USABLE_FRACTION - window_rate_hz) / 2.0;
    if slack <= 0.0 {
        return hw_center_hz;
    }
    ideal_center_hz().clamp(hw_center_hz - slack, hw_center_hz + slack)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The plan's span and its ideal centre are derived from the table, so
    /// adding a channel moves them rather than leaving them stale.
    #[test]
    fn the_span_and_centre_come_from_the_table() {
        assert_eq!(span_hz(), 350_000.0);
        let lo = CHANNELS[0].center_hz - VDL2_CHANNEL_SPACING_HZ / 2.0;
        assert_eq!(ideal_center_hz(), lo + span_hz() / 2.0);
        assert_eq!(CHANNELS.iter().filter(|c| c.csc).count(), 1);
    }

    /// A window at the target rate holds the whole plan — which is the thing
    /// that rate exists to guarantee.
    #[test]
    fn the_target_rate_holds_the_whole_plan() {
        let got = channels_in_window(ideal_center_hz(), WINDOW_TARGET_RATE_HZ);
        assert_eq!(got.len(), CHANNELS.len(), "only {got:?} fit");
    }

    /// ...and so does the rate every front end in this tree actually lands on.
    ///
    /// The downconverter rounds its decimation to the nearest whole number, so
    /// the achievable rates are a ladder rather than a dial; this is the ladder,
    /// and every rung above 467 kHz has to hold all fourteen. The 2.4 Msps rung
    /// is the tight one: a 480 kHz window clears the outermost channel by
    /// 5 kHz, and it is the rate the commonest receiver in the world runs at.
    #[test]
    fn the_rates_front_ends_land_on_hold_the_plan() {
        for &in_rate in &[2_400_000.0f64, 2_048_000.0, 2_500_000.0, 2_025_000.0, 8_000_000.0] {
            let rate = sdroxide_dsp::Ddc::rate_for(in_rate, WINDOW_TARGET_RATE_HZ);
            let got = channels_in_window(ideal_center_hz(), rate);
            assert_eq!(
                got.len(),
                CHANNELS.len(),
                "{in_rate} gives a {rate} Hz window holding only {got:?}"
            );
        }
    }

    /// A narrow window keeps what it can reach and says so, rather than
    /// refusing outright. An RTL-SDR at its slowest still hears nine channels.
    #[test]
    fn a_narrow_window_keeps_what_it_reaches() {
        let got = channels_in_window(ideal_center_hz(), 250_000.0);
        assert!(!got.is_empty() && got.len() < CHANNELS.len(), "{got:?}");
        // ...and one narrow enough for a single channel gets the one it is on.
        let got = channels_in_window(VDL2_CSC_HZ, 40_000.0);
        assert_eq!(got, vec![CHANNELS.len() - 1]);
    }

    /// The window slides inside the front end's span to reach the plan, and
    /// stops at the edge rather than asking for samples that are not there.
    #[test]
    fn the_window_slides_towards_the_plan_but_not_past_the_span() {
        // A wide front end centred elsewhere: the window reaches the plan.
        let c = window_center_for(137_500_000.0, 8_000_000.0, 500_000.0);
        assert_eq!(c, ideal_center_hz());
        // A narrow one centred well away: the window goes as far as it can.
        let c = window_center_for(137_500_000.0, 1_000_000.0, 500_000.0);
        assert!(c > 137_300_000.0 && c < 137_500_000.0, "{c}");
        // No room at all: it stays on the hardware centre and reaches nothing.
        let c = window_center_for(137_500_000.0, 500_000.0, 500_000.0);
        assert_eq!(c, 137_500_000.0);
        assert!(channels_in_window(c, 500_000.0).is_empty());
    }

    /// Nothing anywhere else in the plan arrives at a channel's decision.
    ///
    /// The one kind of interference no later stage can undo is another channel
    /// landing *on top of* this one, and at a channel rate near 105 kHz there
    /// are two ways for that to happen: a neighbour close enough to sit inside
    /// the downconverter's passband, or a distant one folding back into it.
    /// Rather than reason about where each lands, this puts a tone at every
    /// offset in the plan through the very chain the worker builds — the
    /// per-channel [`sdroxide_dsp::Ddc`] and then
    /// [`crate::demod::ChannelFilter`] — and measures what comes out against
    /// the same tone on channel.
    ///
    /// 60 dB is the figure a real receiver has to meet, and it is what stops a
    /// ground station a mile away on 136.925 being decoded as traffic on
    /// 136.900.
    #[test]
    fn no_channel_of_the_plan_reaches_another_channels_decision() {
        for &window in &[480_000.0f64, 500_000.0, 506_250.0, 512_000.0, 250_000.0] {
            let rate = sdroxide_dsp::Ddc::rate_for(window, CHANNEL_TARGET_RATE_HZ);
            let sps = rate / crate::demod::SYMBOL_RATE;
            assert!((3.0..20.0).contains(&sps), "{window} Hz gives {sps} samples per symbol");

            // Only offsets two channels of the plan can actually have while
            // both are inside the same window: anything further apart than that
            // was removed by the window's own downconverter, which is a
            // different filter's job and a different test's.
            let max_sep = window * USABLE_FRACTION - VDL2_CHANNEL_SPACING_HZ;
            let on_channel = through_one_channel(window, 0.0);
            for k in 1..CHANNELS.len() {
                let offset = k as f64 * VDL2_CHANNEL_SPACING_HZ;
                if offset > max_sep {
                    break;
                }
                let db = 20.0 * (through_one_channel(window, offset) / on_channel).log10();
                assert!(
                    db < -60.0,
                    "a {window} Hz window gives {rate} Hz channels, and a station \
                     {offset} Hz away arrives {db:.0} dB down"
                );
            }
        }
    }

    /// Level of a unit tone `offset_hz` from a channel's centre, after that
    /// channel's downconverter and receive filter. Amplitude, not power.
    fn through_one_channel(window_rate_hz: f64, offset_hz: f64) -> f64 {
        use sdroxide_dsp::Complex32;

        let mut ddc = sdroxide_dsp::Ddc::new(window_rate_hz, CHANNEL_TARGET_RATE_HZ);
        ddc.set_offset_hz(0.0);
        let n = (window_rate_hz * 0.05) as usize;
        let w = std::f64::consts::TAU * offset_hz / window_rate_hz;
        let tone: Vec<Complex32> = (0..n)
            .map(|i| {
                let p = w * i as f64;
                Complex32::new(p.cos() as f32, p.sin() as f32)
            })
            .collect();
        let mut base = Vec::new();
        ddc.process(&tone, &mut base);

        // The same filter `ChannelRx` builds, on the rate the downconverter
        // settled on.
        let sps = ddc.out_rate() / crate::demod::SYMBOL_RATE;
        let rrc = crate::demod::ChannelFilter::new(sps, 8, 16);
        let mut out = Vec::new();
        rrc.run(&base, &mut out);
        // Past both filters' run-up, and past the first thousandth of a second
        // of the downconverter's own.
        let skip = (out.len() / 4).max(rrc.half() + 1);
        let tail = &out[skip..out.len() - rrc.half() - 1];
        (tail.iter().map(|z| f64::from(z.norm_sqr())).sum::<f64>() / tail.len() as f64).sqrt()
    }
}
