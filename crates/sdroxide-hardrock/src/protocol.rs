//! The Hardrock serial protocol: ASCII, `;`-terminated, `HR`-prefixed, plus the
//! Kenwood-style `FA`/`IF` frequency commands the amplifier also accepts.
//!
//! Source: *Hardrock-500 Operating Instructions*, 3-1-21, section 6 "Serial
//! Commands". The HR50 manual text is visibly the ancestor of that section
//! (several HR500 examples still say "HR50"), which is why the same command set
//! is expected to work on both. Places where the manual contradicts itself are
//! called out next to the code that has to pick one reading.
//!
//! Nothing here touches a port: [`Command::encode`] makes the bytes,
//! [`Framer`] splits what comes back into frames and [`Response::parse`]
//! reads one. That keeps the whole protocol testable without an amplifier.

use std::fmt;

/// The ten bands the HR500 switches, numbered as `HRBN` numbers them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Band {
    M10,
    M12,
    M15,
    M17,
    M20,
    M30,
    M40,
    M60,
    M80,
    M160,
}

impl Band {
    pub const ALL: [Band; 10] = [
        Band::M160,
        Band::M80,
        Band::M60,
        Band::M40,
        Band::M30,
        Band::M20,
        Band::M17,
        Band::M15,
        Band::M12,
        Band::M10,
    ];

    /// The `HRBN` code: 1 = 10 m up to 10 = 160 m.
    pub fn code(self) -> u8 {
        match self {
            Band::M10 => 1,
            Band::M12 => 2,
            Band::M15 => 3,
            Band::M17 => 4,
            Band::M20 => 5,
            Band::M30 => 6,
            Band::M40 => 7,
            Band::M60 => 8,
            Band::M80 => 9,
            Band::M160 => 10,
        }
    }

    /// `None` for anything else, including the 99 the amplifier reports when it
    /// is on no band it knows.
    pub fn from_code(code: u8) -> Option<Band> {
        Band::ALL.into_iter().find(|b| b.code() == code)
    }

    pub fn label(self) -> &'static str {
        match self {
            Band::M10 => "10m",
            Band::M12 => "12m",
            Band::M15 => "15m",
            Band::M17 => "17m",
            Band::M20 => "20m",
            Band::M30 => "30m",
            Band::M40 => "40m",
            Band::M60 => "60m",
            Band::M80 => "80m",
            Band::M160 => "160m",
        }
    }

    /// Accepts `40`, `40m` or `40M`.
    pub fn from_label(s: &str) -> Option<Band> {
        let s = s.trim().trim_end_matches(['m', 'M']);
        Band::ALL.into_iter().find(|b| b.label().trim_end_matches('m') == s)
    }
}

impl fmt::Display for Band {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// `HRMD`: whether the amplifier keys with the radio or stays in bypass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyMode {
    Off,
    Ptt,
}

/// The four `HRPW` readings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Meter {
    Forward,
    Reflected,
    Drive,
    Swr,
}

impl Meter {
    fn letter(self) -> char {
        match self {
            Meter::Forward => 'F',
            Meter::Reflected => 'R',
            Meter::Drive => 'D',
            Meter::Swr => 'V',
        }
    }

