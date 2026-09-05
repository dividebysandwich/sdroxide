//! The **N1MM+ `contactinfo` broadcast**: one XML datagram per logged contact.
//!
//! A second dialect of what [`crate`] already does in WSJT-X's (issue #337).
//! The two are not alternatives so much as different vocabularies: a logger
//! that ignores WSJT-X's binary datagrams may well listen for N1MM's XML, and
//! some — the World Radio League's desktop bridge among them — listen for both.
//!
//! Only the logging half exists here. N1MM broadcasts several message types;
//! this sends `contactinfo` and nothing else, because that is the one that says
//! "a contact happened" and the rest describe a contest database sdroxide does
//! not have.
//!
//! # The two fields that are not what they look like
//!
//! - **`rxfreq`/`txfreq` are in tens of hertz**, not hertz and not kilohertz.
//!   N1MM's own example logs 3.52519 MHz as `352519`. A logger fed hertz here
//!   files the contact four decimal places away and says nothing about it.
//! - **`band` is the band's lower edge in megahertz as text** — `3.5`, `14`,
//!   `1.8` — and not the ADIF `80m` spelling. N1MM's documentation notes that
//!   the decimal separator follows the sending machine's Windows locale, so a
//!   reader has to cope with both; this always writes a point, which is the
//!   form every reader handles.
//!
//! Everything else is either the contact as ADIF has it or a contest field
//! sdroxide has no equivalent for, sent empty. Empty is deliberate and not a
//! gap to fill later: `section`, `prec` and `ck` mean whatever a particular
//! contest's rules say they mean, so inventing values would be inventing
//! claims about a contest that was not being worked.
//!
//! NATIVE ONLY — it binds a UDP socket.

use std::net::{ToSocketAddrs, UdpSocket};

use sdroxide_types::{N1mmConfig, QsoRecord};
use tracing::{debug, info};

/// A configured sender. Fire-and-forget, like [`crate::WsjtxUdp`]: a logger
/// that is not running must never stall the radio.
pub struct N1mmUdp {
    sock: UdpSocket,
    dest: std::net::SocketAddr,
    station: String,
    addr: String,
}

impl N1mmUdp {
    /// Bind a socket and resolve the destination. Fails only on a bad address
    /// or an unusable local port.
    pub fn start(cfg: &N1mmConfig) -> Result<Self, String> {
        let host = if cfg.host.trim().is_empty() { "127.0.0.1" } else { cfg.host.trim() };
        let dest = (host, cfg.port)
            .to_socket_addrs()
            .map_err(|e| format!("{host}:{}: {e}", cfg.port))?
            .next()
            .ok_or_else(|| format!("{host}:{}: no address", cfg.port))?;
        let sock = UdpSocket::bind(if dest.is_ipv4() { "0.0.0.0:0" } else { "[::]:0" })
            .map_err(|e| e.to_string())?;
        // N1MM's own documentation tells operators to send to their subnet's
        // broadcast address so every position on a contest network hears it,
        // and a socket has to be asked before the kernel will carry one.
        if dest.is_ipv4() {
            let _ = sock.set_broadcast(true);
        }
        if dest.ip().is_multicast() {
            let _ = sock.set_multicast_ttl_v4(2);
        }
        let station = if cfg.station.trim().is_empty() {
            "SDROXIDE".to_string()
        } else {
            cfg.station.trim().to_string()
        };
        let addr = format!("{host}:{}", cfg.port);
        info!(dest = %addr, station = %station, "N1MM contactinfo broadcast started");
        Ok(N1mmUdp { sock, dest, station, addr })
    }

    /// The destination this sender was built for, so the engine can tell a
    /// config change that needs a rebuild from one that doesn't.
    pub fn addr(&self) -> &str {
        &self.addr
    }

    /// Announce one completed contact.
    pub fn qso_logged(&self, q: &QsoRecord) {
        let packet = contactinfo(q, &self.station);
        if let Err(e) = self.sock.send_to(packet.as_bytes(), self.dest) {
            debug!(dest = %self.addr, error = %e, "N1MM UDP send failed");
        }
    }
}

