use crate::hf::level::Bandwidth;

pub const ACK_STAY: u32 = 1;
pub const NACK: u32 = 2;
pub const QRT: u32 = 3;
pub const ACK_UP2: u32 = 5;
pub const ACK_DOWN: u32 = 6;
pub const ACK_UP1: u32 = 7;
pub const ACK_UP3: u32 = 8;
pub const ACK_UP4: u32 = 9;
pub const ACK_SECOND_ONLY: u32 = 10;
pub const ACK_FIRST_ONLY: u32 = 11;
pub const ACK_REQ_0: u32 = 12;
pub const ACK_REQ_1: u32 = 13;
pub const GO_AHEAD: u32 = 14;
pub const CONNECT_REPLY_2300: u32 = 15;
pub const NACK_MISSED: u32 = 19;
pub const NO_TRAFFIC: u32 = 21;
pub const CONNECT_REPLY_500: u32 = 22;
pub const CONNECT_REQ_2300: u32 = 26;
pub const CONNECT_REQ_500: u32 = 31;
pub const CONNECT_REQ_2750: u32 = 36;
pub const CONNECT_REPLY_2750: u32 = 41;
pub const REJECT: u32 = 43;
pub const KISS_L1: u32 = 45;
pub const QRT_NO_CWID: u32 = 46;
pub const KISS_L2: u32 = 47;
pub const CQ: u32 = 48;
pub const BREAK: u32 = 49;
pub const IDLE: u32 = 50;
pub const START_LEVEL: u32 = 50;
pub const KISS_L4: u32 = 58;

pub const ACK_REQ: [u32; 2] = [ACK_REQ_0, ACK_REQ_1];
pub const QRT_ANY: [u32; 2] = [QRT, QRT_NO_CWID];
pub const KISS: [u32; 3] = [KISS_L1, KISS_L2, KISS_L4];
pub const LONG_LEADIN: [u32; 4] = [GO_AHEAD, NO_TRAFFIC, BREAK, IDLE];
pub const UNUSED: [u32; 2] = [4, 20];
pub const ALIASES: u32 = 5;
pub const QUALITIES: u32 = 4;
pub const START_LEVELS: u32 = 7;

const BANDS: [Bandwidth; 3] = [Bandwidth::Hz500, Bandwidth::Hz2300, Bandwidth::Hz2750];

pub const fn connect_req(bw: Bandwidth, alias: u32) -> u32 {
    alias
        + match bw {
            Bandwidth::Hz500 => CONNECT_REQ_500,
            Bandwidth::Hz2300 => CONNECT_REQ_2300,
            Bandwidth::Hz2750 => CONNECT_REQ_2750,
        }
}

pub fn connect_req_alias(code: u32) -> Option<(Bandwidth, u32)> {
    BANDS.into_iter().find_map(|bw| code.checked_sub(connect_req(bw, 0)).filter(|&k| k < ALIASES).map(|k| (bw, k)))
}

pub const fn connect_reply(bw: Bandwidth, q: u32) -> u32 {
    q - 1
        + match bw {
            Bandwidth::Hz500 => CONNECT_REPLY_500,
            Bandwidth::Hz2300 => CONNECT_REPLY_2300,
            Bandwidth::Hz2750 => CONNECT_REPLY_2750,
        }
}

pub fn connect_reply_q(code: u32) -> Option<(Bandwidth, u32)> {
    BANDS
        .into_iter()
        .find_map(|bw| code.checked_sub(connect_reply(bw, 1)).filter(|&k| k < QUALITIES).map(|k| (bw, k + 1)))
}

pub const fn start_level_code(n: u32) -> u32 {
    START_LEVEL + n
}

pub fn start_level(code: u32) -> Option<u32> {
    code.checked_sub(START_LEVEL).filter(|n| (1..=START_LEVELS).contains(n))
}

pub fn connect_reqs(bw: Bandwidth, aliases: u32) -> impl Iterator<Item = u32> {
    (0..aliases).map(move |k| connect_req(bw, k))
}

pub fn connect_replies(bw: Bandwidth) -> impl Iterator<Item = u32> {
    (1..=QUALITIES).map(move |q| connect_reply(bw, q))
}

pub fn start_levels() -> impl Iterator<Item = u32> {
    (1..=START_LEVELS).map(start_level_code)
}