    fn from_letter(c: char) -> Option<Meter> {
        match c.to_ascii_uppercase() {
            'F' => Some(Meter::Forward),
            'R' => Some(Meter::Reflected),
            'D' => Some(Meter::Drive),
            'V' => Some(Meter::Swr),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TempScale {
    C,
    F,
}

/// How the last ATU tune ended (`HRTR`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TuneResult {
    Aborted,
    SwrTooHigh,
    NotBelow2to1,
    PowerTooHigh,
    PowerTooLow,
    Tuned,
    Unknown(char),
}

impl TuneResult {
    fn from_letter(c: char) -> TuneResult {
        match c {
            'A' => TuneResult::Aborted,
            'E' => TuneResult::SwrTooHigh,
            'F' => TuneResult::NotBelow2to1,
            'H' => TuneResult::PowerTooHigh,
            'L' => TuneResult::PowerTooLow,
            // The manual lists both T and S as "tuned successfully".
            'T' | 'S' => TuneResult::Tuned,
            other => TuneResult::Unknown(other),
        }
    }
}

/// Everything a computer can send. Each GET is the SET's letters with no data.
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// `HRAA;` — is anything listening.
    Ping,
    /// `HRST;` — every reading at once; the one to poll.
    Status,
    /// `HRPWx;`
    Read(Meter),
    GetBand,
    SetBand(Band),
    /// `FA<11 digits>;` — the amplifier works out the band itself.
    SetFrequency(u64),
    GetMode,
    SetMode(KeyMode),
    GetAntenna,
    SetAntenna(u8),
    /// `HRTP;`
    GetTemperature,
    SetTempScale(TempScale),
    /// `HRVT;`
    GetVoltage,
    /// `HRAP;`
    AtuPresent,
    /// `HRTB0;` bypasses the ATU, `HRTB1;` puts it in line.
    SetAtuActive(bool),
    GetAtuActive,
    /// `HRTU;` — same as the front-panel TUNE button. Cancels a tune in progress.
    Tune,
    /// `HRTT;`
    IsTuning,
    /// `HRTR;`
    LastTuneResult,
    /// `HRTMP;` — the tuner's `P` message passed through: the network it has
    /// switched in, which side the capacitor is on, and the return loss.
    AtuParams,
    /// Anything else, sent as written with a `;` added if missing.
    Raw(String),
}

impl Command {
    pub fn encode(&self) -> String {
        match self {
            Command::Ping => "HRAA;".into(),
            Command::Status => "HRST;".into(),
            Command::Read(m) => format!("HRPW{};", m.letter()),
            Command::GetBand => "HRBN;".into(),
            Command::SetBand(b) => format!("HRBN{};", b.code()),
            Command::SetFrequency(hz) => format!("FA{hz:011};"),
            Command::GetMode => "HRMD;".into(),
            Command::SetMode(KeyMode::Off) => "HRMD0;".into(),
            Command::SetMode(KeyMode::Ptt) => "HRMD1;".into(),
            Command::GetAntenna => "HRAN;".into(),
            Command::SetAntenna(n) => format!("HRAN{n};"),
            Command::GetTemperature => "HRTP;".into(),
            Command::SetTempScale(TempScale::C) => "HRTSC;".into(),
            Command::SetTempScale(TempScale::F) => "HRTSF;".into(),
            Command::GetVoltage => "HRVT;".into(),
            Command::AtuPresent => "HRAP;".into(),
            Command::SetAtuActive(on) => format!("HRTB{};", u8::from(*on)),
            Command::GetAtuActive => "HRTB;".into(),
            Command::Tune => "HRTU;".into(),
            Command::IsTuning => "HRTT;".into(),
            Command::LastTuneResult => "HRTR;".into(),
            Command::AtuParams => "HRTMP;".into(),
            Command::Raw(s) => {
                let s = s.trim();
                if s.ends_with(';') { s.to_string() } else { format!("{s};") }
            }
        }
    }

    /// The prefix of the frame that answers this command, or `None` for a SET
    /// that gets no answer.
    pub fn reply_prefix(&self) -> Option<&'static str> {
        match self {
            Command::Ping => Some("HRAA"),
            Command::Status => Some("HRST"),
            Command::Read(_) => Some("HRPW"),
            Command::GetBand => Some("HRBN"),
            Command::GetMode => Some("HRMD"),
            Command::GetAntenna => Some("HRAN"),
            Command::GetTemperature => Some("HRTP"),
            Command::GetVoltage => Some("HRVT"),
            Command::AtuPresent => Some("HRAP"),
            Command::GetAtuActive => Some("HRTB"),
            Command::IsTuning => Some("HRTT"),
            // The manual's RSP line for HRTR reads "HRTTx;", which looks like a
            // copy of the line above it. Both are accepted; see `Response::parse`.
            Command::LastTuneResult => Some("HRTR"),
            Command::AtuParams => Some("HRTM"),
            _ => None,
        }
    }
}

