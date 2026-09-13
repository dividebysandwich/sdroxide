//! DX-cluster telnet client. One blocking thread connects to a node, logs in
//! with the operator's callsign, and parses `DX de …` spot lines into [`Spot`]s.
//! Modelled on the `sdroxide-tci` net thread: a blocking `TcpStream` with a
//! short read timeout, a `crossbeam` control channel, and a `Drop` that shuts
//! the thread down.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, Sender};
use sdroxide_types::{ClusterConfig, Spot, SpotKind};
use tracing::{debug, warn};

use crate::event::{EventTx, FeedTx, NetEvent};

/// How long a cluster spot stays in the rolling list.
const SPOT_TTL_SECS: i64 = 3600;
/// Cap on retained spots.
const MAX_SPOTS: usize = 400;

enum Ctrl {
    Shutdown,
}

/// A live DX-cluster connection. Dropping it stops the thread.
pub struct ClusterHandle {
    ctrl: Sender<Ctrl>,
    join: Option<JoinHandle<()>>,
}

impl ClusterHandle {
    /// Connect and spawn the reader thread. `login` is the callsign to send at
    /// the node's prompt; `feed`/`events` carry spots and status back.
    pub fn connect(
        cfg: ClusterConfig,
        login: String,
        now_utc: impl Fn() -> i64 + Send + 'static,
        feed: FeedTx,
        events: EventTx,
    ) -> ClusterHandle {
        let (ctrl_tx, ctrl_rx) = crossbeam_channel::unbounded();
        let join = std::thread::Builder::new()
            .name("sdroxide-dxcluster".into())
            .spawn(move || run(cfg, login, now_utc, feed, events, ctrl_rx))
            .ok();
        ClusterHandle { ctrl: ctrl_tx, join }
    }
}

impl Drop for ClusterHandle {
    fn drop(&mut self) {
        let _ = self.ctrl.send(Ctrl::Shutdown);
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }
}

fn run(
    cfg: ClusterConfig,
    login: String,
    now_utc: impl Fn() -> i64,
    feed: FeedTx,
    events: EventTx,
    ctrl: Receiver<Ctrl>,
) {
    let addr = format!("{}:{}", cfg.host.trim(), cfg.port);
    let _ = events.send(NetEvent::Status(Some(format!("Cluster: connecting to {addr}…"))));
    let stream = match TcpStream::connect(&addr) {
        Ok(s) => s,
        Err(e) => {
            let _ = events.send(NetEvent::Status(Some(format!("Cluster: connect failed: {e}"))));
            warn!("dx cluster connect {addr}: {e}");
            return;
        }
    };
    if stream.set_read_timeout(Some(Duration::from_millis(200))).is_err() {
        return;
    }
    let mut stream = stream;

    let mut buf = String::new();
    let mut raw = [0u8; 4096];
    let mut logged_in = false;
    let mut spots: Vec<Spot> = Vec::new();
    let connected_at = Instant::now();
    let _ = events.send(NetEvent::Status(Some(format!("Cluster: connected to {addr}"))));
    // A node sends nothing until it has a callsign, and with none to give it
    // the connection sits at the login prompt looking healthy. Say so where
    // the operator is looking for spots (issue #410).
    if login.trim().is_empty() {
        let _ = events.send(NetEvent::Status(Some(format!(
            "Cluster: connected to {addr}, but no callsign to log in with — set your callsign, \
             or a cluster login, and the node will start sending spots"
        ))));
    }

    loop {
        if matches!(ctrl.try_recv(), Ok(Ctrl::Shutdown)) {
            let _ = stream.write_all(b"bye\r\n");
            let _ = events.send(NetEvent::Status(None));
            return;
        }

        match stream.read(&mut raw) {
            Ok(0) => {
                let _ = events.send(NetEvent::Status(Some("Cluster: disconnected".into())));
                return;
            }
            Ok(n) => buf.push_str(&String::from_utf8_lossy(&raw[..n])),
            Err(e)
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut => {}
            Err(e) => {
                let _ = events.send(NetEvent::Status(Some(format!("Cluster: read error: {e}"))));
                return;
            }
        }

        // Extract complete lines (terminated by \n or \r\n).
        let mut changed = false;
        while let Some(pos) = buf.find('\n') {
            let line: String = buf.drain(..=pos).collect();
            let line = line.trim_end_matches(['\r', '\n']).to_string();
            if !logged_in && wants_login(&line) {
                send_login(&mut stream, &login, &cfg);
                logged_in = true;
                continue;
            }
            if let Some(spot) = parse_dx_line(&line, now_utc()) {
                merge_spot(&mut spots, spot);
                changed = true;
            }
        }

        // A login prompt may arrive without a trailing newline (e.g. "login: ").
        if !logged_in && wants_login(&buf) {
            send_login(&mut stream, &login, &cfg);
            logged_in = true;
            buf.clear();
        }
        // Fallback: some nodes drop you straight to a prompt; if nothing looked
        // like a login prompt within a second, send the call anyway.
        if !logged_in && connected_at.elapsed() > Duration::from_secs(2) {
            send_login(&mut stream, &login, &cfg);
            logged_in = true;
        }

        if changed {
            let now = now_utc();
            spots.retain(|s| now - s.when_utc < SPOT_TTL_SECS);
            if spots.len() > MAX_SPOTS {
                let drop = spots.len() - MAX_SPOTS;
                spots.drain(..drop);
            }
            let _ = feed.send((SpotKind::DxCluster, spots.clone()));
        }
    }
}

pub(crate) fn wants_login(s: &str) -> bool {
    let l = s.to_ascii_lowercase();
    l.contains("login")
        || l.contains("enter your call")
        || l.contains("please enter")
        || l.contains("callsign")
        || l.trim_end().ends_with("call:")
}

