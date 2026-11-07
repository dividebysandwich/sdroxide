//! The station's HobbyPCB Hardrock amplifier: the HR500, or the HR50 that a lot
//! of ELAD owners pair with their FDM-DUO.
//!
//! Like the T/R switch this is one box for the whole station, so its settings
//! live with the rest of the station's in `amp.json` and travel in
//! [`crate::StationConfig`]. The serial port is driven by the `sdroxide-hardrock`
//! crate on the engine's machine; what crosses the wire is the vocabulary here,
//! which is plain data and wasm-safe, so a remote operator gets the same panel
//! as the one sitting at the station.

use serde::{Deserialize, Serialize};

use crate::SerialConfig;

/// Which Hardrock it is. They share one command set; what differs is how far
/// the meters go and where the warnings start.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum AmpModel {
    #[default]
    Hr500,
    Hr50,
}

impl AmpModel {
    pub const ALL: [AmpModel; 2] = [AmpModel::Hr500, AmpModel::Hr50];

    pub fn label(self) -> &'static str {
        match self {
            AmpModel::Hr500 => "Hardrock-500",
            AmpModel::Hr50 => "Hardrock-50 (untested)",
        }
    }

    /// Only the HR500 has been tested on a real amplifier. The HR50's limits
    /// are scaled from the HR500's and its tuner is shown as raw steps; either
    /// could be wrong in ways that matter to the hardware.
    pub fn tested(self) -> bool {
        matches!(self, AmpModel::Hr500)
    }

    /// The short name the panel's title carries.
    pub fn short(self) -> &'static str {
        match self {
            AmpModel::Hr500 => "HR500",
            AmpModel::Hr50 => "HR50",
        }
    }

    /// Meter full scales, and where each reading turns from green to yellow
    /// and from yellow to red the way the amplifier's own display does. Red
    /// is a trip: the amplifier has taken itself off line.
    pub fn limits(self) -> AmpLimits {
        match self {
            // Hardrock-500 Operating Instructions (3-1-21), "Status
            // Indicators" and "Temperature Indicator". The scales run past
            // the trip points so the red stretch shows. FWD runs to 625 W
            // rather than the devices' 750 W, since the firmware trips above
            // 600 W: its labels fall every 125 W, so yellow starts on the
            // 500 mark (as the operator asked).
            AmpModel::Hr500 => AmpLimits {
                fwd_w: 625.0,
                fwd_warn_w: 500.0,
                fwd_trip_w: 600.0,
                rfl_w: 60.0,
                rfl_warn_w: 30.0,
                rfl_trip_w: 50.0,
                // 20 W full scale puts the 8 W yellow and the 12 W red on
                // labelled ticks (4, 8, 12, 16, 20). On 15 W the yellow began
                // just past the 7.5 W tick and read as about 7.25 W (AJ3G).
                drive_w: 20.0,
                drive_warn_w: 8.0,
                drive_trip_w: 12.0,
                vdd_v: 60.0,
                vdd_low_v: 45.0,
                idd_a: 25.0,
                idd_warn_a: 18.0,
                idd_trip_a: 20.0,
                // Yellow from 120 °F (48.9 °C), a step ahead of the amplifier's
                // own 122 °F, as the operator asked.
                temp_warn_c: 48.8,
                temp_red_c: 65.0,
                temp_trip_c: 70.0,
            },
            // The HR50 is a 50 W amplifier driven with about 5 W from a 13.8 V
            // supply. Scaled from the HR500 until someone checks it against an
            // HR50 manual.
            AmpModel::Hr50 => AmpLimits {
                fwd_w: 75.0,
                fwd_warn_w: 50.0,
                fwd_trip_w: 60.0,
                rfl_w: 10.0,
                rfl_warn_w: 5.0,
                rfl_trip_w: 8.0,
                drive_w: 10.0,
                drive_warn_w: 6.0,
                drive_trip_w: 8.0,
                vdd_v: 16.0,
                vdd_low_v: 11.5,
                idd_a: 12.0,
                idd_warn_a: 9.0,
                idd_trip_a: 10.0,
                // Yellow from 120 °F (48.9 °C), a step ahead of the amplifier's
                // own 122 °F, as the operator asked.
                temp_warn_c: 48.8,
                temp_red_c: 65.0,
                temp_trip_c: 70.0,
            },
        }
    }

    /// How the tuner's steps turn into inductance and capacitance, where
    /// known.
    ///
    /// The HR50's built-in tuner may use different parts; until its values
    /// are known the panel shows raw steps.
    pub fn atu_scale(self) -> Option<AtuScale> {
        match self {
            // ESTIMATE, not measured on the board. The ATU-500's firmware
            // (V1.1F) drives one relay per step bit, with no lookup table:
            //   * L: binary coils, 0.05 µH for bit 0 doubling up to 1.6 µH,
            //     which is 0.05 µH a step (HobbyPCB's Control Panel and the
            //     manual's 3.2 µH);
            //   * C: series pairs of 22, 47, 100, 220, 470, 1000 and 2200 pF
            //     (schematic sheet 1), so each bit is worth half its part.
            //     These aren't binary, so C is the sum of the bits that are on.
            // Plus a floor for what the board adds with every bit off: FR4
            // traces with no controlled impedance, and general-purpose relays
            // with contact inductance closed and contact capacitance open. A
            // schematic can't give the floor, so it was fitted on the bench
            // (AJ3G, 17 Oct 2026, 20 m, tune at L step 3 / C step 12,
            // C IN) to the tuner's 1.23:1 into a load the VNA measured at
            // 30.5 − j0.88 Ω, and the VNA at the amp input, same tune, into
            // 50 Ω (43.6 − j20.4 Ω). Best fit: 50 nH (also the manual's
            // minimum) and 10 pF. Full working: docs/hardrock/atu-values.md.
            AmpModel::Hr500 => Some(AtuScale {
                uh: 0.05,
                pf_bits: [11.0, 23.5, 50.0, 110.0, 235.0, 500.0, 1100.0],
                uh_floor: 0.05,
                pf_floor: 10.0,
            }),
            AmpModel::Hr50 => None,
        }
    }

    /// How many antenna jacks it has.
    pub fn antennas(self) -> u8 {
        match self {
            AmpModel::Hr500 => 2,
            AmpModel::Hr50 => 1,
        }
    }
}