/// The amplifier's own indicator lights, from the 8-digit field it sends
/// after the antenna (not in the 3-1-21 manual). Each light is 1 green,
/// 2 yellow, 3 red, 0 not lit. Checked against an HR500's screen on a 160 m
/// overdrive trip (Oct 2026): FWD red, DRV and VDD yellow. The RFL and IDD
/// positions are the remaining two, by elimination; neither has been seen
/// to change yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lights {
    /// Transmitting.
    pub tx: bool,
    pub fwd: u8,
    pub rfl: u8,
    pub drv: u8,
    pub vdd: u8,
    pub idd: u8,
}

/// One `HRST` report.
#[derive(Debug, Clone, PartialEq)]
pub struct Status {
    pub forward_w: f32,
    pub reflected_w: f32,
    /// Sent in tenths of a watt.
    pub drive_w: f32,
    /// Sent in tenths; 0.0 when there is too little power to measure.
    pub swr: f32,
    pub vdd_v: f32,
    /// The manual says whole amps, but a real HR500 sends tenths: it reported
    /// 70 for about 7 A on a ~100 W carrier, which read as a 70 A trip.
    pub idd_a: f32,
    /// In whichever scale the amplifier is set to.
    pub temperature: f32,
    pub mode: KeyMode,
    /// `None` when the amplifier reports 99 (no band).
    pub band: Option<Band>,
    pub antenna: u8,
    /// Fields after the antenna, which the manual leaves out. The stock
    /// control panel reads per-indicator status digits from them.
    pub extra: Vec<String>,
}

impl Status {
    /// The amplifier's own lights, when its firmware sends them.
    pub fn lights(&self) -> Option<Lights> {
        let d: Vec<u8> = self.extra.first()?.bytes().map(|b| b.wrapping_sub(b'0')).collect();
        if d.len() < 6 || d.iter().any(|&x| x > 9) {
            return None;
        }
        Some(Lights { tx: d[0] == 1, fwd: d[1], rfl: d[2], drv: d[3], vdd: d[4], idd: d[5] })
    }

    /// `HRST-fff-rrr-ddd-sss-vvv-iii-ttt-m-bb-a-` (the `;` already stripped).
    ///
    /// Fields are taken by position and trimmed, so zero- or space-padding both
    /// read. Trailing fields past the antenna are kept, unread, in `extra`.
    pub fn parse(body: &str) -> Option<Status> {
        let rest = body.strip_prefix("HRST")?;
        let f: Vec<&str> = rest.split('-').map(str::trim).filter(|s| !s.is_empty()).collect();
        if f.len() < 10 {
            return None;
        }
        let num = |i: usize| f[i].parse::<f32>().ok();
        Some(Status {
            forward_w: num(0)?,
            reflected_w: num(1)?,
            drive_w: num(2)? / 10.0,
            swr: num(3)? / 10.0,
            vdd_v: num(4)?,
            idd_a: num(5)? / 10.0,
            temperature: num(6)?,
            mode: if f[7] == "1" { KeyMode::Ptt } else { KeyMode::Off },
            band: f[8].parse::<u8>().ok().and_then(Band::from_code),
            antenna: f[9].parse().ok()?,
            extra: f[10..].iter().map(|x| x.to_string()).collect(),
        })
    }
}

