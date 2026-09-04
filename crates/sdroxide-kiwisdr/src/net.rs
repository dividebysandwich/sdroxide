//! The two sockets, and the threads that own them.
//!
//! A KiwiSDR session is two WebSockets to the same host, distinguished only by
//! the last element of the path — `/<timestamp>/SND` and `/<timestamp>/W/F`.
//! The timestamp is the client's own and groups them into one session on the
//! receiver's side; anything unique will do, and the browser uses the epoch
//! milliseconds it happened to open at.
//!
//! Both threads use the same shape as the TCI client's: a short read timeout on
//! the underlying `TcpStream`, so a blocking `read` becomes a poll, and the
//! control channel is served between reads rather than from another thread
//! holding a lock on the socket.

use std::io::ErrorKind;
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicI64, AtomicU8, AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use rtrb::RingBuffer;
use sdroxide_types::KiwiConfig;
use tungstenite::{Message, WebSocket, handshake::HandshakeError};

use crate::error::{Error, Result};
use crate::handle::{Ctrl, KiwiHandle, KiwiInfo, Pending, Shared, WaterfallFrame, push_iq};
use crate::proto as p;

/// Ring depth for the I/Q lane: about a second and a half at 12 kHz complex.
/// Generous because the engine's read cadence is set by its audio block and not
/// by this receiver's frame rate, and because a network hiccup that costs
/// samples cannot be recovered.
const RING_SLOTS: usize = 1 << 15;

/// How long a blocking read waits before the loop gets a turn at the control
/// channel. Short enough that a dial drag feels immediate.
const POLL: Duration = Duration::from_millis(20);

/// Longest the opening exchange may take before it is called a failure.
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);

/// Sent this often so the receiver does not drop an idle socket. Note this is
/// *not* what its inactivity timer watches — that resets on control changes —
/// so a receiver with one set will still eventually disconnect a parked
/// listener, which is its operator's right.
const KEEPALIVE: Duration = Duration::from_secs(5);

/// The dB window the receiver quantises its waterfall into. Wide enough to hold
/// a strong broadcast band and a quiet one at once; the engine applies its own
/// display range on top.
const WF_MAX_DB: i32 = -10;
const WF_MIN_DB: i32 = -134;

/// Open a session: connect the audio socket, configure it, and start both
/// threads.
///
/// Blocks until the receiver has accepted the session and stated its rate, so a
/// wrong address, a full receiver or a wrong password comes back as an ordinary
/// error rather than as a stream that never starts.
pub fn spawn(cfg: &KiwiConfig, ident: &str, center_hz: f64) -> Result<KiwiHandle> {
    let endpoint = cfg.endpoint();
    let (host, port) = split_addr(&endpoint)?;
    // One timestamp for both sockets: that is what makes them one session.
    let session =
        SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0);

    let mut snd = connect(&host, port, &format!("/{session}/SND"))?;
    let info = audio_handshake(&mut snd, cfg, ident, center_hz)?;

    let shared = Arc::new(Shared {
        alive: AtomicBool::new(true),
        refusal: std::sync::Mutex::new(None),
        last_rx_ms: AtomicU64::new(0),
        wf: std::sync::Mutex::new(None),
        smeter_centi_dbm: AtomicI32::new(-12_700),
        adc_overflow: AtomicBool::new(false),
        center_milli_hz: AtomicI64::new((center_hz * 1000.0) as i64),
        wf_speed: AtomicU8::new(cfg.wf_speed.clamp(1, 4)),
        wf_zoom: AtomicU8::new(cfg.wf_zoom.min(sdroxide_types::KiwiConfig::WF_ZOOM_MAX)),
        stop: AtomicBool::new(false),
    });

    let (prod, cons) = RingBuffer::<f32>::new(RING_SLOTS);
    let (ctrl_tx, ctrl_rx) = crossbeam_channel::unbounded::<Ctrl>();

    let label = format!("KiwiSDR {endpoint}");
    let mut threads = Vec::new();
    {
        let shared = Arc::clone(&shared);
        let wf_cal = info.wf_cal;
        threads.push(
            std::thread::Builder::new()
                .name("kiwi-snd".into())
                .spawn(move || audio_thread(snd, prod, ctrl_rx, shared, wf_cal))
                .map_err(|e| Error::Net(format!("could not start the audio thread: {e}")))?,
        );
    }

    if cfg.wide_lane {
        let shared = Arc::clone(&shared);
        let (host, info2) = (host.clone(), info.clone());
        // The band view is a nicety compared with the receiver itself, so this
        // socket opens on its own thread and a failure costs the strip rather
        // than the radio.
        match std::thread::Builder::new()
            .name("kiwi-wf".into())
            .spawn(move || waterfall_thread(&host, port, session, shared, info2))
        {
            Ok(t) => threads.push(t),
            Err(e) => tracing::warn!("could not start the KiwiSDR waterfall thread: {e}"),
        }
    }

    Ok(KiwiHandle::from_parts(cons, ctrl_tx, shared, threads, info, label))
}

