//! Upload decoded radiosonde frames to SondeHub (`api.v2.sondehub.org`).
//!
//! Mirrors OpenWebRX+ `owrx/reporting/sondehub.py`:
//! - gzip PUT of JSON arrays to `/sondes/telemetry`
//! - separate DFM buffer (30 s, ≥10 packets) vs standard (15 s)
//! - dedupe `(serial, frame)`
//! - retries on 5xx
//! - `/listeners` heartbeat every 6 hours
//!
//! Callsign and grid come from the digi identity the engine already shares
//! with WSPRnet.

use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, Sender, unbounded};
use sdroxide_types::Rs41Sonde;
use tracing::{debug, info, warn};

use crate::event::EventTx;

const DEFAULT_BASE: &str = "https://api.v2.sondehub.org";
const MAX_PENDING: usize = 500;
const STANDARD_FLUSH: Duration = Duration::from_secs(15);
const DFM_FLUSH: Duration = Duration::from_secs(30);
const DFM_MIN_PACKETS: usize = 10;
const UPLOAD_RETRIES: u32 = 5;
const LISTENER_INTERVAL: Duration = Duration::from_secs(3600 * 6);
const FAREWELL: Duration = Duration::from_secs(10);
const TICK: Duration = Duration::from_millis(200);

/// Who is doing the hearing.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Reporter {
    pub call: String,
    pub grid: String,
    pub antenna: String,
}

/// One reception ready to upload.
#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub sonde: Rs41Sonde,
}

pub struct SondeHubUploadHandle {
    tx: Sender<Item>,
    stopping: Arc<AtomicBool>,
}

impl Drop for SondeHubUploadHandle {
    fn drop(&mut self) {
        self.stopping.store(true, Ordering::Relaxed);
    }
}

impl SondeHubUploadHandle {
    pub fn report(&self, item: Item) {
        let _ = self.tx.send(item);
    }
}

/// Start telemetry + listener uploaders. `endpoint` empty means the public API.
pub fn spawn_upload(rx: Reporter, endpoint: String, _events: EventTx) -> SondeHubUploadHandle {
    let (tx, queue_rx) = unbounded::<Item>();
    let stopping = Arc::new(AtomicBool::new(false));
    let stop = Arc::clone(&stopping);
    let base = normalize_base(&endpoint);
    let telem_url = format!("{base}/sondes/telemetry");
    let listen_url = format!("{base}/listeners");
    info!(
        call = %rx.call,
        grid = %rx.grid,
        %telem_url,
        %listen_url,
        "SondeHub uploaders starting"
    );

    let rx_telem = rx.clone();
    let stop_telem = Arc::clone(&stop);
    std::thread::Builder::new()
        .name("sdroxide-sondehub".into())
        .spawn(move || telemetry_worker(rx_telem, telem_url, queue_rx, stop_telem))
        .expect("spawn sondehub upload");

    let rx_listen = rx;
    let stop_listen = Arc::clone(&stopping);
    std::thread::Builder::new()
        .name("sdroxide-sondehub-lsn".into())
        .spawn(move || listener_worker(rx_listen, listen_url, stop_listen))
        .expect("spawn sondehub listener");

    SondeHubUploadHandle { tx, stopping }
}

fn normalize_base(endpoint: &str) -> String {
    let e = endpoint.trim().trim_end_matches('/');
    if e.is_empty() {
        return DEFAULT_BASE.to_string();
    }
    if let Some(stripped) = e.strip_suffix("/sondes/telemetry") {
        return stripped.trim_end_matches('/').to_string();
    }
    if let Some(stripped) = e.strip_suffix("/listeners") {
        return stripped.trim_end_matches('/').to_string();
    }
    e.to_string()
}