fn send_login(stream: &mut TcpStream, login: &str, cfg: &ClusterConfig) {
    send_login_to(stream, login, &cfg.commands, "dx cluster");
}

/// Send the callsign at a node's prompt, then any post-login commands.
///
/// Shared with the RBN reader, which logs in exactly the same way at exactly
/// the same kind of prompt — the two differ in what they do with the spot
/// lines, not in how they get to them.
pub(crate) fn send_login_to(stream: &mut TcpStream, login: &str, commands: &[String], what: &str) {
    let call = login.trim();
    if call.is_empty() {
        warn!("{what}: no login callsign configured");
        return;
    }
    debug!("{what}: logging in as {call}");
    let _ = stream.write_all(format!("{call}\r\n").as_bytes());
    for cmd in commands {
        let c = cmd.trim();
        if !c.is_empty() {
            let _ = stream.write_all(format!("{c}\r\n").as_bytes());
        }
    }
    let _ = stream.flush();
}

/// Insert or update a spot (dedup by id: same call on ~same freq refreshes).
fn merge_spot(spots: &mut Vec<Spot>, spot: Spot) {
    if let Some(existing) = spots.iter_mut().find(|s| s.id == spot.id) {
        *existing = spot;
    } else {
        spots.push(spot);
    }
}

/// Parse a `DX de <spotter>: <freq_khz> <dxcall> <comment...> <time>` line.
///
/// Also the RBN reader's entry point: a skimmer line is this shape exactly
/// (`DX de EA5WU-#: 7018.3 RW1M CW 19 dB 18 WPM CQ 2259Z`), and what that
/// reader needs beyond it — the `-#` marker and the `dB` figure — it takes
/// off the parsed spot rather than by parsing the line a second time.
pub(crate) fn parse_dx_line(line: &str, now: i64) -> Option<Spot> {
    // Cluster nodes append alert BEL bytes (and occasionally other control /
    // non-UTF-8 bytes) to spot lines; map them to spaces so they don't show as
    // tofu and don't glue onto the trailing time token.
    let cleaned: String =
        line.chars().map(|c| if c.is_control() || c == '\u{FFFD}' { ' ' } else { c }).collect();
    let line = cleaned.trim();
    let rest = line.strip_prefix("DX de ").or_else(|| line.strip_prefix("Dx de "))?;
    let (spotter, after) = rest.split_once(':')?;
    let spotter = spotter.trim().to_ascii_uppercase();
    let mut toks = after.split_whitespace();
    let freq_khz: f64 = toks.next()?.parse().ok()?;
    if !(1.0..=1_500_000.0).contains(&freq_khz) {
        return None;
    }
    let call = toks.next()?.trim().to_ascii_uppercase();
    if call.is_empty() {
        return None;
    }
    let mut comment_toks: Vec<&str> = toks.collect();
    // Drop a trailing time token like "1230Z".
    if comment_toks.last().is_some_and(|t| {
        t.len() >= 4 && t.ends_with('Z') && t[..t.len() - 1].chars().all(|c| c.is_ascii_digit())
    }) {
        comment_toks.pop();
    }
    let comment = comment_toks.join(" ");
    let mode = detect_mode(&comment);
    let freq_hz = freq_khz * 1000.0;
    Some(Spot {
        id: Spot::make_id(SpotKind::DxCluster, &call, freq_hz),
        kind: SpotKind::DxCluster,
        freq_hz,
        call,
        spotter,
        mode,
        comment,
        reference: None,
        grid: None,
        loc: None,
        when_utc: now,
        snr_db: None,
    })
}

/// Pull a mode token out of a spot comment, if present.
fn detect_mode(comment: &str) -> String {
    const MODES: &[&str] = &[
        "FT8", "FT4", "FT2", "CW", "SSB", "USB", "LSB", "RTTY", "PSK", "PSK31", "JT65", "JT9",
        "FM", "AM", "SSTV", "OLIVIA", "MFSK", "JS8", "WSPR",
    ];
    for tok in comment.split_whitespace() {
        let up = tok.trim_matches(|c: char| !c.is_ascii_alphanumeric()).to_ascii_uppercase();
        if MODES.contains(&up.as_str()) {
            return up;
        }
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_standard_spot() {
        let s =
            parse_dx_line("DX de W3LPL:     14025.0  DL9XYZ       CW  loud   1230Z", 100).unwrap();
        assert_eq!(s.call, "DL9XYZ");
        assert_eq!(s.spotter, "W3LPL");
        assert_eq!(s.freq_hz, 14_025_000.0);
        assert_eq!(s.mode, "CW");
        assert!(!s.comment.contains("1230Z"));
    }

    #[test]
    fn rejects_non_spot_lines() {
        assert!(parse_dx_line("Hello and welcome to the cluster", 0).is_none());
        assert!(parse_dx_line("DX de N0CALL: notafreq K1ABC", 0).is_none());
    }

    #[test]
    fn detects_ft8() {
        let s = parse_dx_line("DX de EA1AB:      7074.0  JA1ZZZ  FT8 -12dB  2359Z", 0).unwrap();
        assert_eq!(s.mode, "FT8");
        assert_eq!(s.freq_hz, 7_074_000.0);
    }

    #[test]
    fn strips_trailing_bell_and_time() {
        // Real cluster spots end with two BEL alert bytes after the time.
        let s = parse_dx_line("DX de IK5ABC:  7123.0  IU1LSY  leone di creta/ 1421Z\u{7}\u{7}", 0)
            .unwrap();
        assert_eq!(s.call, "IU1LSY");
        assert_eq!(s.comment, "leone di creta/");
        assert!(!s.comment.contains('\u{7}'));
        assert!(!s.comment.contains("1421Z"));
    }
}