/// Connect one socket and upgrade it, with a bounded handshake.
fn connect(host: &str, port: u16, path: &str) -> Result<WebSocket<TcpStream>> {
    let sockaddr = (host, port)
        .to_socket_addrs()
        .map_err(|e| Error::Net(format!("resolve {host}:{port}: {e}")))?
        .next()
        .ok_or_else(|| Error::Net(format!("no address for {host}:{port}")))?;
    let stream = TcpStream::connect_timeout(&sockaddr, Duration::from_secs(6))
        .map_err(|e| Error::Net(format!("connect {host}:{port}: {e}")))?;
    // The upgrade gets a generous timeout of its own, and only the finished
    // WebSocket gets the short streaming poll. Windows reports a receive
    // timeout as `WSAETIMEDOUT` rather than the `EAGAIN` a Unix gives, and
    // tungstenite resumes an interrupted handshake for `WouldBlock` alone —
    // anything else, `TimedOut` included, is a failure it cannot come back
    // from. With the poll set here first, every receiver more than 20 ms away
    // failed to open on Windows and only on Windows (issue #266).
    stream
        .set_read_timeout(Some(HANDSHAKE_TIMEOUT))
        .map_err(|e| Error::Net(format!("set read timeout: {e}")))?;
    let url = format!("ws://{host}:{port}{path}");

    let deadline = Instant::now() + HANDSHAKE_TIMEOUT;
    let mut attempt = tungstenite::client(url.as_str(), stream);
    let ws = loop {
        match attempt {
            Ok((ws, _resp)) => break ws,
            Err(HandshakeError::Interrupted(mid)) => {
                if Instant::now() > deadline {
                    return Err(Error::Net("timed out during the WebSocket handshake".into()));
                }
                attempt = mid.handshake();
            }
            Err(e) => {
                let msg = e.to_string();
                // Nearly half the public receivers are reached through the
                // project's own reverse proxy, and that is what it answers for
                // one whose owner has switched it off. "404" would send an
                // operator hunting for a wrong address.
                if host.contains("proxy.kiwisdr.com") && msg.contains("404") {
                    return Err(Error::Refused(
                        "this receiver is listed but not currently online".into(),
                    ));
                }
                return Err(Error::Net(format!("WebSocket handshake failed: {msg}")));
            }
        }
    };
    ws.get_ref()
        .set_read_timeout(Some(POLL))
        .map_err(|e| Error::Net(format!("set read timeout: {e}")))?;
    Ok(ws)
}

/// Send one `SET` line, ignoring a socket that has already gone: the read side
/// notices first and reports it properly.
fn send(ws: &mut WebSocket<TcpStream>, line: impl AsRef<str>) {
    let _ = ws.send(Message::Text(line.as_ref().into()));
    let _ = ws.flush();
}

