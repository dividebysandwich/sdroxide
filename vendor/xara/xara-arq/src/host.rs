use crate::compression::Compression;
use xara_phy::fm::level::{FmBandwidth, Record};
use xara_phy::hf::level::Bandwidth;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkBw {
    Hf(Bandwidth),
    Fm(FmBandwidth),
}

impl From<Bandwidth> for LinkBw {
    fn from(bw: Bandwidth) -> Self {
        LinkBw::Hf(bw)
    }
}

impl From<FmBandwidth> for LinkBw {
    fn from(bw: FmBandwidth) -> Self {
        LinkBw::Fm(bw)
    }
}

#[derive(Clone, Debug)]
pub struct HostSettings {
    pub mycalls: Vec<String>,
    pub chat: bool,
    pub registered: bool,
    pub retries: u32,
    pub compression: Compression,
}

impl HostSettings {
    pub fn new(mycall: &str, retries: u32) -> Self {
        Self { mycalls: vec![mycall.into()], chat: false, registered: false, retries, compression: Compression::Text }
    }

    pub fn primary_call(&self) -> String {
        self.mycalls.first().cloned().unwrap_or_default()
    }
}

pub const QRT_RECEIVED: &str = "QRT Received";

#[derive(Clone, Debug, PartialEq)]
pub enum HostEvent {
    Connected { caller: String, answerer: String, vias: Vec<String>, bw: LinkBw },
    Disconnected,
    UnableToConnect,
    TimedOut,
    Pending,
    CancelPending,
    Unencrypted,
    Registered(bool),
    Buffer(usize),
    Data(Vec<u8>),
    Sn(f64),
    RxFrame(Record),
    Log(String),
}