/// See [`AmpModel::atu_scale`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AtuScale {
    /// Per inductor step.
    pub uh: f32,
    /// Each capacitor bit's value, bit 0 first.
    pub pf_bits: [f32; 7],
    /// In circuit whatever the step: relay and trace strays.
    pub uh_floor: f32,
    pub pf_floor: f32,
}

impl AtuScale {
    pub fn inductance_uh(&self, step: u8) -> f32 {
        self.uh_floor + f32::from(step) * self.uh
    }

    pub fn capacitance_pf(&self, step: u8) -> f32 {
        let on = self.pf_bits.iter().enumerate().filter(|(i, _)| step >> i & 1 == 1);
        self.pf_floor + on.map(|(_, pf)| pf).sum::<f32>()
    }
}

/// See [`AmpModel::limits`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AmpLimits {
    pub fwd_w: f32,
    pub fwd_warn_w: f32,
    pub fwd_trip_w: f32,
    pub rfl_w: f32,
    pub rfl_warn_w: f32,
    pub rfl_trip_w: f32,
    pub drive_w: f32,
    pub drive_warn_w: f32,
    pub drive_trip_w: f32,
    pub vdd_v: f32,
    /// Supply below this is yellow. A low supply never trips the amplifier.
    pub vdd_low_v: f32,
    pub idd_a: f32,
    pub idd_warn_a: f32,
    pub idd_trip_a: f32,
    pub temp_warn_c: f32,
    /// The numerals turn red here…
    pub temp_red_c: f32,
    /// …and the amplifier goes off line here until it cools.
    pub temp_trip_c: f32,
}

/// One of the amplifier's readings, for [`AmpLimits::level`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AmpGauge {
    Fwd,
    Rfl,
    Drive,
    Vdd,
    Idd,
    Temp,
}

/// The colour the amplifier shows a reading in. Red means it tripped.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AmpLevel {
    #[default]
    Green,
    Yellow,
    Red,
}

impl AmpLimits {
    /// How a reading stands against this model's levels.
    pub fn level(&self, g: AmpGauge, r: &AmpReadings) -> AmpLevel {
        // The manual's levels are "greater than": exactly 500 W is still green.
        let above = |v: f32, warn: f32, trip: f32| {
            if v > trip {
                AmpLevel::Red
            } else if v > warn {
                AmpLevel::Yellow
            } else {
                AmpLevel::Green
            }
        };
        match g {
            AmpGauge::Fwd => above(r.fwd_w, self.fwd_warn_w, self.fwd_trip_w),
            AmpGauge::Rfl => above(r.rfl_w, self.rfl_warn_w, self.rfl_trip_w),
            // In OFF (bypass) the radio's power goes straight through to the
            // tuner, so a 100 W radio running barefoot reads far over the
            // drive limits with nothing wrong (AJ3G, 18 Oct).
            AmpGauge::Drive if !r.ptt => AmpLevel::Green,
            AmpGauge::Drive => above(r.drive_w, self.drive_warn_w, self.drive_trip_w),
            AmpGauge::Idd => above(r.idd_a, self.idd_warn_a, self.idd_trip_a),
            AmpGauge::Temp => above(r.temp_c(), self.temp_warn_c, self.temp_red_c),
            // Zero means not measured yet.
            AmpGauge::Vdd if r.vdd_v > 0.0 && r.vdd_v < self.vdd_low_v => AmpLevel::Yellow,
            AmpGauge::Vdd => AmpLevel::Green,
        }
    }

