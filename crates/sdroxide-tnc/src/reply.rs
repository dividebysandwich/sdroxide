//! A reading of the modem's reply lines, for the status display.
//!
//! Only what the panel shows is told apart; anything else is [`Reply::Other`].
//! Both modems word these lines the same way (Mercury `docs/TNC.md`, Data2G
//! `native/core/host/host.hpp`).

#[derive(Clone, Debug, PartialEq)]
pub enum Reply {
    Ok,
    Wrong,
    /// `CONNECTED <source> <destination> [<bandwidth>]`.
    Connected {
        caller: String,
        answerer: String,
        bw: Option<String>,
    },
    /// `DISCONNECTED [why]`.
    Disconnected,
    /// An incoming call (or our own CQ frame) is under way.
    Pending,
    CancelPending,
    Ptt(bool),
    Busy(bool),
    /// Bytes not yet acknowledged by the far end.
    Buffer(u32),
    /// SNR of the last burst heard, dB.
    Sn(f32),
    /// Mercury: `BITRATE (<level>) <bps> BPS`.
    Bitrate {
        level: u8,
        bps: u32,
    },
    /// Data2G: `MODE <name> …`, the submode now in use.
    Mode(String),
    IAmAlive,
    Registered(String),
    Version(String),
    Other(String),
}

impl Reply {
    pub fn parse(line: &str) -> Reply {
        let line = line.trim();
        let mut w = line.split_ascii_whitespace();
        let head = w.next().unwrap_or("").to_ascii_uppercase();
        let on = |s: Option<&str>| s.is_some_and(|s| s.eq_ignore_ascii_case("ON"));
        match head.as_str() {
            "OK" => Reply::Ok,
            "WRONG" => Reply::Wrong,
            "CONNECTED" => {
                let caller = w.next().unwrap_or("").to_string();
                let answerer = w.next().unwrap_or("").to_string();
                let bw = w.next().map(str::to_string);
                Reply::Connected { caller, answerer, bw }
            }
            "DISCONNECTED" => Reply::Disconnected,
            "PENDING" => Reply::Pending,
            "CANCELPENDING" => Reply::CancelPending,
            "PTT" => Reply::Ptt(on(w.next())),
            "BUSY" => Reply::Busy(on(w.next())),
            "BUFFER" => match w.next().and_then(|n| n.parse().ok()) {
                Some(n) => Reply::Buffer(n),
                None => Reply::Other(line.into()),
            },
            "SN" => match w.next().and_then(|n| n.parse().ok()) {
                Some(s) => Reply::Sn(s),
                None => Reply::Other(line.into()),
            },
            "BITRATE" => {
                let level = w.next().map(|l| l.trim_matches(|c| c == '(' || c == ')'));
                let bps = w.next();
                match (level.and_then(|l| l.parse().ok()), bps.and_then(|b| b.parse().ok())) {
                    (Some(level), Some(bps)) => Reply::Bitrate { level, bps },
                    _ => Reply::Other(line.into()),
                }
            }
            "MODE" => match w.next() {
                Some(m) => Reply::Mode(m.to_string()),
                None => Reply::Other(line.into()),
            },
            "IAMALIVE" => Reply::IAmAlive,
            "REGISTERED" => Reply::Registered(w.next().unwrap_or("").to_string()),
            "VERSION" => Reply::Version(w.collect::<Vec<_>>().join(" ")),
            _ => Reply::Other(line.into()),
        }
    }

    /// Lines that repeat on their own and are not worth a transcript entry.
    pub fn is_chatter(&self) -> bool {
        matches!(
            self,
            Reply::Ptt(_) | Reply::Busy(_) | Reply::Buffer(_) | Reply::IAmAlive | Reply::Sn(_)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_both_modems_wordings() {
        assert_eq!(
            Reply::parse("CONNECTED KC2G N0CALL 2300"),
            Reply::Connected {
                caller: "KC2G".into(),
                answerer: "N0CALL".into(),
                bw: Some("2300".into())
            }
        );
        assert_eq!(Reply::parse("PTT ON"), Reply::Ptt(true));
        assert_eq!(Reply::parse("BUSY OFF"), Reply::Busy(false));
        assert_eq!(Reply::parse("BUFFER 1234"), Reply::Buffer(1234));
        assert_eq!(Reply::parse("SN -3.5"), Reply::Sn(-3.5));
        assert_eq!(Reply::parse("BITRATE (4) 1240 BPS"), Reply::Bitrate { level: 4, bps: 1240 });
        assert_eq!(Reply::parse("MODE w48q34 2400 120 64 0.50 6.00"), Reply::Mode("w48q34".into()));
        assert_eq!(Reply::parse("DISCONNECTED timeout"), Reply::Disconnected);
        assert_eq!(Reply::parse("VERSION Data2G 0.4.3"), Reply::Version("Data2G 0.4.3".into()));
        assert!(matches!(Reply::parse("BUFFER x"), Reply::Other(_)));
    }
}