/// The tuner's answer to `HRTMP;`.
///
/// The manual lists the `P` message ("reports the tuning parameters") but not
/// its layout. A real ATU-500 answers `HRTM-L000-C000-H0-B1-S255-F255-`: each
/// field is the letter of the tuner command that sets or reports it, then
/// its value. So `L` and `C` are the steps, `H` the capacitor side, `B` the
/// tuner's band, `S` the last autotune's status and `F` its final return
/// loss, 255 meaning none. It is read by letter, not position, so a firmware
/// that orders the fields differently still reads. The step sizes are the
/// ones HobbyPCB's own HR500 Control Panel multiplies by.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AtuParams {
    /// Inductor step, 0-63.
    pub l_step: u8,
    /// Capacitor step, 0-127.
    pub c_step: u8,
    /// The tuner reports itself bypassed (a `Y1` field). The `P` reply seen so
    /// far carries no `Y`; the amplifier's `HRTB` is the reliable answer.
    pub bypassed: bool,
    /// The capacitor sits on the antenna side of the inductor rather than the
    /// amplifier side (`H1`).
    pub cap_output: bool,
    /// Return loss in tenths of a dB; `None` when there is nothing to read.
    pub return_loss_tenths: Option<u16>,
}

impl AtuParams {
    /// The HR500 Control Panel's step size for the ATU-500's inductor bank.
    pub const UH_PER_L_STEP: f32 = 0.05;
    /// The HR500 Control Panel's step size for the ATU-500's capacitor bank.
    pub const PF_PER_C_STEP: f32 = 10.0;

    /// Read the body of an `HRTM…` frame (the `;` already stripped).
    pub fn parse(body: &str) -> Option<AtuParams> {
        let rest = body.strip_prefix("HRTM")?;
        let mut l = None;
        let mut c = None;
        let mut h = None;
        let mut y = None;
        let mut rl = None;
        for field in rest.split('-').map(str::trim).filter(|f| !f.is_empty()) {
            let mut chars = field.chars();
            let tag = chars.next()?.to_ascii_uppercase();
            let value: u16 = chars.as_str().trim().parse().ok()?;
            match tag {
                'L' => l = Some(value),
                'C' => c = Some(value),
                'H' => h = Some(value),
                'Y' => y = Some(value),
                // The final return loss after a tune; `R` is the tuner's
                // command for the current one, should a firmware send that.
                'F' => rl = Some(value),
                'R' if rl.is_none() => rl = Some(value),
                _ => {}
            }
        }
        let l_step = u8::try_from(l?).ok().filter(|&l| l <= 63)?;
        let c_step = u8::try_from(c?).ok().filter(|&c| c <= 127)?;
        Some(AtuParams {
            l_step,
            c_step,
            bypassed: y == Some(1),
            cap_output: h? == 1,
            // The Control Panel only trusts 0 < RL < 241; 255 is "none".
            return_loss_tenths: rl.filter(|&rl| rl > 0 && rl < 241),
        })
    }

    /// The inductance switched in, using the ATU-500's step size.
    pub fn inductance_uh(&self) -> f32 {
        f32::from(self.l_step) * Self::UH_PER_L_STEP
    }

    /// The capacitance switched in, using the ATU-500's step size.
    pub fn capacitance_pf(&self) -> f32 {
        f32::from(self.c_step) * Self::PF_PER_C_STEP
    }

    /// SWR from the return loss, the way the Control Panel works it out.
    pub fn swr(&self) -> Option<f32> {
        let k = 10f32.powf(f32::from(self.return_loss_tenths?) / 200.0);
        Some((k + 1.0) / (k - 1.0))
    }
}

/// One frame from the amplifier, read.
#[derive(Debug, Clone, PartialEq)]
pub enum Response {
    Ack,
    Status(Status),
    Meter(Meter, f32),
    Band(Option<Band>),
    Mode(KeyMode),
    Antenna(u8),
    Temperature(f32, Option<TempScale>),
    TempScale(TempScale),
    Voltage(f32),
    AtuPresent(bool),
    AtuActive(bool),
    Tuning(bool),
    TuneResult(TuneResult),
    AtuParams(AtuParams),
    /// `FA…;` or `IF…;` echoed back.
    Frequency(u64),
    /// Anything this version does not read, kept whole.
    Other(String),
}