    /// The colour a live reading shows. Past a trip level is yellow here: only
    /// a trip the amplifier actually makes, held in [`AmpAlerts`], is red.
    /// The temperature's red is the amplifier's own display colour, not a
    /// trip, so it stays.
    pub fn live_level(&self, g: AmpGauge, r: &AmpReadings) -> AmpLevel {
        let l = self.level(g, r);
        if g == AmpGauge::Temp { l } else { l.min(AmpLevel::Yellow) }
    }

    /// Hot enough that the amplifier has gone off line until it cools.
    pub fn over_temp(&self, r: &AmpReadings) -> bool {
        r.temp_c() > self.temp_trip_c
    }
}

/// The five status indicators across the top of the amplifier's meter screen.
/// A warning or trip during a transmission stays lit after it, so the
/// operator sees it happened; the next key-up clears them all.
///
/// A reading past a trip level is only a trip if the amplifier actually went
/// off line: a poll can catch a peak the amplifier's own, filtered, trip
/// logic rides through. So the held levels keep "past the trip level", but
/// show red only once [`AmpAlerts::tripped`] is set.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AmpAlerts {
    pub fwd: AmpLevel,
    pub rfl: AmpLevel,
    pub drive: AmpLevel,
    pub vdd: AmpLevel,
    pub idd: AmpLevel,
    /// The amplifier was seen to drop off line while keyed: drive going in,
    /// in PTT, and no gain coming out.
    pub tripped: bool,
    /// These are the amplifier's own lights, read from its status, rather
    /// than worked out here from the readings: show them exactly as sent.
    pub from_amp: bool,
}

impl AmpAlerts {
    /// The colour an indicator shows: red only after a real trip.
    pub fn get(&self, g: AmpGauge) -> AmpLevel {
        let held = match g {
            AmpGauge::Fwd => self.fwd,
            AmpGauge::Rfl => self.rfl,
            AmpGauge::Drive => self.drive,
            AmpGauge::Vdd => self.vdd,
            AmpGauge::Idd => self.idd,
            AmpGauge::Temp => AmpLevel::Green,
        };
        if self.tripped { held } else { held.min(AmpLevel::Yellow) }
    }

    /// The colour to show for a gauge: the amplifier's own light when it
    /// sends them, otherwise the live reading or what the transmission held,
    /// whichever is worse. The amplifier has no temperature light.
    pub fn shown(&self, l: &AmpLimits, g: AmpGauge, r: &AmpReadings) -> AmpLevel {
        if g == AmpGauge::Drive && !r.ptt {
            // Not a warning in bypass (see `AmpLimits::level`), whatever is lit.
            AmpLevel::Green
        } else if self.from_amp && g != AmpGauge::Temp {
            self.get(g)
        } else {
            l.live_level(g, r).max(self.get(g))
        }
    }

    /// Fold one reading taken while transmitting into the held indicators.
    pub fn hold(&mut self, l: &AmpLimits, r: &AmpReadings) {
        for (slot, g) in [
            (&mut self.fwd, AmpGauge::Fwd),
            (&mut self.rfl, AmpGauge::Rfl),
            (&mut self.drive, AmpGauge::Drive),
            (&mut self.vdd, AmpGauge::Vdd),
            (&mut self.idd, AmpGauge::Idd),
        ] {
            *slot = (*slot).max(l.level(g, r));
        }
    }

    /// Mark one indicator as the cause of a trip that no reading caught.
    pub fn blame(&mut self, g: AmpGauge) {
        match g {
            AmpGauge::Fwd => self.fwd = AmpLevel::Red,
            AmpGauge::Rfl => self.rfl = AmpLevel::Red,
            AmpGauge::Drive => self.drive = AmpLevel::Red,
            AmpGauge::Idd => self.idd = AmpLevel::Red,
            AmpGauge::Vdd | AmpGauge::Temp => {}
        }
    }