/// Read one message, or `Ok(None)` when the poll simply expired.
fn poll_read(ws: &mut WebSocket<TcpStream>) -> Result<Option<Vec<u8>>> {
    match ws.read() {
        Ok(Message::Binary(b)) => Ok(Some(b.into())),
        Ok(Message::Text(t)) => Ok(Some(t.as_bytes().to_vec())),
        Ok(Message::Close(_)) => Err(Error::Net("the receiver closed the connection".into())),
        Ok(_) => Ok(None),
        Err(tungstenite::Error::Io(e))
            if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut =>
        {
            Ok(None)
        }
        Err(tungstenite::Error::ConnectionClosed | tungstenite::Error::AlreadyClosed) => {
            Err(Error::Net("the receiver closed the connection".into()))
        }
        Err(e) => Err(Error::Net(format!("read: {e}"))),
    }
}

/// Authenticate, wait for the receiver to describe itself, and start the I/Q.
///
/// The order matters and is the receiver's, not ours: nothing is sent until it
/// has answered `auth`, and it sends no samples until it has been given both a
/// mode and an `AR OK`.
fn audio_handshake(
    ws: &mut WebSocket<TcpStream>,
    cfg: &KiwiConfig,
    ident: &str,
    center_hz: f64,
) -> Result<KiwiInfo> {
    send(ws, p::set_auth(cfg.password.trim()));
    send(ws, p::set_ident(ident));

    let mut info = KiwiInfo::default();
    let deadline = Instant::now() + HANDSHAKE_TIMEOUT;
    let mut configured = false;
    loop {
        if Instant::now() > deadline {
            return Err(Error::Net(
                "the receiver accepted the connection but never started the stream".into(),
            ));
        }
        let Some(msg) = poll_read(ws)? else { continue };
        match p::split(&msg) {
            p::Frame::Msg(text) => {
                if let Some(r) = refusal_in(text) {
                    return Err(Error::Refused(r));
                }
                let mut ready = false;
                for (k, v) in p::msg_params(text) {
                    match k {
                        "sample_rate" => info.sample_rate_hz = v.parse().unwrap_or(0.0),
                        "center_freq" => info.center_hz = v.parse().unwrap_or(0.0),
                        "bandwidth" => info.bandwidth_hz = v.parse().unwrap_or(0.0),
                        "version_maj" => info.version.0 = v.parse().unwrap_or(0),
                        "version_min" => info.version.1 = v.parse().unwrap_or(0),
                        "rx_chans" => info.rx_chans = v.parse().unwrap_or(0),
                        "wf_cal" => info.wf_cal = v.parse().unwrap_or(0),
                        "audio_init" => ready = true,
                        "keepalive" => send(ws, p::set_keepalive()),
                        _ => {}
                    }
                }
                if ready && !configured {
                    if info.sample_rate_hz <= 0.0 {
                        return Err(Error::Proto(
                            "the receiver started the stream without stating its sample rate"
                                .into(),
                        ));
                    }
                    send(ws, p::set_mod_iq(center_hz / 1e3));
                    send(ws, p::set_agc(cfg.agc, cfg.man_gain));
                    // Linear samples: the I/Q path has no ADPCM decoder, and it
                    // does not need one — the receiver never compresses a
                    // stereo stream.
                    send(ws, p::set_compression(false));
                    send(ws, p::set_ar_ok(info.sample_rate_hz.round() as u32));
                    configured = true;
                }
            }
            // The first frame of I/Q is the proof the session is up. Waiting
            // for it rather than for the acknowledgement is what makes a
            // failure to open a failure rather than a silent stream.
            p::Frame::Snd(body) if configured => {
                let mut scratch = Vec::new();
                p::decode_snd_iq(body, &mut scratch).map_err(Error::Proto)?;
                return Ok(info);
            }
            _ => {}
        }
    }
}