fn telemetry_worker(rx: Reporter, url: String, queue: Receiver<Item>, stopping: Arc<AtomicBool>) {
    let mut standard: VecDeque<serde_json::Value> = VecDeque::new();
    let mut dfm: VecDeque<serde_json::Value> = VecDeque::new();
    let mut standard_seen: HashSet<(String, u32)> = HashSet::new();
    let mut dfm_seen: HashSet<(String, u32)> = HashSet::new();
    let mut next_standard = Instant::now() + STANDARD_FLUSH;
    let mut next_dfm = Instant::now() + DFM_FLUSH;
    let mut farewell_at: Option<Instant> = None;
    debug!(%url, "SondeHub telemetry worker started");

    loop {
        match queue.recv_timeout(TICK) {
            Ok(item) => {
                let serial = item.sonde.serial.clone();
                let kind = item.sonde.kind.label();
                if let Some(entry) = telemetry_json(&rx, &item.sonde) {
                    let is_dfm = item.sonde.kind.is_dfm()
                        || entry.get("type").and_then(|v| v.as_str()) == Some("DFM");
                    let key = frame_key(&entry);
                    let (buf, seen) = if is_dfm {
                        (&mut dfm, &mut dfm_seen)
                    } else {
                        (&mut standard, &mut standard_seen)
                    };
                    if let Some(ref k) = key {
                        if seen.contains(k) {
                            debug!(
                                %serial,
                                frame = k.1,
                                "SondeHub skip duplicate (serial, frame)"
                            );
                            continue;
                        }
                        seen.insert(k.clone());
                    }
                    if buf.len() >= MAX_PENDING {
                        buf.pop_front();
                    }
                    buf.push_back(entry);
                    debug!(
                        %serial,
                        kind,
                        is_dfm,
                        queued = buf.len(),
                        "SondeHub queued telemetry"
                    );
                } else {
                    debug!(
                        %serial,
                        kind,
                        call_empty = rx.call.trim().is_empty(),
                        has_pos = item.sonde.has_position(),
                        "SondeHub skip — no publishable payload"
                    );
                }
            }
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {}
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
        }

        if stopping.load(Ordering::Relaxed) {
            farewell_at.get_or_insert_with(Instant::now);
        }
        if let Some(t) = farewell_at
            && t.elapsed() >= FAREWELL
        {
            break;
        }

        let now = Instant::now();
        if now >= next_standard {
            if !standard.is_empty() {
                let batch: Vec<_> = standard.drain(..).collect();
                standard_seen.clear();
                debug!(n = batch.len(), %url, "SondeHub publishing standard batch");
                upload_batch(&url, &batch, "standard");
            }
            next_standard = now + STANDARD_FLUSH;
        }
        if now >= next_dfm {
            if dfm.len() >= DFM_MIN_PACKETS {
                let batch: Vec<_> = dfm.drain(..).collect();
                dfm_seen.clear();
                debug!(n = batch.len(), %url, "SondeHub publishing dfm batch");
                upload_batch(&url, &batch, "dfm");
            } else if !dfm.is_empty() {
                debug!(
                    queued = dfm.len(),
                    need = DFM_MIN_PACKETS,
                    "SondeHub dfm flush deferred — below packet threshold"
                );
            }
            next_dfm = now + DFM_FLUSH;
        }

        if farewell_at.is_some() {
            if !standard.is_empty() {
                let batch: Vec<_> = standard.drain(..).collect();
                debug!(n = batch.len(), "SondeHub farewell standard flush");
                upload_batch(&url, &batch, "standard");
            }
            if dfm.len() >= DFM_MIN_PACKETS {
                let batch: Vec<_> = dfm.drain(..).collect();
                debug!(n = batch.len(), "SondeHub farewell dfm flush");
                upload_batch(&url, &batch, "dfm");
            }
            break;
        }
    }
    debug!("SondeHub telemetry worker stopped");
}