    /// The indicators that were past their trip level when the amplifier
    /// tripped, by name. Empty when it has not tripped, and also when it
    /// tripped with nothing seen past a level.
    pub fn trips(&self) -> Vec<&'static str> {
        if !self.tripped {
            return Vec::new();
        }
        [("FWD", self.fwd), ("RFL", self.rfl), ("DRV", self.drive), ("IDD", self.idd)]
            .into_iter()
            .filter(|(_, l)| *l == AmpLevel::Red)
            .map(|(n, _)| n)
            .collect()
    }
}

/// Settings for the amplifier link, saved as `amp.json` on the engine's machine.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AmpConfig {
    pub enabled: bool,
    pub model: AmpModel,
    /// The same type the CAT panel edits, so the port picker is the same
    /// widget. Only the path and the baud rate matter; the amplifier is 8-N-1.
    pub serial: SerialConfig,
    /// Send the dial frequency (`FA`) on every band change, so the amplifier's
    /// filters follow the radio without a CAT cable of their own.
    pub follow_band: bool,
    /// Show the amplifier's forward power and SWR on the TX meter when the
    /// radio itself does not measure them.
    pub feed_tx_meter: bool,
}

impl Default for AmpConfig {
    fn default() -> Self {
        AmpConfig {
            enabled: false,
            model: AmpModel::Hr500,
            // HobbyPCB's own Control Panel insists on 115200, so an amplifier
            // that has ever been used with it is already there.
            serial: SerialConfig { baud: 115_200, ..SerialConfig::default() },
            follow_band: true,
            feed_tx_meter: true,
        }
    }
}

/// Baud rates the Hardrock offers on its USB port.
pub const AMP_BAUD_RATES: [u32; 5] = [115_200, 38_400, 19_200, 9_600, 4_800];

/// What the operator asked the amplifier to do from the panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AmpAction {
    /// `true` puts the amplifier in line (PTT), `false` in bypass (OFF).
    SetPtt(bool),
    SetAntenna(u8),
    /// `HRBN` code: 1 = 10 m … 10 = 160 m.
    SetBand(u8),
    BandUp,
    BandDown,
    /// Start a tune, or stop one in progress.
    Tune,
    /// `true` puts the tuner in line, `false` bypasses it.
    SetAtu(bool),
    /// `true` for Celsius.
    SetCelsius(bool),
}

/// One `HRST` report.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AmpReadings {
    pub fwd_w: f32,
    pub rfl_w: f32,
    pub drive_w: f32,
    /// 0 when there is too little power to measure.
    pub swr: f32,
    pub vdd_v: f32,
    pub idd_a: f32,
    pub temperature: f32,
    /// `Some(true)` Celsius, `Some(false)` Fahrenheit, `None` not yet known.
    pub celsius: Option<bool>,
    /// In line (PTT) rather than bypassed (OFF).
    pub ptt: bool,
    /// `HRBN` code, `None` when the amplifier is on no band it knows.
    pub band: Option<u8>,
    pub antenna: u8,
}

impl AmpReadings {
    /// The temperature in Celsius, whichever scale the amplifier reports in.
    pub fn temp_c(&self) -> f32 {
        if self.celsius == Some(false) { (self.temperature - 32.0) / 1.8 } else { self.temperature }
    }
}

/// The tuner, when there is one.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AtuReadout {
    /// In line (`HRTB1`) rather than bypassed.
    pub active: bool,
    pub tuning: bool,
    /// How the last tune ended, in words.
    pub last_result: Option<String>,
    /// From `HRTMP`; `None` until the tuner has answered.
    pub network: Option<AtuNetwork>,
}

/// The L network the tuner has switched in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AtuNetwork {
    pub l_step: u8,
    pub c_step: u8,
    pub bypassed: bool,
    /// Capacitor on the antenna side of the inductor.
    pub cap_output: bool,
    pub swr: Option<f32>,
}

/// What the engine knows about the amplifier, sent when it changes.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AmpStatus {
    /// An amplifier is enabled in the settings.
    pub configured: bool,
    /// It is answering on its port.
    pub connected: bool,
    pub model: AmpModel,
    /// Why it is not connected, when it is not.
    pub error: Option<String>,
    pub readings: Option<AmpReadings>,
    /// The status indicators as the amplifier would be showing them.
    pub alerts: AmpAlerts,
    /// `None` when no tuner is fitted.
    pub atu: Option<AtuReadout>,
}