/// A receiver's own words for declining, or `None`.
///
/// Read out of the `MSG` stream rather than inferred from a closed socket,
/// because the difference decides whether anything reconnects.
fn refusal_in(text: &str) -> Option<String> {
    for (k, v) in p::msg_params(text) {
        match k {
            "badp" if v != "0" => {
                return Some("the receiver wants a password, or the one given was wrong".into());
            }
            "too_busy" => {
                return Some("every channel on this receiver is in use".into());
            }
            // The receiver's inactivity limit, its 24-hour per-address limit,
            // or its operator pressing the kick button. All three are the
            // receiver saying no, and none of them is a reason to come back
            // immediately.
            "down" | "kick" | "inactivity_timeout" | "ip_limit" => {
                return Some(format!("the receiver ended the session ({k})"));
            }
            _ => {}
        }
    }
    None
}

/// Own the audio socket: decode I/Q, serve the dial, keep the session alive.
fn audio_thread(
    mut ws: WebSocket<TcpStream>,
    mut ring: rtrb::Producer<f32>,
    ctrl: crossbeam_channel::Receiver<Ctrl>,
    shared: Arc<Shared>,
    _wf_cal: i32,
) {
    let started = Instant::now();
    let mut iq = Vec::with_capacity(2048);
    let mut last_keepalive = Instant::now();
    let mut dropped: u64 = 0;
    let mut last_report = Instant::now();

    loop {
        if shared.stop.load(Ordering::Relaxed) {
            break;
        }

        // Collapse the whole control channel before touching the socket, so a
        // dial drag costs one retune rather than a hundred.
        let mut pending = Pending::default();
        while let Ok(c) = ctrl.try_recv() {
            pending.absorb(c);
        }
        if pending.shutdown {
            break;
        }
        if !pending.is_empty() {
            if let Some(hz) = pending.center {
                send(&mut ws, p::set_mod_iq(hz / 1e3));
                shared.center_milli_hz.store((hz * 1000.0) as i64, Ordering::Relaxed);
            }
            if let Some((on, gain)) = pending.agc {
                send(&mut ws, p::set_agc(on, gain));
            }
        }

        if last_keepalive.elapsed() >= KEEPALIVE {
            send(&mut ws, p::set_keepalive());
            last_keepalive = Instant::now();
        }

        match poll_read(&mut ws) {
            Ok(None) => continue,
            Ok(Some(msg)) => match p::split(&msg) {
                p::Frame::Snd(body) => {
                    iq.clear();
                    match p::decode_snd_iq(body, &mut iq) {
                        Ok(h) => {
                            shared
                                .smeter_centi_dbm
                                .store((h.smeter_dbm * 100.0) as i32, Ordering::Relaxed);
                            shared.adc_overflow.store(h.adc_overflow, Ordering::Relaxed);
                            if !push_iq(&mut ring, &iq) {
                                dropped += iq.len() as u64 / 2;
                            }
                            shared
                                .last_rx_ms
                                .store(started.elapsed().as_millis() as u64, Ordering::Relaxed);
                        }
                        Err(e) => {
                            tracing::warn!("KiwiSDR audio frame: {e}");
                            break;
                        }
                    }
                }
                p::Frame::Msg(text) => {
                    if let Some(r) = refusal_in(text) {
                        tracing::info!("KiwiSDR session ended: {r}");
                        if let Ok(mut g) = shared.refusal.lock() {
                            *g = Some(r);
                        }
                        break;
                    }
                    for (k, _) in p::msg_params(text) {
                        if k == "keepalive" {
                            send(&mut ws, p::set_keepalive());
                            last_keepalive = Instant::now();
                        }
                    }
                }
                _ => {}
            },
            Err(e) => {
                tracing::info!("KiwiSDR audio link: {e}");
                break;
            }
        }

        if dropped > 0 && last_report.elapsed() >= Duration::from_secs(5) {
            tracing::warn!("KiwiSDR: {dropped} I/Q samples dropped (the engine is behind)");
            dropped = 0;
            last_report = Instant::now();
        }
    }

    // Close politely rather than dropping the socket: a KiwiSDR has four or
    // eight channels and this frees ours at once instead of at its timeout.
    let _ = ws.close(None);
    let _ = ws.flush();
    shared.alive.store(false, Ordering::Relaxed);
}