fn listener_worker(rx: Reporter, url: String, stopping: Arc<AtomicBool>) {
    debug!(%url, "SondeHub listener worker started");
    // Upload once soon after start, then every 6 hours.
    let mut next = Instant::now() + Duration::from_secs(5);
    while !stopping.load(Ordering::Relaxed) {
        if Instant::now() >= next {
            debug!(%url, call = %rx.call, "SondeHub publishing listener");
            match upload_listener(&rx, &url) {
                Ok(()) => info!(%url, "SondeHub listener uploaded"),
                Err(e) => warn!(%e, "SondeHub listener upload failed"),
            }
            next = Instant::now() + LISTENER_INTERVAL;
        }
        std::thread::sleep(Duration::from_secs(1));
    }
    debug!("SondeHub listener worker stopped");
}

/// Pull a compact error summary out of a SondeHub 2xx body, if any.
fn sondehub_response_errors(body: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(body).ok()?;
    let errs = v.get("errors")?.as_array()?;
    if errs.is_empty() {
        return None;
    }
    let msgs: Vec<String> = errs
        .iter()
        .filter_map(|e| e.get("error_message").and_then(|m| m.as_str()).map(str::to_string))
        .collect();
    if msgs.is_empty() {
        Some(format!("{} error(s)", errs.len()))
    } else {
        Some(msgs.join("; "))
    }
}

fn frame_key(entry: &serde_json::Value) -> Option<(String, u32)> {
    let serial = entry.get("serial")?.as_str()?.to_string();
    let frame =
        entry.get("frame")?.as_u64().or_else(|| entry.get("frame")?.as_i64().map(|n| n as u64))?
            as u32;
    Some((serial, frame))
}

fn upload_batch(url: &str, batch: &[serde_json::Value], batch_type: &str) {
    if batch.is_empty() {
        return;
    }
    let serials: Vec<&str> = batch
        .iter()
        .filter_map(|v| v.get("serial").and_then(|s| s.as_str()))
        .collect();
    let body = serde_json::Value::Array(batch.to_vec()).to_string();
    for attempt in 1..=UPLOAD_RETRIES {
        debug!(
            n = batch.len(),
            batch_type,
            attempt,
            %url,
            serials = ?serials,
            "SondeHub PUT /sondes/telemetry"
        );
        match crate::http::put_gzip_json(url, &body) {
            Ok((status, text)) if (200..300).contains(&status) => {
                // SondeHub often answers 202 Accepted even when every object
                // was rejected (unknown software_name, bad serial, …). The
                // real answer is in `errors`.
                if let Some(errs) = sondehub_response_errors(&text) {
                    warn!(
                        n = batch.len(),
                        batch_type,
                        status,
                        serials = ?serials,
                        errors = %errs,
                        body = %text.chars().take(200).collect::<String>(),
                        "SondeHub rejected telemetry (HTTP {status} with errors) — \
                         new software_name values are blocked until allowlisted; \
                         see https://github.com/projecthorus/sondehub-infra/wiki/Sonde-Telemetry-Validation"
                    );
                    return;
                }
                info!(
                    n = batch.len(),
                    batch_type,
                    status,
                    serials = ?serials,
                    body = %text.chars().take(120).collect::<String>(),
                    "SondeHub upload ok"
                );
                return;
            }
            Ok((status, text)) if (500..600).contains(&status) && attempt < UPLOAD_RETRIES => {
                warn!(
                    attempt,
                    status,
                    batch_type,
                    body = %text.chars().take(120).collect::<String>(),
                    "SondeHub upload server error, retrying"
                );
                std::thread::sleep(Duration::from_secs(1));
            }
            Ok((status, text)) => {
                warn!(
                    n = batch.len(),
                    batch_type,
                    status,
                    body = %text.chars().take(200).collect::<String>(),
                    "SondeHub upload failed"
                );
                return;
            }
            Err(e) => {
                warn!(%e, n = batch.len(), batch_type, attempt, "SondeHub upload failed");
                if attempt < UPLOAD_RETRIES {
                    std::thread::sleep(Duration::from_secs(1));
                } else {
                    return;
                }
            }
        }
    }
}