/// The band names `HRBN` codes stand for, 1 = 10 m … 10 = 160 m.
pub fn amp_band_label(code: u8) -> &'static str {
    match code {
        1 => "10M",
        2 => "12M",
        3 => "15M",
        4 => "17M",
        5 => "20M",
        6 => "30M",
        7 => "40M",
        8 => "60M",
        9 => "80M",
        10 => "160M",
        _ => "UNK",
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_tuner_values_include_the_board_strays() {
        let sc = AmpModel::Hr500.atu_scale().unwrap();
        // Step 3 / step 12 from the 30.5 Ω bench test.
        assert!((sc.inductance_uh(3) - 0.20).abs() < 1e-4);
        assert!((sc.capacitance_pf(12) - 170.0).abs() < 1e-3);
        // Not linear at the top: bit 6 alone is the 2200 pF pair.
        assert!((sc.capacitance_pf(64) - 1110.0).abs() < 1e-3);
        assert!((sc.capacitance_pf(127) - 2039.5).abs() < 1e-3);
        assert!(AmpModel::Hr50.atu_scale().is_none());
    }

    use super::*;

    #[test]
    fn fahrenheit_converts() {
        let r = AmpReadings { temperature: 77.0, celsius: Some(false), ..AmpReadings::default() };
        assert!((r.temp_c() - 25.0).abs() < 1e-4);
    }

    #[test]
    fn default_matches_the_hobbypcb_panel() {
        let c = AmpConfig::default();
        assert_eq!(c.serial.baud, 115_200);
        assert!(c.follow_band);
        assert_eq!(amp_band_label(5), "20M");
        assert_eq!(amp_band_label(99), "UNK");
    }

    #[test]
    fn levels_follow_the_hr500_manual() {
        let l = AmpModel::Hr500.limits();
        let at =
            |t: f32| AmpReadings { temperature: t, celsius: Some(true), ..AmpReadings::default() };
        assert_eq!(l.level(AmpGauge::Temp, &at(48.0)), AmpLevel::Green);
        assert_eq!(l.level(AmpGauge::Temp, &at(49.0)), AmpLevel::Yellow);
        assert_eq!(l.level(AmpGauge::Temp, &at(55.0)), AmpLevel::Yellow);
        let f =
            |t: f32| AmpReadings { temperature: t, celsius: Some(false), ..AmpReadings::default() };
        assert_eq!(l.level(AmpGauge::Temp, &f(119.0)), AmpLevel::Green);
        assert_eq!(l.level(AmpGauge::Temp, &f(120.0)), AmpLevel::Yellow);
        assert_eq!(l.level(AmpGauge::Temp, &at(66.0)), AmpLevel::Red);
        assert!(!l.over_temp(&at(69.0)) && l.over_temp(&at(71.0)));
        let fwd = |w: f32| AmpReadings { fwd_w: w, ..AmpReadings::default() };
        assert_eq!(l.level(AmpGauge::Fwd, &fwd(500.0)), AmpLevel::Green);
        assert_eq!(l.level(AmpGauge::Fwd, &fwd(550.0)), AmpLevel::Yellow);
        assert_eq!(l.level(AmpGauge::Fwd, &fwd(610.0)), AmpLevel::Red);
        let v = |v: f32| AmpReadings { vdd_v: v, ..AmpReadings::default() };
        assert_eq!(l.level(AmpGauge::Vdd, &v(0.0)), AmpLevel::Green);
        assert_eq!(l.level(AmpGauge::Vdd, &v(52.0)), AmpLevel::Green);
        // A sagging supply warns but never trips.
        assert_eq!(l.level(AmpGauge::Vdd, &v(30.0)), AmpLevel::Yellow);
    }

    #[test]
    fn alerts_hold_the_worst_of_a_transmission() {
        let l = AmpModel::Hr500.limits();
        let mut a = AmpAlerts::default();
        a.hold(&l, &AmpReadings { rfl_w: 55.0, fwd_w: 520.0, ..AmpReadings::default() });
        a.hold(&l, &AmpReadings::default());
        assert_eq!((a.rfl, a.fwd), (AmpLevel::Red, AmpLevel::Yellow));
        // Past the trip level but the amp stayed on line: a warning, not a trip.
        assert_eq!(a.get(AmpGauge::Rfl), AmpLevel::Yellow);
        assert!(a.trips().is_empty());
        a.tripped = true;
        assert_eq!(a.get(AmpGauge::Rfl), AmpLevel::Red);
        assert_eq!(a.trips(), vec!["RFL"]);
        assert_eq!(
            l.live_level(AmpGauge::Idd, &AmpReadings { idd_a: 21.0, ..AmpReadings::default() }),
            AmpLevel::Yellow
        );
    }
}