/// The zoom and centre the waterfall should be on now, or `None` when it is
/// already there.
///
/// At zoom 0 the window *is* the band and the centre is the band's own, so
/// tuning never moves it — which is the ordinary case and must cost nothing.
/// Zoomed in it follows the dial, but only once the dial has left the middle
/// half of what is on screen: a `SET zoom` per hertz of a drag would be a
/// command per frame at the receiver, and a window that jumped under the
/// operator's hand at every one of them.
fn wanted_window(shared: &Shared, info: &KiwiInfo, have: (u8, f64)) -> Option<(u8, f64)> {
    let zoom = shared.wf_zoom.load(Ordering::Relaxed).min(KiwiConfig::WF_ZOOM_MAX);
    if zoom == 0 {
        let cf = info.center_hz / 1e3;
        return (have != (0, cf)).then_some((0, cf));
    }
    let span = info.bandwidth_hz / f64::from(1u32 << zoom);
    let dial = shared.center_milli_hz.load(Ordering::Relaxed) as f64 / 1000.0;
    // The receiver clamps a window that would reach past a band edge, so clamp
    // it here too and the two agree about what was asked for.
    let (lo, hi) =
        (info.center_hz - info.bandwidth_hz / 2.0, info.center_hz + info.bandwidth_hz / 2.0);
    let cf = dial.clamp(lo + span / 2.0, hi - span / 2.0) / 1e3;
    if have.0 == zoom && (have.1 - cf).abs() * 1e3 < span / 4.0 {
        return None;
    }
    Some((zoom, cf))
}

/// Own the waterfall socket: one frame at a time into the slot.
fn waterfall_thread(host: &str, port: u16, session: u64, shared: Arc<Shared>, info: KiwiInfo) {
    let mut ws = match connect(host, port, &format!("/{session}/W/F")) {
        Ok(ws) => ws,
        Err(e) => {
            // The receiver still works; only its band view is missing.
            tracing::warn!("KiwiSDR waterfall unavailable: {e}");
            return;
        }
    };
    send(&mut ws, p::set_auth(""));
    send(&mut ws, p::set_ident("sdroxide"));

    let mut configured = false;
    let mut speed = 0u8;
    // The window last asked for, as (zoom, centre kHz). A zoomed waterfall has
    // to follow the dial, and the dial moves whenever the operator tunes.
    let mut window = (u8::MAX, f64::NAN);
    let mut bins = Vec::with_capacity(p::WF_WIDTH);
    let mut last_keepalive = Instant::now();

    loop {
        if shared.stop.load(Ordering::Relaxed) {
            break;
        }
        let want = shared.wf_speed.load(Ordering::Relaxed);
        if configured && want != speed {
            send(&mut ws, p::set_wf_speed(want));
            speed = want;
        }
        if configured && let Some(w) = wanted_window(&shared, &info, window) {
            send(&mut ws, p::set_zoom_cf(u32::from(w.0), w.1));
            window = w;
        }
        if last_keepalive.elapsed() >= KEEPALIVE {
            send(&mut ws, p::set_keepalive());
            last_keepalive = Instant::now();
        }

        match poll_read(&mut ws) {
            Ok(None) => continue,
            Ok(Some(msg)) => match p::split(&msg) {
                p::Frame::Msg(text) => {
                    if refusal_in(text).is_some() {
                        break;
                    }
                    for (k, _) in p::msg_params(text) {
                        if k == "wf_setup" && !configured {
                            // Zoom 0 is the receiver's whole band, which is what
                            // this lane is for: showing what the 12 kHz of I/Q
                            // cannot. An operator who wants a real waterfall of
                            // a few hundred kilohertz instead asks for one, and
                            // the window then follows the dial (issue #303).
                            let w = wanted_window(&shared, &info, window)
                                .unwrap_or((0, info.center_hz / 1e3));
                            send(&mut ws, p::set_zoom_cf(u32::from(w.0), w.1));
                            window = w;
                            send(&mut ws, p::set_maxdb_mindb(WF_MAX_DB, WF_MIN_DB));
                            send(&mut ws, p::set_wf_comp(false));
                            send(&mut ws, p::set_interp(false));
                            speed = want;
                            send(&mut ws, p::set_wf_speed(speed));
                            configured = true;
                        }
                    }
                }
                p::Frame::Wf(body) => {
                    bins.clear();
                    match p::decode_wf(body, info.wf_cal, &mut bins) {
                        Ok(h) => {
                            // Where the frame says it is, not where the last
                            // request asked for: the receiver clamps a window
                            // that would hang off a band edge, and a frame
                            // already in flight when the window moves still
                            // describes where it was.
                            let (center_hz, span_hz) =
                                p::wf_window(&h, info.center_hz, info.bandwidth_hz);
                            if let Ok(mut slot) = shared.wf.lock() {
                                // Newest wins: an unread frame is overwritten
                                // rather than queued.
                                *slot =
                                    Some(WaterfallFrame { center_hz, span_hz, bins: bins.clone() });
                            }
                        }
                        Err(e) => {
                            tracing::warn!("KiwiSDR waterfall frame: {e}");
                            break;
                        }
                    }
                }
                _ => {}
            },
            Err(e) => {
                tracing::debug!("KiwiSDR waterfall link: {e}");
                break;
            }
        }
    }
    let _ = ws.close(None);
    let _ = ws.flush();
}

