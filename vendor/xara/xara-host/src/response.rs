use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkWidth {
    Hz(u32),
    Narrow,
    Wide,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Response {
    Ok,
    Wrong,
    Busy(bool),
    Ptt(bool),
    Bitrate { level: usize, bps: u32, tx: bool },
    Pending,
    CancelPending,
    Connected { caller: String, answerer: String, vias: Vec<String>, width: LinkWidth },
    Disconnected,
    LinkRegistered(bool),
    Buffer(usize),
    Sn(f64),
    IAmAlive,
    Registered(Option<Vec<String>>),
    EncryptionDisabled,
    UnencryptedLink,
    Tune(f64),
    CleanTxBuffer { empty: bool },
    Version(&'static str),
    Line(String),
}

fn printable(s: &str) -> String {
    s.chars().map(|c| if c.is_ascii_graphic() || c == ' ' { c } else { '?' }).collect()
}

pub fn shown_call(c: &str) -> String {
    let mut v: Vec<char> = c.chars().collect();
    if v.len() == 9 && v[7] == '-' && "RTXL".contains(v[8]) {
        v.remove(7);
    }
    printable(&v.into_iter().collect::<String>()).replace(' ', "?")
}

fn on_off(on: bool) -> &'static str {
    if on { "ON" } else { "OFF" }
}

impl fmt::Display for Response {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Response::Ok => f.write_str("OK"),
            Response::Wrong => f.write_str("WRONG"),
            Response::Busy(on) => write!(f, "BUSY {}", on_off(*on)),
            Response::Ptt(on) => write!(f, "PTT {}", on_off(*on)),
            Response::Bitrate { level, bps, tx } => {
                write!(f, "BITRATE ({level})  {bps} bps {}", if *tx { "TX" } else { "RX" })
            }
            Response::Pending => f.write_str("PENDING"),
            Response::CancelPending => f.write_str("CANCELPENDING"),
            Response::Connected { caller, answerer, vias, width } => {
                write!(f, "CONNECTED {} {}", shown_call(caller), shown_call(answerer))?;
                if !vias.is_empty() {
                    f.write_str(" VIA")?;
                    for v in vias {
                        write!(f, " {}", shown_call(v))?;
                    }
                }
                match width {
                    LinkWidth::Hz(hz) => write!(f, " {hz}"),
                    LinkWidth::Narrow => f.write_str(" NARROW"),
                    LinkWidth::Wide => f.write_str(" WIDE"),
                }
            }
            Response::Disconnected => f.write_str("DISCONNECTED"),
            Response::LinkRegistered(r) => f.write_str(if *r { "LINK REGISTERED" } else { "LINK UNREGISTERED" }),
            Response::Buffer(n) => write!(f, "BUFFER {n}"),
            Response::Sn(s) => write!(f, "SN {s:.1}"),
            Response::IAmAlive => f.write_str("IAMALIVE"),
            Response::Registered(calls) => {
                f.write_str("REGISTERED")?;
                for c in calls.iter().flatten() {
                    write!(f, " {}", printable(c))?;
                }
                Ok(())
            }
            Response::EncryptionDisabled => f.write_str("ENCRYPTION DISABLED"),
            Response::UnencryptedLink => f.write_str("UNENCRYPTED LINK"),
            Response::Tune(d) => write!(f, "TUNE {d}"),
            Response::CleanTxBuffer { empty: true } => f.write_str("CLEANTXBUFFER BUFFEREMPTY"),
            Response::CleanTxBuffer { empty: false } => f.write_str("CLEANTXBUFFER FAILED "),
            Response::Version(v) => f.write_str(v),
            Response::Line(l) => f.write_str(&printable(l)),
        }
    }
}