/// Build the `contactinfo` document for one contact.
///
/// Pure, and tested as such: UDP gives no feedback whatsoever, so a field of
/// the wrong shape does not fail, it quietly files somebody's contact wrong.
pub fn contactinfo(q: &QsoRecord, station: &str) -> String {
    let el = |name: &str, value: &str| format!("  <{name}>{}</{name}>\n", escape(value));
    let mode = q.mode.to_ascii_uppercase();
    let stamp = timestamp(q.start_utc);
    // N1MM has one frequency per direction and sdroxide logs one for the
    // contact, so both carry it. Tens of hertz — see the module note.
    let tens = format!("{}", (q.freq_hz / 10.0).round().max(0.0) as u64);
    let mut out = String::with_capacity(1024);
    out.push_str("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<contactinfo>\n");
    // "N1MM", not "sdroxide": the readers of this packet dispatch on it, the
    // same reason `WsjtxConfig::id` defaults to "WSJT-X".
    out.push_str(&el("app", "N1MM"));
    out.push_str(&el("contestname", if q.contest_id.trim().is_empty() {
        "DX"
    } else {
        q.contest_id.trim()
    }));
    out.push_str(&el("contestnr", "1"));
    out.push_str(&el("timestamp", &stamp));
    out.push_str(&el("mycall", q.my_call.trim()));
    out.push_str(&el("band", &band(q)));
    out.push_str(&el("rxfreq", &tens));
    out.push_str(&el("txfreq", &tens));
    out.push_str(&el("operator", q.operator.trim()));
    out.push_str(&el("mode", &mode));
    out.push_str(&el("call", q.call.trim()));
    out.push_str(&el("countryprefix", ""));
    out.push_str(&el("wpxprefix", ""));
    out.push_str(&el("stationprefix", q.my_call.trim()));
    out.push_str(&el("continent", q.continent.trim()));
    out.push_str(&el("snt", &report(q.rst_sent)));
    out.push_str(&el("sntnr", &q.stx.unwrap_or(0).to_string()));
    out.push_str(&el("rcv", &report(q.rst_rcvd)));
    out.push_str(&el("rcvnr", &q.srx.unwrap_or(0).to_string()));
    out.push_str(&el("gridsquare", q.grid.as_deref().unwrap_or("")));
    out.push_str(&el("exchange1", q.srx_string.trim()));
    // The contest fields sdroxide has no equivalent for — see the module note
    // on why they are empty rather than guessed at.
    out.push_str(&el("section", ""));
    out.push_str(&el("comment", q.comment.trim()));
    out.push_str(&el("qth", q.qth.trim()));
    out.push_str(&el("name", q.name.trim()));
    out.push_str(&el("power", &q.tx_pwr.map(|w| format!("{w}")).unwrap_or_default()));
    out.push_str(&el("misctext", ""));
    out.push_str(&el("zone", &q.cq_zone.unwrap_or(0).to_string()));
    out.push_str(&el("prec", ""));
    out.push_str(&el("ck", "0"));
    // Multiplier and point scoring belong to a contest's rules, which are the
    // logger's business and not ours: claiming a point for a contact worked
    // outside a contest would put a number in somebody's score.
    out.push_str(&el("ismultiplier1", "0"));
    out.push_str(&el("ismultiplier2", "0"));
    out.push_str(&el("ismultiplier3", "0"));
    out.push_str(&el("points", "0"));
    out.push_str(&el("radionr", "1"));
    out.push_str(&el("RoverLocation", ""));
    out.push_str(&el("RadioInterfaced", "1"));
    out.push_str(&el("NetworkedCompNr", "0"));
    // This *is* the station the contact was logged on — sdroxide is not
    // forwarding somebody else's record.
    out.push_str(&el("IsOriginal", "True"));
    out.push_str(&el("NetBiosName", station));
    out.push_str(&el("IsRunQSO", "0"));
    out.push_str(&el("StationName", station));
    out.push_str(&el("ID", &id(q)));
    out.push_str(&el("IsClaimedQso", "1"));
    // In a `contactinfo` these are the same as `timestamp` and `call`; they
    // differ only in the `contactreplace` packet, which sdroxide never sends.
    out.push_str(&el("oldtimestamp", &stamp));
    out.push_str(&el("oldcall", q.call.trim()));
    out.push_str(&el("SentExchange", q.stx_string.trim()));
    out.push_str("</contactinfo>\n");
    out
}

/// A signal report as N1MM writes one: the number alone, empty where there is
/// none.
fn report(db: Option<i16>) -> String {
    db.map(|v| v.to_string()).unwrap_or_default()
}

/// The band as N1MM writes it: the frequency in megahertz, trimmed to the band
/// designator its example uses — `3.5`, `14`, `1.8`.
///
/// From the frequency rather than from the ADIF `band` string, because the two
/// spellings have nothing in common: `80m` is `3.5` here, and there is no table
/// to look one up in that the number does not already answer.
fn band(q: &QsoRecord) -> String {
    let mhz = q.freq_hz / 1e6;
    if mhz <= 0.0 {
        return String::new();
    }
    // One decimal where the band needs one (1.8, 3.5, 10.1), none where it does
    // not (14, 21, 144) — which is what N1MM's own examples show.
    let rounded = (mhz * 10.0).floor() / 10.0;
    if (rounded.fract()).abs() < 1e-9 {
        format!("{}", rounded as i64)
    } else {
        format!("{rounded:.1}")
    }
}

