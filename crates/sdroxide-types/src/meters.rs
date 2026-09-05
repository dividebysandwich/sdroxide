use serde::{Deserialize, Serialize};

/// Fraction of converter samples at full scale above which the front end is
/// called overloaded — see [`Meters::adc_clip`].
///
/// Not zero, because "at full scale" is measured with a shade of margin: the
/// backends disagree on what the top code converts to (0.9922 on a packed 8-bit
/// front end, 0.99688 on an RTL-SDR, 0.99997 on a 16-bit one), so the test has
/// to sit under all of them and a signal that legitimately fills the converter
/// then grazes it on the odd sample. One in two hundred is well clear of that
/// and far below what any genuinely clipped signal produces — the mildest
/// clipped case measured for issue #173 was already at 43 %.
///
/// Lives here rather than beside the meter that fills it because the UI asks
/// this question too, and the UI builds for wasm32 where the DSP crate does not
/// follow.
pub const OVERLOAD_FRACTION: f32 = 0.005;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TxMeters {
    /// Forward power in watts, if the device exposes a sensor for it.
    pub fwd_w: Option<f32>,
    pub swr: Option<f32>,
    /// 0.0..=1.0 modulation drive level.
    pub alc: f32,
    /// The rig's own power-output meter as a `0.0..=1.0` fraction of full
    /// scale, if it has one. Deliberately not watts: see [`TxTelemetry::po`].
    pub po: Option<f32>,
}

/// TX-side telemetry a rig reports out-of-band (CAT / TCI): forward power,
/// SWR, the rig's own ALC and its power-output meter. Distinct from
/// [`TxMeters`], which also carries the engine's own ALC — this is only what
/// the *device* measures, merged into `TxMeters` by the engine while
/// transmitting.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct TxTelemetry {
    /// Forward power in watts, if the device exposes a sensor for it.
    pub fwd_w: Option<f32>,
    /// SWR as a ratio (e.g. `1.4` = 1.4:1), if the device measures it.
    pub swr: Option<f32>,
    /// ALC as `0.0..=1.0` of the rig's own meter, if it reports one.
    ///
    /// This is the rig saying how hard its automatic level control is working,
    /// which is the number that says whether the audio being fed to it is too
    /// hot. Nothing on this side can compute it: [`TxMeters::alc`] is what
    /// SDRoxide SENDS, and this is what the rig does about it. On a CAT rig the
    /// two are different measurements and only this one is the operator's
    /// answer to "am I overdriving it".
    pub alc: Option<f32>,
    /// The rig's power-output meter as a `0.0..=1.0` fraction of full scale.
    ///
    /// Kept separate from `fwd_w`, and deliberately not converted into it,
    /// because the two are different claims. `fwd_w` is watts a device has
    /// actually measured; this is a needle position. An Icom answers its PO
    /// meter as a raw `0..255` with published breakpoints for the *scale* but
    /// no calibrated wattage behind them, so turning it into watts would
    /// invent a precision the rig never offered — the same reasoning the ALC
    /// reading is reported as a percentage rather than in dB.
    ///
    /// A device that genuinely measures forward power still fills `fwd_w`, and
    /// the two can be present together on a rig that reports both.
    pub po: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Meters {
    /// Signal level in the RX passband, dBm (after `cal_offset_db`).
    pub s_dbm: f32,
    /// ADC headroom indicator: the highest either converter axis reached over
    /// the last meter window, in dBFS. `f32::NEG_INFINITY` before anything has
    /// been measured.
    pub adc_peak_dbfs: f32,
    /// The radio's own temperature in degrees Celsius, where it measures one.
    ///
    /// The board's sensor, not a figure derived on this side, and reported in
    /// receive as well as transmit: what a small PA does is heat up over an
    /// afternoon and cool slowly afterwards, and both halves of that are what
    /// the operator is watching (issue #333). `None` on the great majority of
    /// radios, which have no such sensor — see `IqSource::pa_temp_c`.
    #[serde(default)]
    pub pa_temp_c: Option<f32>,
    /// Fraction of converter samples at full scale over the same window,
    /// `0.0..=1.0`. Above [`OVERLOAD_FRACTION`] — ask [`Meters::adc_overloaded`] —
    /// the front end is
    /// running into its rails and everything downstream is reading a distorted
    /// signal — including the `s_dbm` beside this, which understates a clipped
    /// carrier.
    ///
    /// Beside the peak rather than instead of it because neither answers on its
    /// own: the peak cannot distinguish a signal that fills the converter from
    /// one twice too big for it, and this saturates as soon as a
    /// constant-envelope signal passes √2 of full scale. Together they say both
    /// *whether* and roughly *how far*.
    pub adc_clip: f32,
    /// Present while transmitting.
    pub tx: Option<TxMeters>,
    /// A WFM stereo pilot is locked on the main receiver. Drives the `ST`
    /// indicator; always `false` in every other mode.
    pub stereo: bool,
    /// The CTCSS tone or DCS code being received on the main receiver. Drives
    /// the sub-audible readout; always `None` outside NFM, and `None` in NFM
    /// until a tone has been present long enough to be sure of.
    pub tone: Option<crate::SubTone>,
}

impl Meters {
    /// The front end is running into its rails, so nothing downstream — this
    /// struct's own `s_dbm` included — is reading an undistorted signal.
    pub fn adc_overloaded(&self) -> bool {
        self.adc_clip > OVERLOAD_FRACTION
    }

    /// S-units for display: S9 = -73 dBm, 6 dB per unit below, dB-over-9 above.
    pub fn s_units(&self) -> (u8, f32) {
        let over = self.s_dbm + 73.0;
        if over >= 0.0 {
            (9, over)
        } else {
            let units = 9.0 + over / 6.0;
            (units.max(0.0) as u8, 0.0)
        }
    }
}
