use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TxMeters {
    /// Forward power in watts, if the device exposes a sensor for it.
    pub fwd_w: Option<f32>,
    pub swr: Option<f32>,
    /// 0.0..=1.0 modulation drive level.
    pub alc: f32,
}

/// TX-side telemetry a rig reports out-of-band (CAT / TCI): forward power and
/// SWR. Distinct from [`TxMeters`], which also carries the engine's own ALC —
/// this is only what the *device* measures, merged into `TxMeters` by the
/// engine while transmitting.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct TxTelemetry {
    /// Forward power in watts, if the device exposes a sensor for it.
    pub fwd_w: Option<f32>,
    /// SWR as a ratio (e.g. `1.4` = 1.4:1), if the device measures it.
    pub swr: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Meters {
    /// Signal level in the RX passband, dBm (after `cal_offset_db`).
    pub s_dbm: f32,
    /// ADC headroom indicator.
    pub adc_peak_dbfs: f32,
    /// Present while transmitting.
    pub tx: Option<TxMeters>,
}

impl Meters {
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