impl Response {
    /// Read one frame. `frame` may or may not still carry its `;`.
    pub fn parse(frame: &str) -> Response {
        let body = frame.trim().trim_end_matches(';').trim();
        let up = body.to_ascii_uppercase();
        let tail = |p: &str| up.strip_prefix(p).map(str::trim);

        if up == "HRAA" {
            return Response::Ack;
        }
        if up.starts_with("HRST")
            && let Some(s) = Status::parse(&up)
        {
            return Response::Status(s);
        }
        if let Some(t) = tail("HRPW") {
            let mut c = t.chars();
            if let Some(m) = c.next().and_then(Meter::from_letter)
                && let Ok(v) = c.as_str().trim().parse::<f32>()
            {
                return Response::Meter(m, v);
            }
        }
        if let Some(t) = tail("HRBN")
            && let Ok(n) = t.parse::<u8>()
        {
            return Response::Band(Band::from_code(n));
        }
        if let Some(t) = tail("HRMD") {
            match t {
                "0" => return Response::Mode(KeyMode::Off),
                "1" => return Response::Mode(KeyMode::Ptt),
                _ => {}
            }
        }
        if let Some(t) = tail("HRAN")
            && let Ok(n) = t.parse()
        {
            return Response::Antenna(n);
        }
        if let Some(t) = tail("HRTS") {
            match t {
                "C" => return Response::TempScale(TempScale::C),
                "F" => return Response::TempScale(TempScale::F),
                _ => {}
            }
        }
        if let Some(t) = tail("HRTP") {
            let (num, scale) = match t.chars().last() {
                Some('C') => (&t[..t.len() - 1], Some(TempScale::C)),
                Some('F') => (&t[..t.len() - 1], Some(TempScale::F)),
                _ => (t, None),
            };
            if let Ok(v) = num.trim().parse() {
                return Response::Temperature(v, scale);
            }
        }
        if let Some(t) = tail("HRVT")
            && let Ok(v) = t.trim_end_matches('V').trim().parse()
        {
            return Response::Voltage(v);
        }
        if let Some(t) = tail("HRAP") {
            return Response::AtuPresent(t == "1");
        }
        if let Some(t) = tail("HRTB") {
            return Response::AtuActive(t == "1");
        }
        if up.starts_with("HRTM")
            && let Some(p) = AtuParams::parse(&up)
        {
            return Response::AtuParams(p);
        }
        if let Some(t) = tail("HRTR")
            && let Some(c) = t.chars().next()
        {
            return Response::TuneResult(TuneResult::from_letter(c));
        }
        if let Some(t) = tail("HRTT") {
            match t {
                "0" => return Response::Tuning(false),
                "1" => return Response::Tuning(true),
                // A letter here is the HRTR answer in the form the manual
                // prints it ("HRTTx;").
                _ => {
                    if let Some(c) = t.chars().next() {
                        return Response::TuneResult(TuneResult::from_letter(c));
                    }
                }
            }
        }
        if let Some(t) = tail("FA").or_else(|| tail("IF")) {
            let digits: String = t.chars().take(11).collect();
            if digits.len() == 11
                && let Ok(hz) = digits.parse()
            {
                return Response::Frequency(hz);
            }
        }
        Response::Other(body.to_string())
    }
}

/// Splits the byte stream into `;`-terminated frames. Partial frames are kept
/// until the rest arrives; stray CR/LF and NULs between frames are dropped.
#[derive(Debug, Default)]
pub struct Framer {
    buf: String,
}