/// N1MM's `ID`: a 32-character hex identifier, unique per contact in the log.
///
/// Derived from the contact rather than random, so the same QSO announced twice
/// — a re-send, or two clients watching one station — carries one identity and
/// a logger can recognise the repeat instead of logging it again.
fn id(q: &QsoRecord) -> String {
    // FNV-1a over the fields that identify a contact, twice with different
    // offsets to fill the 128 bits N1MM's format has room for. Not a GUID and
    // not pretending to be one: what the readers need is 32 hex characters that
    // do not collide between two contacts, which this gives without a
    // dependency.
    let seed = format!("{}|{}|{}|{}", q.call.trim(), q.start_utc, q.freq_hz, q.mode.trim());
    let mut out = String::with_capacity(32);
    for offset in [0xcbf2_9ce4_8422_2325u64, 0x9e37_79b9_7f4a_7c15] {
        let mut h = offset;
        for b in seed.as_bytes() {
            h ^= u64::from(*b);
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
        out.push_str(&format!("{h:016x}"));
    }
    out
}

/// `YYYY-MM-DD HH:MM:SS`, UTC — the format N1MM's own examples carry.
fn timestamp(unix: i64) -> String {
    let (y, mo, d, h, mi, s) = sdroxide_types::utc_ymd_hms(unix);
    format!("{y:04}-{mo:02}-{d:02} {h:02}:{mi:02}:{s:02}")
}

/// Escape text for an XML element body.
///
/// A callsign cannot contain any of these, but a name, a QTH or a comment
/// certainly can — and one `&` in an operator's note would otherwise produce a
/// document the reader throws away whole.
fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            // Control characters are not valid in XML at all, and a logger that
            // is strict about it drops the datagram rather than the character.
            c if (c as u32) < 0x20 && c != '\t' => out.push(' '),
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn qso() -> QsoRecord {
        QsoRecord {
            call: "W9XYZ".into(),
            my_call: "W1AW".into(),
            // 3.52519 MHz — N1MM's own documented example, whose `rxfreq` is
            // 352519. The one field most likely to be got wrong.
            freq_hz: 3_525_190.0,
            band: "80m".into(),
            mode: "CW".into(),
            rst_sent: Some(599),
            rst_rcvd: Some(599),
            start_utc: 1_579_279_418, // 2020-01-17 16:43:38 UTC
            ..QsoRecord::default()
        }
    }

    /// Issue #337, and the trap in it: the frequency fields are tens of hertz.
    #[test]
    fn the_frequency_goes_out_in_tens_of_hertz() {
        let x = contactinfo(&qso(), "SHACK");
        assert!(x.contains("<rxfreq>352519</rxfreq>"), "{x}");
        assert!(x.contains("<txfreq>352519</txfreq>"), "{x}");
        // …and the band is the megahertz designator, not the ADIF spelling.
        assert!(x.contains("<band>3.5</band>"), "{x}");
        assert!(!x.contains("80m"), "{x}");
    }

    #[test]
    fn a_contact_reads_back_the_way_n1mm_writes_one() {
        let x = contactinfo(&qso(), "SHACK");
        assert!(x.starts_with("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<contactinfo>"), "{x}");
        assert!(x.ends_with("</contactinfo>\n"), "{x}");
        // The reader dispatches on this, so it says N1MM.
        assert!(x.contains("<app>N1MM</app>"), "{x}");
        assert!(x.contains("<call>W9XYZ</call>"), "{x}");
        assert!(x.contains("<mycall>W1AW</mycall>"), "{x}");
        assert!(x.contains("<mode>CW</mode>"), "{x}");
        assert!(x.contains("<snt>599</snt>"), "{x}");
        assert!(x.contains("<timestamp>2020-01-17 16:43:38</timestamp>"), "{x}");
        assert!(x.contains("<StationName>SHACK</StationName>"), "{x}");
        // A contactinfo's old* fields repeat the current ones.
        assert!(x.contains("<oldcall>W9XYZ</oldcall>"), "{x}");
        assert!(x.contains("<oldtimestamp>2020-01-17 16:43:38</oldtimestamp>"), "{x}");
    }

    /// A band whose designator has no decimal, and one that does.
    #[test]
    fn the_band_designator_follows_the_frequency() {
        let at = |hz: f64| band(&QsoRecord { freq_hz: hz, ..QsoRecord::default() });
        assert_eq!(at(1_830_000.0), "1.8");
        assert_eq!(at(3_525_190.0), "3.5");
        assert_eq!(at(10_120_000.0), "10.1");
        assert_eq!(at(14_074_000.0), "14");
        assert_eq!(at(144_200_000.0), "144.2");
        assert_eq!(at(0.0), "");
    }

    /// One `&` in an operator's note must not cost the whole datagram.
    #[test]
    fn text_that_would_break_the_document_is_escaped() {
        let q = QsoRecord {
            name: "Bob & Sue".into(),
            comment: "<not a tag>".into(),
            ..qso()
        };
        let x = contactinfo(&q, "SHACK");
        assert!(x.contains("<name>Bob &amp; Sue</name>"), "{x}");
        assert!(x.contains("<comment>&lt;not a tag&gt;</comment>"), "{x}");
    }

    /// The same contact announced twice carries one identity, so a logger can
    /// tell a repeat from a second QSO.
    #[test]
    fn the_contact_id_is_stable_and_the_shape_n1mm_expects() {
        let a = id(&qso());
        assert_eq!(a.len(), 32, "{a}");
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()), "{a}");
        assert_eq!(a, id(&qso()));
        // A different contact is a different identity.
        let other = QsoRecord { call: "K1ABC".into(), ..qso() };
        assert_ne!(a, id(&other));
    }
}