fn upload_listener(rx: &Reporter, url: &str) -> Result<(), String> {
    let call = rx.call.trim().to_uppercase();
    if call.is_empty() {
        return Err("empty callsign".into());
    }
    let Some((lat, lon)) = grid_home(&rx.grid) else {
        return Err("configure My grid for SondeHub listener position".into());
    };
    let payload = serde_json::json!({
        "software_name": "radiosonde_auto_rx",
        "software_version": env!("CARGO_PKG_VERSION"),
        "uploader_callsign": call,
        "uploader_position": [lat, lon, 0.0],
        "uploader_radio": "radiosonde_auto_rx",
        "uploader_antenna": rx.antenna.trim(),
        "mobile": false,
    });
    crate::http::put_json(url, &payload.to_string())?;
    Ok(())
}

/// Build one SondeHub telemetry object, or `None` when the sonde has no fix yet.
pub fn telemetry_json(rx: &Reporter, s: &Rs41Sonde) -> Option<serde_json::Value> {
    let (lat, lon) = (s.lat?, s.lon?);
    let alt = f64::from(s.altitude_m.unwrap_or(0.0));
    let call = rx.call.trim().to_uppercase();
    if call.is_empty() || s.serial.is_empty() {
        return None;
    }
    if s.kind.is_dfm() && s.serial.eq_ignore_ascii_case("xxxxxxxx") {
        return None;
    }

    let received = rfc3339(s.last_at);
    // SondeHub wants the sonde's own timestamp in `datetime` (GPS/as-broadcast),
    // not wall-clock receive time. Fall back to receive time only when *mod
    // did not emit one.
    let datetime = s
        .datetime
        .as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| received.clone());
    let manufacturer = if s.manufacturer.is_empty() {
        s.kind.manufacturer().to_string()
    } else {
        s.manufacturer.clone()
    };

    let frame = s.frame.unwrap_or(s.frames);
    let mut obj = serde_json::json!({
        "software_name": "radiosonde_auto_rx",
        "software_version": env!("CARGO_PKG_VERSION"),
        "uploader_callsign": call,
        "time_received": received,
        "manufacturer": manufacturer,
        "type": s.kind.sondehub_type(),
        "serial": s.serial,
        "frame": frame,
        "datetime": datetime,
        "lat": lat,
        "lon": lon,
        "alt": alt,
        "frequency": s.frequency_hz / 1e6,
    });
    if let Some(ref st) = s.subtype {
        if !st.is_empty() {
            obj["subtype"] = serde_json::json!(st);
        }
    }
    if s.rssi_dbfs > -200.0 {
        obj["rssi"] = serde_json::json!(s.rssi_dbfs);
    }
    if let Some(t) = s.temperature_c.filter(|v| *v > -273.0) {
        obj["temp"] = serde_json::json!(t);
    }
    if let Some(h) = s.humidity_pct.filter(|v| *v >= 0.0) {
        obj["humidity"] = serde_json::json!(h);
    }
    if let Some(p) = s.pressure_hpa.filter(|v| *v > 0.0) {
        obj["pressure"] = serde_json::json!(p);
    }
    if let Some(b) = s.battery_v.filter(|v| *v >= 0.0) {
        obj["batt"] = serde_json::json!(b);
    }
    if let Some(v) = s.ascent_rate_mps {
        obj["vel_v"] = serde_json::json!(v);
    }
    if let Some(v) = s.vel_h_mps {
        obj["vel_h"] = serde_json::json!(v);
    }
    if let Some(h) = s.heading_deg {
        obj["heading"] = serde_json::json!(h);
    }
    if let Some(n) = s.sats {
        obj["sats"] = serde_json::json!(n);
    }
    if let Some((lat, lon)) = grid_home(&rx.grid) {
        obj["uploader_position"] = serde_json::json!([lat, lon, 0.0]);
    }
    Some(obj)
}