impl Framer {
    pub fn push(&mut self, bytes: &[u8]) -> Vec<String> {
        self.buf
            .extend(bytes.iter().filter(|b| !matches!(b, b'\r' | b'\n' | 0)).map(|&b| b as char));
        let mut out = Vec::new();
        while let Some(i) = self.buf.find(';') {
            let frame: String = self.buf.drain(..=i).collect();
            let frame = frame.trim().to_string();
            if frame != ";" {
                out.push(frame);
            }
        }
        // A runaway with no terminator is noise, not a frame worth waiting on.
        if self.buf.len() > 256 {
            self.buf.clear();
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_examples_encode() {
        assert_eq!(Command::SetMode(KeyMode::Off).encode(), "HRMD0;");
        assert_eq!(Command::SetBand(Band::M40).encode(), "HRBN7;");
        assert_eq!(Command::SetFrequency(14_060_000).encode(), "FA00014060000;");
        assert_eq!(Command::SetAntenna(2).encode(), "HRAN2;");
        assert_eq!(Command::SetAtuActive(false).encode(), "HRTB0;");
        assert_eq!(Command::SetTempScale(TempScale::C).encode(), "HRTSC;");
        assert_eq!(Command::Read(Meter::Forward).encode(), "HRPWF;");
        assert_eq!(Command::Raw("HRTMC?".into()).encode(), "HRTMC?;");
    }

    #[test]
    fn band_codes_round_trip() {
        for b in Band::ALL {
            assert_eq!(Band::from_code(b.code()), Some(b));
            assert_eq!(Band::from_label(b.label()), Some(b));
        }
        assert_eq!(Band::from_code(99), None);
        assert_eq!(Band::from_label("160"), Some(Band::M160));
        assert_eq!(Band::from_label("6m"), None);
    }

    #[test]
    fn status_reads_every_field() {
        let r = Response::parse("HRST-350-012-040-013-058-180-041-1-07-1-;");
        let Response::Status(s) = r else { panic!("{r:?}") };
        assert_eq!(s.forward_w, 350.0);
        assert_eq!(s.reflected_w, 12.0);
        assert_eq!(s.drive_w, 4.0);
        assert!((s.swr - 1.3).abs() < 1e-6);
        assert_eq!(s.vdd_v, 58.0);
        assert!((s.idd_a - 18.0).abs() < 1e-5);
        assert_eq!(s.temperature, 41.0);
        assert_eq!(s.mode, KeyMode::Ptt);
        assert_eq!(s.band, Some(Band::M40));
        assert_eq!(s.antenna, 1);
    }

    #[test]
    fn lights_from_a_real_trip() {
        // AJ3G's HR500 just after a 160 m overdrive trip: transmitting,
        // FWD red, DRV and VDD yellow.
        let r = Response::parse("HRST-000-000-035-000-042-000-075-1-01-1-13122101-;");
        let Response::Status(s) = r else { panic!("{r:?}") };
        let l = s.lights().unwrap();
        assert!(l.tx);
        assert_eq!((l.fwd, l.rfl, l.drv, l.vdd, l.idd), (3, 1, 2, 2, 1));
        let plain = Response::parse("HRST-350-012-040-013-058-180-041-1-07-1-;");
        let Response::Status(s) = plain else { panic!() };
        assert_eq!(s.lights(), None);
    }

    #[test]
    fn drain_current_is_in_tenths() {
        // What a real HR500 sent on a ~100 W carrier.
        let r = Response::parse("HRST-100-001-030-011-058-070-030-1-07-1-;");
        let Response::Status(s) = r else { panic!("{r:?}") };
        assert!((s.idd_a - 7.0).abs() < 1e-5);
    }

    #[test]
    fn status_tolerates_padding_and_unknown_band() {
        let r = Response::parse("HRST-  0-  0-  0-  0- 58-  0- 25-0-99-2-;");
        let Response::Status(s) = r else { panic!("{r:?}") };
        assert_eq!(s.band, None);
        assert_eq!(s.mode, KeyMode::Off);
        assert_eq!(s.antenna, 2);
    }

    #[test]
    fn short_status_is_not_a_status() {
        assert!(matches!(Response::parse("HRST-350-012-;"), Response::Other(_)));
    }

    #[test]
    fn single_readings() {
        assert_eq!(Response::parse("HRPWF350;"), Response::Meter(Meter::Forward, 350.0));
        assert_eq!(Response::parse("HRPWV1.5;"), Response::Meter(Meter::Swr, 1.5));
        assert_eq!(Response::parse("HRBN7;"), Response::Band(Some(Band::M40)));
        assert_eq!(Response::parse("HRBN99;"), Response::Band(None));
        assert_eq!(Response::parse("HRMD1;"), Response::Mode(KeyMode::Ptt));
        assert_eq!(Response::parse("HRAP1;"), Response::AtuPresent(true));
        assert_eq!(Response::parse("HRTT0;"), Response::Tuning(false));
        assert_eq!(Response::parse("HRTP41C;"), Response::Temperature(41.0, Some(TempScale::C)));
        assert_eq!(Response::parse("HRVT57.9V;"), Response::Voltage(57.9));
        assert_eq!(Response::parse("HRAA;"), Response::Ack);
        assert_eq!(Response::parse("FA00014060000;"), Response::Frequency(14_060_000));
    }

    #[test]
    fn tune_result_in_either_spelling() {
        assert_eq!(Response::parse("HRTRT;"), Response::TuneResult(TuneResult::Tuned));
        assert_eq!(Response::parse("HRTTE;"), Response::TuneResult(TuneResult::SwrTooHigh));
    }

    #[test]
    fn atu_params_from_a_real_hr500() {
        // AJ3G's HR500 and ATU-500, untuned, nothing transmitted (17 Oct 2026).
        let r = Response::parse("HRTM-L000-C000-H0-B1-S255-F255-;");
        let Response::AtuParams(p) = r else { panic!("{r:?}") };
        assert_eq!((p.l_step, p.c_step), (0, 0));
        // H0: capacitor on the input side. B1 is the tuner's band, not bypass:
        // switching HRTB0/HRTB1 left both digits unchanged.
        assert_eq!((p.bypassed, p.cap_output), (false, false));
        // 255 is the "nothing measured" value, past the panel's 240 limit.
        assert_eq!(p.return_loss_tenths, None);
        // No tune attempted yet: a bare HRTR.
        assert!(matches!(Response::parse("HRTR;"), Response::Other(_)));
    }

    #[test]
    fn atu_params_by_position() {
        // A tuned network in the real tuner's layout.
        let r = Response::parse("HRTM-L012-C034-H1-B5-S000-F145-;");
        let Response::AtuParams(p) = r else { panic!("{r:?}") };
        assert_eq!((p.l_step, p.c_step), (12, 34));
        assert!(!p.bypassed);
        assert!(p.cap_output);
        assert!((p.inductance_uh() - 0.60).abs() < 1e-4);
        assert_eq!(p.capacitance_pf(), 340.0);
        assert_eq!(p.return_loss_tenths, Some(145));
        // 14.5 dB return loss is about 1.46:1.
        assert!((p.swr().unwrap() - 1.46).abs() < 0.01);
        assert_eq!(Command::AtuParams.encode(), "HRTMP;");
    }

    #[test]
    fn atu_params_reject_nonsense() {
        assert!(AtuParams::parse("HRTM 8").is_none());
        assert!(AtuParams::parse("HRTM-L099-C034-H1-B5-S000-F145-").is_none());
        assert!(AtuParams::parse("HRTM-L012-C034-B5-F145-").is_none());
        // Fields in another order, and a tuner that says it is bypassed.
        let p = AtuParams::parse("HRTM-F000-Y1-C000-L000-H0-").unwrap();
        assert!(p.bypassed);
        assert_eq!(p.return_loss_tenths, None);
        assert_eq!(p.swr(), None);
    }

    #[test]
    fn framer_handles_split_and_joined_frames() {
        let mut f = Framer::default();
        assert!(f.push(b"HRPW").is_empty());
        assert_eq!(f.push(b"F350;\r\nHRAA;HRBN"), vec!["HRPWF350;", "HRAA;"]);
        assert_eq!(f.push(b"7;"), vec!["HRBN7;"]);
    }
}