/// Split `host:port`, defaulting the port. Handles a bracketed IPv6 literal.
pub(crate) fn split_addr(address: &str) -> Result<(String, u16)> {
    let a = address.trim();
    let a = a.strip_prefix("ws://").or_else(|| a.strip_prefix("http://")).unwrap_or(a);
    let a = a.trim_end_matches('/');
    if a.is_empty() {
        return Err(Error::Net("no address given".into()));
    }
    if let Some(close) = a.rfind(']') {
        let host = a[..=close].to_string();
        return match a[close + 1..].strip_prefix(':') {
            Some(port) => Ok((
                host,
                port.parse().map_err(|_| Error::Net(format!("invalid port in {address:?}")))?,
            )),
            None => Ok((host, KiwiConfig::DEFAULT_PORT)),
        };
    }
    match a.rsplit_once(':') {
        Some((host, port)) if !host.is_empty() => Ok((
            host.to_string(),
            port.parse().map_err(|_| Error::Net(format!("invalid port in {address:?}")))?,
        )),
        _ => Ok((a.to_string(), KiwiConfig::DEFAULT_PORT)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addresses_split_the_way_the_directory_writes_them() {
        assert_eq!(
            split_addr("porky.dahlbros.net:8073").unwrap(),
            ("porky.dahlbros.net".into(), 8073)
        );
        // The directory always writes a port, but an operator typing one by
        // hand will not.
        assert_eq!(split_addr("kiwi.local").unwrap(), ("kiwi.local".into(), 8073));
        assert_eq!(
            split_addr("22033.proxy.kiwisdr.com:80").unwrap(),
            ("22033.proxy.kiwisdr.com".into(), 80)
        );
        assert_eq!(split_addr("http://kiwi.local:8074/").unwrap(), ("kiwi.local".into(), 8074));
        assert_eq!(split_addr("[2001:db8::1]:8073").unwrap(), ("[2001:db8::1]".into(), 8073));
        assert_eq!(split_addr("[2001:db8::1]").unwrap(), ("[2001:db8::1]".into(), 8073));
        assert!(split_addr("").is_err());
        assert!(split_addr("kiwi.local:not-a-port").is_err());
    }

    #[test]
    fn a_refusal_is_recognised_from_the_receivers_own_words() {
        assert!(refusal_in("badp=1").is_some());
        assert!(refusal_in("too_busy").is_some());
        assert!(refusal_in("inactivity_timeout").is_some());
        // The ordinary case: a receiver that is perfectly happy.
        assert_eq!(refusal_in("badp=0 audio_init=0 sample_rate=11998.87"), None);
    }
}