fn grid_home(grid: &str) -> Option<(f64, f64)> {
    sdroxide_types::grid_to_latlon(grid.trim())
}

fn rfc3339(unix: i64) -> String {
    let days = unix.div_euclid(86_400);
    let secs = unix.rem_euclid(86_400) as u32;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    let h = secs / 3600;
    let mi = (secs % 3600) / 60;
    let s = secs % 60;
    format!("{y:04}-{m:02}-{d:02}T{h:02}:{mi:02}:{s:02}.000000Z")
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdroxide_types::SondeKind;

    #[test]
    fn skips_sonde_without_position() {
        let rx = Reporter { call: "N0CALL".into(), grid: "FN31".into(), antenna: String::new() };
        let s = Rs41Sonde::new("U1234567".into(), 403e6, SondeKind::Rs41, 1_700_000_000);
        assert!(telemetry_json(&rx, &s).is_none());
    }

    #[test]
    fn builds_payload_with_fix() {
        let rx =
            Reporter { call: "N0CALL".into(), grid: "FN31".into(), antenna: "colinear".into() };
        let mut s =
            Rs41Sonde::new("U1234567".into(), 403_100_000.0, SondeKind::Rs41, 1_700_000_000);
        s.lat = Some(40.1);
        s.lon = Some(-74.2);
        s.altitude_m = Some(12_000.0);
        s.temperature_c = Some(-45.0);
        s.subtype = Some("RS41-SGP".into());
        s.frame = Some(99);
        let v = telemetry_json(&rx, &s).unwrap();
        assert_eq!(v["serial"], "U1234567");
        assert_eq!(v["uploader_callsign"], "N0CALL");
        assert_eq!(v["type"], "RS41");
        assert_eq!(v["manufacturer"], "Vaisala");
        assert_eq!(v["subtype"], "RS41-SGP");
        assert_eq!(v["frame"], 99);
        assert!(v["uploader_position"].is_array());
    }

    #[test]
    fn dfm_payload_uses_graw() {
        let rx = Reporter { call: "N0CALL".into(), grid: "FN31".into(), antenna: String::new() };
        let mut s = Rs41Sonde::new("21041234".into(), 402e6, SondeKind::Dfm17, 1_700_000_000);
        s.lat = Some(50.0);
        s.lon = Some(10.0);
        s.altitude_m = Some(5_000.0);
        let v = telemetry_json(&rx, &s).unwrap();
        assert_eq!(v["type"], "DFM");
        assert_eq!(v["manufacturer"], "Graw");
    }

    #[test]
    fn normalize_strips_telemetry_suffix() {
        assert_eq!(
            normalize_base("https://api.v2.sondehub.org/sondes/telemetry"),
            "https://api.v2.sondehub.org"
        );
        assert_eq!(normalize_base(""), DEFAULT_BASE);
    }

    #[test]
    fn prefers_sonde_datetime_over_receive_time() {
        let rx = Reporter { call: "N0CALL".into(), grid: "FN31".into(), antenna: String::new() };
        let mut s =
            Rs41Sonde::new("U1234567".into(), 403e6, SondeKind::Rs41, 1_700_000_000);
        s.lat = Some(40.0);
        s.lon = Some(-74.0);
        s.altitude_m = Some(1_000.0);
        s.datetime = Some("2026-10-15T12:34:56.000Z".into());
        let v = telemetry_json(&rx, &s).unwrap();
        assert_eq!(v["datetime"], "2026-10-15T12:34:56.000Z");
        assert_ne!(v["time_received"], v["datetime"]);
    }

    #[test]
    fn parses_sondehub_soft_reject_body() {
        let body = r#"{"message":"some or all payloads could not be processed","errors":[{"error_message":"This software is unknown to us!"}],"warnings":[]}"#;
        let e = sondehub_response_errors(body).unwrap();
        assert!(e.contains("unknown to us"));
        assert!(sondehub_response_errors("^v^ telm logged").is_none());
    }
}
