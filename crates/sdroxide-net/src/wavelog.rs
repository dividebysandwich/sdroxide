//! The operator's Wavelog: checking the connection, and sending it QSOs.
//!
//! Written against Wavelog's own `application/controllers/Api.php` rather than
//! its documentation, because three of its behaviours are not obvious from the
//! outside and each one decides how a reply is read:
//!
//! * **A duplicate is an error.** `api/qso` answers 400 `"abort"` when the
//!   contact is already in the location's log (same call, band, mode and
//!   minute), with a message built from `__("Duplicate for")` — translated
//!   into the server's language, so the word cannot be matched. The line does
//!   start with the untranslated `Date/Time: `, though, and no other refusal
//!   does; that prefix is what [`is_duplicate`] reads. For an upload a
//!   duplicate is success: the QSO is there.
//! * **The station callsign and locator must match the location.** A QSO
//!   whose `STATION_CALLSIGN` is not the location's callsign, or whose
//!   `MY_GRIDSQUARE` is somewhere else, is refused with a sentence saying so.
//!   Nothing here second-guesses it; the sentence is passed on, because it is
//!   the operator's settings that need changing and it names them.
//! * **Most of the read endpoints take the key in the URL.** `station_info`,
//!   `check_auth` and `auth` read it from the path, not from a body; `version`,
//!   `qso` and `get_contacts_adif` read it from JSON. Each is called the way
//!   it reads.

use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use sdroxide_types::{
    UploadResult, UploadTarget, WavelogBatch, WavelogConfig, WavelogProbe, WavelogStation,
};
use serde_json::{Value, json};

use crate::event::{EventTx, NetEvent};
use crate::tls_pin;

/// Why a request did not produce an answer worth reading.
#[derive(Debug, Clone, PartialEq)]
pub struct Failure {
    pub message: String,
    /// The server was unreachable or busy rather than refusing; asking again
    /// later could work.
    pub retryable: bool,
    /// It was the certificate. Kept apart from `retryable` because the remedy
    /// is the operator's — trusting it, or finding out why it changed.
    pub tls: bool,
}

impl Failure {
    fn refused(message: impl Into<String>) -> Self {
        Failure { message: message.into(), retryable: false, tls: false }
    }
}

/// `https://log.example.org/` → `https://log.example.org/index.php/api/<endpoint>`.
///
/// Wavelog is CodeIgniter, and `index.php/…` reaches its controllers whether
/// or not the server rewrites pretty URLs, so that is the form always used.
/// The operator may paste the front page, the address with `index.php`, or
/// any page inside it (`…/index.php/dashboard`); all three mean the same
/// server.
pub fn api_url(base: &str, endpoint: &str) -> Result<String, String> {
    let base = base.trim();
    let lower = base.to_ascii_lowercase();
    if !(lower.starts_with("http://") || lower.starts_with("https://")) {
        return Err("The Wavelog address must start with http:// or https://".into());
    }
    let root = match lower.find("/index.php") {
        Some(i) => &base[..i],
        None => base.trim_end_matches('/'),
    };
    Ok(format!("{root}/index.php/api/{endpoint}"))
}

/// The base of an `https` address, for the certificate probe. `None` for plain
/// HTTP, where there is no certificate to look at.
fn https_root(base: &str) -> Option<String> {
    let url = api_url(base, "").ok()?;
    url.to_ascii_lowercase().starts_with("https://").then_some(url)
}

/// Seconds allowed for an exchange that moves one row: an upload, a check.
const SHORT_S: u64 = 20;
/// Seconds allowed for a download page.
const PAGE_S: u64 = 180;
/// QSOs asked for per download page. Small enough that a page of a big log is
/// a megabyte or two and comes back well inside [`PAGE_S`] on a slow server;
/// large enough that a first sync of a 20,000-QSO log is ten requests.
const PAGE_QSOS: u64 = 2000;

/// A configured connection to one Wavelog.
pub struct Client {
    key: String,
    base: String,
    pin: Option<String>,
}

impl Client {
    pub fn new(cfg: &WavelogConfig) -> Result<Client, String> {
        if !cfg.is_configured() {
            return Err("Wavelog address or API key not set".into());
        }
        api_url(&cfg.url, "")?;
        let pin =
            match cfg.cert_sha256.trim() {
                "" => None,
                p => Some(tls_pin::normalize_pin(p).ok_or(
                    "The trusted certificate fingerprint is not a SHA-256 (64 hex digits)",
                )?),
            };
        Ok(Client { key: cfg.api_key.trim().to_string(), base: cfg.url.trim().to_string(), pin })
    }

    /// An agent allowing `timeout_s` for the whole exchange.
    fn agent(&self, timeout_s: u64) -> ureq::Agent {
        let config = ureq::Agent::config_builder()
            .timeout_connect(Some(Duration::from_secs(10)))
            .timeout_global(Some(Duration::from_secs(timeout_s)))
            .http_status_as_error(false)
            .user_agent(concat!("sdroxide/", env!("CARGO_PKG_VERSION")))
            .build();
        match &self.pin {
            Some(pin) => tls_pin::pinned_agent(config, Some(pin.clone())).0,
            None => config.into(),
        }
    }

    /// POST a JSON body (with the key added) to `endpoint`.
    fn post(&self, endpoint: &str, body: Value) -> Result<(u16, String), Failure> {
        self.post_waiting(endpoint, body, SHORT_S)
    }

    /// [`Client::post`] allowing `timeout_s`, for a download page the server
    /// has to assemble before it sends a byte.
    fn post_waiting(
        &self,
        endpoint: &str,
        mut body: Value,
        timeout_s: u64,
    ) -> Result<(u16, String), Failure> {
        let url = api_url(&self.base, endpoint).map_err(Failure::refused)?;
        body["key"] = Value::String(self.key.clone());
        let resp = self
            .agent(timeout_s)
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .send(body.to_string());
        read(resp)
    }

    /// GET `endpoint/<key>`, for the endpoints that read the key from the path.
    fn get_keyed(&self, endpoint: &str) -> Result<(u16, String), Failure> {
        let url =
            api_url(&self.base, &format!("{endpoint}/{}", self.key)).map_err(Failure::refused)?;
        read(self.agent(SHORT_S).get(&url).call())
    }
}

/// Status and body, or a [`Failure`] saying what kind of not-an-answer it was.
fn read(
    resp: Result<ureq::http::Response<ureq::Body>, ureq::Error>,
) -> Result<(u16, String), Failure> {
    let mut resp = resp.map_err(transport_failure)?;
    let status = resp.status().as_u16();
    // A full log is far past ureq's 10 MB default; 256 MB is a log no amateur
    // has, and still a ceiling.
    let body = resp
        .body_mut()
        .with_config()
        .limit(256 * 1024 * 1024)
        .read_to_string()
        .map_err(transport_failure)?;
    Ok((status, body))
}

/// Whether `e` was the certificate check. The handshake runs lazily inside
/// the first write, so a refused certificate usually surfaces as an I/O error
/// carrying the rustls one rather than as ureq's own TLS variant.
fn is_tls_error(e: &ureq::Error) -> bool {
    match e {
        ureq::Error::Rustls(_) | ureq::Error::Tls(_) => true,
        ureq::Error::Io(io) => {
            io.get_ref().is_some_and(|inner| inner.downcast_ref::<rustls::Error>().is_some())
        }
        _ => false,
    }
}

fn transport_failure(e: ureq::Error) -> Failure {
    if is_tls_error(&e) {
        return Failure {
            message: format!("Wavelog: the server's certificate was not accepted ({e})"),
            retryable: false,
            tls: true,
        };
    }
    let message = match &e {
        ureq::Error::HostNotFound => "Wavelog: server name not found".to_string(),
        ureq::Error::ConnectionFailed => "Wavelog: could not connect to the server".to_string(),
        ureq::Error::Timeout(_) => "Wavelog: the server did not answer in time".to_string(),
        other => format!("Wavelog: {other}"),
    };
    Failure { message, retryable: true, tls: false }
}

/// Wavelog's messages are HTML fragments: `<br>` between lines and `<b>`
/// around the interesting part. Flatten to text.
fn strip_html(s: &str) -> String {
    let s = s.replace("<br>", "\n").replace("<br/>", "\n").replace("<br />", "\n");
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            c if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">").replace("&#039;", "'")
}

/// The refusal text of an `api/qso` reply, as plain lines.
fn reply_messages(v: &Value) -> Vec<String> {
    let mut lines = Vec::new();
    if let Some(r) = v.get("reason").and_then(Value::as_str) {
        lines.push(r.to_string());
    }
    if let Some(ms) = v.get("messages").and_then(Value::as_array) {
        for m in ms.iter().filter_map(Value::as_str) {
            lines.extend(
                strip_html(m).lines().map(str::trim).filter(|l| !l.is_empty()).map(String::from),
            );
        }
    }
    lines
}

/// A duplicate refusal, recognised by the one part of it Wavelog never
/// translates. See the module comment.
fn is_duplicate(line: &str) -> bool {
    line.trim_start().starts_with("Date/Time: ")
}

/// Re-emit one parsed ADIF record for Wavelog.
///
/// Empty fields are dropped rather than sent: an empty `STATION_CALLSIGN`
/// would be compared with the location's callsign and refused, where an
/// absent one is filled in from the location. Lengths are counted in
/// characters, which is how Wavelog's parser reads them first — it falls back
/// to bytes, but an accented name should not depend on the fallback.
fn wavelog_adif(fields: &[(String, String)]) -> String {
    let mut out = String::new();
    for (name, value) in fields {
        if value.trim().is_empty() {
            continue;
        }
        out.push_str(&format!("<{}:{}>{} ", name, value.chars().count(), value));
    }
    out.push_str("<EOR>");
    out
}

/// Send one QSO (the first record of `adif`) to `station_id`.
///
/// `Ok` carries the line for the network log; a duplicate is `Ok`, since the
/// QSO is in Wavelog either way.
pub fn upload(cfg: &WavelogConfig, station_id: u32, adif: &str) -> Result<String, Failure> {
    let client = Client::new(cfg).map_err(Failure::refused)?;
    if station_id == 0 {
        return Err(Failure::refused(
            "Wavelog: no station location chosen — pick one in Settings → Uploads → Wavelog",
        ));
    }
    let records = sdroxide_types::adif_records(adif);
    let Some(fields) = records.first() else {
        return Err(Failure::refused("Wavelog: nothing to upload (no QSO in the ADIF)"));
    };
    let body = json!({
        // A string, as Wavelog's own clients send it; it is compared, not
        // computed with.
        "station_profile_id": station_id.to_string(),
        "type": "adif",
        "string": wavelog_adif(fields),
    });
    let (status, text) = client.post("qso", body)?;
    interpret_upload(status, &text)
}

/// Read an `api/qso` reply. Apart from [`upload`] so the reading can be tested
/// against replies captured from a real server.
fn interpret_upload(status: u16, text: &str) -> Result<String, Failure> {
    let parsed: Option<Value> = serde_json::from_str(text).ok();
    let lines = parsed.as_ref().map(reply_messages).unwrap_or_default();
    let detail = || {
        if lines.is_empty() { text.chars().take(160).collect::<String>() } else { lines.join("; ") }
    };
    let status_field = parsed.as_ref().and_then(|v| v.get("status")).and_then(Value::as_str);
    match status {
        201 if status_field == Some("created") => Ok("Wavelog: logged".into()),
        400 if !lines.is_empty() && lines.iter().all(|l| is_duplicate(l)) => {
            Ok("Wavelog: already in the log".into())
        }
        400 => Err(Failure::refused(format!("Wavelog refused the QSO: {}", detail()))),
        401 => Err(Failure::refused(format!("Wavelog: {}", detail()))),
        403 => Err(Failure::refused(
            "Wavelog: this API key is read-only — uploading needs a read/write key",
        )),
        429 => Err(Failure {
            message: "Wavelog: too many requests, will try again".into(),
            retryable: true,
            tls: false,
        }),
        500..=599 => Err(Failure {
            message: format!("Wavelog: server error {status}"),
            retryable: true,
            tls: false,
        }),
        // A 200 with `"failed"` is how it reports JSON it could not read.
        _ if parsed.is_none() => Err(Failure::refused(not_wavelog(status))),
        _ => Err(Failure::refused(format!("Wavelog: HTTP {status}: {}", detail()))),
    }
}

/// An answer that is not Wavelog's JSON: almost always a URL that reaches a
/// web server but not the Wavelog on it.
fn not_wavelog(status: u16) -> String {
    format!(
        "Wavelog: the address answered (HTTP {status}) but not like a Wavelog API — it should \
         be the address of your Wavelog's front page"
    )
}

/// Check the connection, the key and the station locations, without writing
/// anything.
///
/// ⛔ READ-ONLY, like every login check: `version`, `station_info` and
/// `check_auth` only read. `api/qso` has a dry-run form, and it is
/// deliberately not used — a mistake in the URL would make it a real insert.
pub fn probe(cfg: &WavelogConfig) -> WavelogProbe {
    let mut out = WavelogProbe::default();
    let client = match Client::new(cfg) {
        Ok(c) => c,
        Err(e) => {
            out.message = e;
            return out;
        }
    };

    // The version answers "is this a Wavelog, and is the key good" in one
    // read. A Wavelog too old to have the endpoint answers 404, and the
    // station list below still settles both questions.
    match client.post("version", json!({})) {
        Ok((200, text)) => {
            let v: Option<Value> = serde_json::from_str(&text).ok();
            match v.as_ref().and_then(|v| v.get("version")).and_then(Value::as_str) {
                Some(ver) => out.version = ver.to_string(),
                None => {
                    out.message = not_wavelog(200);
                    return out;
                }
            }
        }
        Ok((401, _)) => {
            out.message = "Wavelog: the API key was not accepted (wrong, or disabled)".into();
            return out;
        }
        Ok((404, _)) => {}
        Ok((status, _)) => {
            out.message = not_wavelog(status);
            return out;
        }
        Err(f) => {
            out.message = f.message;
            if f.tls {
                out.untrusted_cert = presented_certificate(cfg);
                out.message = match (&out.untrusted_cert, client.pin.is_some()) {
                    (Some(_), true) => "Wavelog: the server's certificate has CHANGED since you \
                                        trusted it. Only trust the new one if you changed it."
                        .into(),
                    (Some(_), false) => "Wavelog: the server's certificate is not signed by a \
                                         known authority. If it is your own server's, compare \
                                         the fingerprint below and trust it."
                        .into(),
                    (None, _) => out.message,
                };
            }
            return out;
        }
    }

    match client.get_keyed("station_info") {
        Ok((200, text)) => match parse_stations(&text) {
            Some(s) => out.stations = s,
            None => {
                out.message = not_wavelog(200);
                return out;
            }
        },
        Ok((401, _)) => {
            out.message = "Wavelog: the API key was not accepted (wrong, or disabled)".into();
            return out;
        }
        Ok((status, _)) => {
            out.message = not_wavelog(status);
            return out;
        }
        Err(f) => {
            out.message = f.message;
            return out;
        }
    }

    out.read_write = rights(&client);
    out.ok = true;
    let rights = match out.read_write {
        Some(true) => "read/write key",
        Some(false) => "READ-ONLY key — downloads only, uploads will be refused",
        None => "key accepted",
    };
    let version = if out.version.is_empty() { String::new() } else { format!(" {}", out.version) };
    out.message = format!(
        "Wavelog{version} · {rights} · {} station location{}",
        out.stations.len(),
        if out.stations.len() == 1 { "" } else { "s" }
    );
    out
}

/// Whether the key may write: `check_auth`'s JSON on a current Wavelog, the
/// older `auth` endpoint's XML otherwise.
fn rights(client: &Client) -> Option<bool> {
    if let Ok((200, text)) = client.get_keyed("check_auth") {
        let v: Value = serde_json::from_str(&text).ok()?;
        return v.get("rights").and_then(Value::as_str).map(|r| r.contains('w'));
    }
    let (_, text) = client.get_keyed("auth").ok()?;
    let start = text.find("<rights>")? + "<rights>".len();
    let end = text[start..].find("</rights>")? + start;
    Some(text[start..end].contains('w'))
}

/// `station_info`'s list. The numbers come as strings or numbers depending on
/// the database driver, so both are read.
fn parse_stations(text: &str) -> Option<Vec<WavelogStation>> {
    let v: Value = serde_json::from_str(text).ok()?;
    let rows = v.as_array()?;
    let num = |v: Option<&Value>| -> Option<u32> {
        match v? {
            Value::Number(n) => n.as_u64().map(|n| n as u32),
            Value::String(s) => s.trim().parse().ok(),
            _ => None,
        }
    };
    let text = |v: Option<&Value>| v.and_then(Value::as_str).unwrap_or_default().trim().to_string();
    let mut out: Vec<WavelogStation> = rows
        .iter()
        .filter_map(|r| {
            Some(WavelogStation {
                id: num(r.get("station_id"))?,
                name: text(r.get("station_profile_name")),
                callsign: text(r.get("station_callsign")).to_ascii_uppercase(),
                grid: text(r.get("station_gridsquare")).to_ascii_uppercase(),
                active: num(r.get("station_active")).unwrap_or(0) != 0,
            })
        })
        .collect();
    out.sort_by_key(|s| s.id);
    Some(out)
}

/// The SHA-256 of the certificate the server presents, for an operator who
/// has been told it was not accepted. Reads only the handshake: the probe
/// agent refuses every certificate, so nothing is sent after it.
fn presented_certificate(cfg: &WavelogConfig) -> Option<String> {
    let url = https_root(&cfg.url)?;
    let config = ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(10)))
        .timeout_global(Some(Duration::from_secs(20)))
        .build();
    let (agent, seen) = tls_pin::pinned_agent(config, None);
    let _ = agent.get(&url).call();
    seen.lock().ok()?.clone()
}

/// Download one station location's QSOs after `from_id`, handing each page
/// to `page` as it arrives.
///
/// Wavelog returns records in ascending id order and says the highest id it
/// sent, so asking again from there is the next page; a page shorter than
/// asked for is the last. The records come back marked as in Wavelog and as
/// having come from `station_id`. `full` is carried onto every batch.
pub fn download(
    cfg: &WavelogConfig,
    station_id: u32,
    mut from_id: u64,
    full: bool,
    mut page: impl FnMut(WavelogBatch),
) -> Result<(), String> {
    let client = Client::new(cfg)?;
    loop {
        let body = json!({ "station_id": station_id, "fetchfromid": from_id, "limit": PAGE_QSOS });
        let (status, text) =
            client.post_waiting("get_contacts_adif", body, PAGE_S).map_err(|f| f.message)?;
        let (last_id, exported, adif) = interpret_download(status, &text)?;
        let mut records = sdroxide_types::adif_to_qso_log(&adif);
        for r in &mut records {
            r.wavelog_sent = true;
            r.wavelog_origin = station_id;
        }
        let last_id = last_id.max(from_id);
        page(WavelogBatch { station_id, from_id, last_id, records, full, ..Default::default() });
        if exported < PAGE_QSOS || last_id <= from_id {
            return Ok(());
        }
        from_id = last_id;
    }
}

/// Read a `get_contacts_adif` reply into (highest id sent, how many, ADIF).
fn interpret_download(status: u16, text: &str) -> Result<(u64, u64, String), String> {
    let parsed: Option<Value> = serde_json::from_str(text).ok();
    let reason = || {
        parsed
            .as_ref()
            .and_then(|v| v.get("reason"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string()
    };
    match status {
        200 => {}
        401 => return Err(format!("Wavelog: {}", reason())),
        // CodeIgniter's answer for a controller method that does not exist.
        404 => {
            return Err("Wavelog: this server has no QSO download (api/get_contacts_adif) — \
                        it needs a newer Wavelog"
                .into());
        }
        429 => return Err("Wavelog: too many requests — will try at the next sync".into()),
        _ if parsed.is_some() => return Err(format!("Wavelog: HTTP {status}: {}", reason())),
        _ => return Err(not_wavelog(status)),
    }
    let Some(v) = parsed.as_ref() else { return Err(not_wavelog(status)) };
    if v.get("status").and_then(Value::as_str) != Some("successful") {
        return Err(format!("Wavelog: {}", reason()));
    }
    let num = |name: &str| -> u64 {
        match v.get(name) {
            Some(Value::Number(n)) => n.as_u64().unwrap_or(0),
            Some(Value::String(s)) => s.trim().parse().unwrap_or(0),
            _ => 0,
        }
    };
    let adif = v.get("adif").and_then(Value::as_str).unwrap_or_default().to_string();
    Ok((num("lastfetchedid"), num("exported_qsos"), adif))
}

/// The mode as Wavelog's radio status wants it: what an operator would log,
/// in at most the ten characters the API takes. FM and AM variants are the
/// plain modes; the data sidebands are the sideband — Wavelog's form treats a
/// digital mode it is not told about as the sideband it rides on.
pub fn radio_mode(mode: sdroxide_types::Mode) -> String {
    use sdroxide_types::Mode;
    let label = match mode {
        Mode::Nfm | Mode::Wfm => "FM",
        Mode::Sam | Mode::Dsb => "AM",
        Mode::Digu | Mode::Isb => "USB",
        Mode::Digl => "LSB",
        other => other.label(),
    };
    label.chars().take(10).collect()
}

/// What the radio is doing, as Wavelog is told it.
#[derive(Debug, Clone, PartialEq)]
pub struct RadioNow {
    pub tx_hz: u64,
    pub rx_hz: u64,
    pub mode: String,
}

/// Seconds between repeats of an unchanged status, so Wavelog keeps the radio
/// listed as live; it stamps each update with the time it arrived.
const RADIO_KEEPALIVE_S: u64 = 120;
/// At most one update a second: a dial being spun is reported where it
/// stops, not every step on the way there.
const RADIO_MIN_GAP_MS: u64 = 1000;

/// Reports the radio's frequency and mode to Wavelog's `api/radio`, the way
/// WaveLogGate does, so Wavelog's live-logging form and bandmap follow it.
///
/// Fed the current state on every engine tick, cheaply — it only replaces the
/// latest value — and does its own deciding on a thread: send on a change, at
/// most once a second, and again every couple of minutes when nothing
/// changes. A server that is down costs a failed request per gap, not a
/// backlog.
pub struct RadioReporter {
    shared: Arc<(Mutex<RadioSlot>, std::sync::Condvar)>,
}

/// The latest state handed to a [`RadioReporter`], and whether it has been
/// looked at. The flag is what makes a change made while the worker is busy
/// sending — when nobody is waiting to be woken — still count: a condition
/// variable's wake-up is not remembered, a flag is.
#[derive(Default)]
struct RadioSlot {
    latest: Option<(WavelogConfig, RadioNow)>,
    changed: bool,
}

impl RadioReporter {
    pub fn start(events: EventTx) -> RadioReporter {
        let shared: Arc<(Mutex<RadioSlot>, std::sync::Condvar)> = Arc::default();
        let worker = shared.clone();
        std::thread::Builder::new()
            .name("sdroxide-wavelog-radio".into())
            .spawn(move || radio_worker(&worker, &events))
            .ok();
        RadioReporter { shared }
    }

    /// The radio's state now. Cheap enough for every tick.
    pub fn update(&self, cfg: &WavelogConfig, now: RadioNow) {
        let (lock, wake) = &*self.shared;
        if let Ok(mut slot) = lock.lock() {
            let fresh = slot.latest.as_ref().is_none_or(|(c, n)| *n != now || c != cfg);
            if fresh {
                slot.latest = Some((cfg.clone(), now));
                slot.changed = true;
                wake.notify_one();
            }
        }
    }
}

/// Take the latest state, marking it seen.
fn take_latest(lock: &Mutex<RadioSlot>) -> Option<(WavelogConfig, RadioNow)> {
    let mut slot = lock.lock().ok()?;
    slot.changed = false;
    slot.latest.clone()
}

fn radio_worker(shared: &(Mutex<RadioSlot>, std::sync::Condvar), events: &EventTx) {
    let (lock, wake) = shared;
    let keepalive = Duration::from_secs(RADIO_KEEPALIVE_S);
    let gap = Duration::from_millis(RADIO_MIN_GAP_MS);
    let mut sent: Option<(String, RadioNow)> = None;
    let mut sent_at: Option<std::time::Instant> = None;
    let mut last_error: Option<String> = None;
    loop {
        // Until something changes, or it is time to say it again.
        {
            let Ok(slot) = lock.lock() else { return };
            if wake.wait_timeout_while(slot, keepalive, |s| !s.changed).is_err() {
                return;
            }
        }
        // Paced: a dial being spun is reported where it has got to after the
        // gap, not at every step.
        if let Some(t) = sent_at
            && t.elapsed() < gap
        {
            std::thread::sleep(gap - t.elapsed());
        }
        let Some((cfg, now)) = take_latest(lock) else { continue };
        if !cfg.radio_status || !cfg.is_configured() {
            sent = None;
            continue;
        }
        let current = (cfg.radio_name.trim().to_string(), now);
        let due = sent_at.is_none_or(|t| t.elapsed() >= keepalive);
        if sent.as_ref() == Some(&current) && !due {
            continue;
        }
        sent_at = Some(std::time::Instant::now());
        match send_radio(&cfg, &current.0, &current.1) {
            Ok(()) => {
                sent = Some(current);
                last_error = None;
            }
            Err(e) => {
                // Said once, not once per attempt.
                if last_error.as_deref() != Some(e.as_str()) {
                    let _ =
                        events.send(NetEvent::Status(Some(format!("Wavelog radio status: {e}"))));
                    last_error = Some(e);
                }
            }
        }
    }
}

/// POST one status to `api/radio`.
pub fn send_radio(cfg: &WavelogConfig, name: &str, now: &RadioNow) -> Result<(), String> {
    let client = Client::new(cfg)?;
    let name = if name.is_empty() { "sdroxide" } else { name };
    let mut body = json!({ "radio": name, "frequency": now.tx_hz, "mode": now.mode });
    // Split: Wavelog's `frequency` is the transmit one and `frequency_rx` the
    // receive one, which the log form fills in as the RX frequency.
    if now.rx_hz != now.tx_hz && now.rx_hz != 0 {
        body["frequency_rx"] = json!(now.rx_hz);
    }
    let (status, text) = client.post("radio", body).map_err(|f| f.message)?;
    let reason = || {
        serde_json::from_str::<Value>(&text)
            .ok()
            .and_then(|v| v.get("reason").and_then(Value::as_str).map(String::from))
            .unwrap_or_else(|| format!("HTTP {status}"))
    };
    match status {
        200 => Ok(()),
        403 => Err("the API key is read-only — the radio status needs a read/write key".into()),
        _ => Err(reason()),
    }
}

/// Uploads to Wavelog, one at a time, in the order they were asked for.
///
/// A queue rather than a thread per QSO, which is what the other services get,
/// because Wavelog is the one service sdroxide sends backlogs to: "send the
/// QSOs it has not got" can be a thousand contacts, and a thousand concurrent
/// requests would be refused by a rate-limited server and felt by a small one.
/// A QSO already waiting is not queued twice, so the retry timer and the
/// operator's button can both ask without doubling the traffic.
pub struct UploadQueue {
    tx: crossbeam_channel::Sender<Job>,
    waiting: Arc<Mutex<HashSet<u64>>>,
}

struct Job {
    cfg: WavelogConfig,
    station_id: u32,
    qso_id: u64,
    adif: String,
}

impl UploadQueue {
    /// Start the worker. Results go to `events` as [`NetEvent::Upload`].
    pub fn start(events: EventTx) -> UploadQueue {
        let (tx, rx) = crossbeam_channel::unbounded::<Job>();
        let waiting: Arc<Mutex<HashSet<u64>>> = Arc::default();
        let done = waiting.clone();
        std::thread::Builder::new()
            .name("sdroxide-wavelog".into())
            .spawn(move || {
                for job in rx {
                    let (ok, message, retryable) = match upload(&job.cfg, job.station_id, &job.adif)
                    {
                        Ok(m) => (true, m, false),
                        Err(f) => (false, f.message, f.retryable),
                    };
                    if let Ok(mut w) = done.lock() {
                        w.remove(&job.qso_id);
                    }
                    let _ = events.send(NetEvent::Upload(UploadResult {
                        qso_id: job.qso_id,
                        target: UploadTarget::Wavelog,
                        ok,
                        message,
                        retryable,
                    }));
                }
            })
            .ok();
        UploadQueue { tx, waiting }
    }

    /// Queue one QSO, unless it is already waiting.
    pub fn push(&self, cfg: &WavelogConfig, station_id: u32, qso_id: u64, adif: String) {
        let fresh = self.waiting.lock().map(|mut w| w.insert(qso_id)).unwrap_or(true);
        if fresh {
            let job = Job { cfg: cfg.clone(), station_id, qso_id, adif };
            let _ = self.tx.send(job);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_way_of_writing_the_address_reaches_the_same_api() {
        let want = "https://log.example.org/index.php/api/qso";
        for base in [
            "https://log.example.org",
            "https://log.example.org/",
            "https://log.example.org/index.php",
            "https://log.example.org/index.php/dashboard",
            "  https://log.example.org/  ",
        ] {
            assert_eq!(api_url(base, "qso").unwrap(), want, "{base}");
        }
        assert_eq!(
            api_url("http://192.168.1.10/wavelog/", "qso").unwrap(),
            "http://192.168.1.10/wavelog/index.php/api/qso"
        );
    }

    #[test]
    fn an_address_without_a_scheme_is_refused_with_a_reason() {
        assert!(api_url("log.example.org", "qso").unwrap_err().contains("http://"));
    }

    #[test]
    fn only_https_has_a_certificate_to_probe() {
        assert!(https_root("http://10.0.0.2/wavelog").is_none());
        assert!(https_root("https://10.0.0.2/wavelog").is_some());
    }

    #[test]
    fn a_created_reply_is_logged() {
        let r = r#"{"status":"created","type":"adif","string":"","adif_count":1,"adif_errors":0,"messages":[""]}"#;
        assert_eq!(interpret_upload(201, r).unwrap(), "Wavelog: logged");
    }

    /// A duplicate as a Spanish-language Wavelog words it: only the
    /// `Date/Time:` prefix is left in English.
    #[test]
    fn a_duplicate_in_any_language_counts_as_uploaded() {
        let r = r#"{"status":"abort","type":"adif","string":"","adif_count":1,"adif_errors":1,"messages":["","Date/Time: 2026-10-11 10:00:00 Callsign: EA1AA Band: 20m Duplicado para EA5ABC<br>"]}"#;
        assert_eq!(interpret_upload(400, r).unwrap(), "Wavelog: already in the log");
    }

    #[test]
    fn a_callsign_mismatch_is_refused_with_wavelogs_own_sentence() {
        let r = r#"{"status":"abort","adif_errors":1,"messages":["","Differing station callsign <b>EA5ABC/P</b> while importing QSO with EA1AA for <b>EA5ABC</b>: SKIPPED<br>"]}"#;
        let f = interpret_upload(400, r).unwrap_err();
        assert!(!f.retryable);
        assert!(f.message.contains("Differing station callsign EA5ABC/P"), "{}", f.message);
        assert!(!f.message.contains("<b>"));
    }

    #[test]
    fn a_read_only_key_says_so() {
        let r = r#"{"status":"failed","reason":"API key does not have write permissions"}"#;
        let f = interpret_upload(403, r).unwrap_err();
        assert!(f.message.contains("read-only"));
        assert!(!f.retryable);
    }

    #[test]
    fn a_busy_or_broken_server_is_worth_retrying() {
        assert!(interpret_upload(503, "").unwrap_err().retryable);
        assert!(interpret_upload(429, "").unwrap_err().retryable);
    }

    #[test]
    fn a_web_page_where_the_api_should_be_says_the_address_is_wrong() {
        let f = interpret_upload(200, "<html><body>Welcome</body></html>").unwrap_err();
        assert!(f.message.contains("front page"), "{}", f.message);
    }

    #[test]
    fn a_download_page_is_read_whichever_way_the_numbers_are_written() {
        let r = r#"{"status":"successful","message":"Export successful","lastfetchedid":"812","exported_qsos":2,"adif":"<CALL:5>EA1AA <EOR>"}"#;
        let (last, n, adif) = interpret_download(200, r).unwrap();
        assert_eq!((last, n), (812, 2));
        assert!(adif.contains("EA1AA"));
    }

    #[test]
    fn nothing_new_is_an_empty_page_not_an_error() {
        let r = r#"{"status":"successful","message":"No new QSOs available.","lastfetchedid":40,"exported_qsos":0,"adif":null}"#;
        assert_eq!(interpret_download(200, r).unwrap(), (40, 0, String::new()));
    }

    #[test]
    fn a_wavelog_without_the_download_says_it_needs_updating() {
        assert!(interpret_download(404, "<html>404</html>").unwrap_err().contains("newer Wavelog"));
    }

    #[test]
    fn a_web_page_instead_of_a_download_says_the_address_is_wrong() {
        assert!(interpret_download(200, "<html>hi</html>").unwrap_err().contains("front page"));
    }

    #[test]
    fn a_location_the_key_cannot_read_is_reported_in_wavelogs_words() {
        let r = r#"{"status":"failed","reason":"Station ID not accessible for this API key"}"#;
        assert!(interpret_download(401, r).unwrap_err().contains("not accessible"));
    }

    #[test]
    fn the_radio_mode_is_the_mode_an_operator_would_log() {
        use sdroxide_types::Mode;
        assert_eq!(radio_mode(Mode::Usb), "USB");
        assert_eq!(radio_mode(Mode::Nfm), "FM");
        assert_eq!(radio_mode(Mode::Digu), "USB");
        assert_eq!(radio_mode(Mode::Ft8), "FT8");
        assert!(radio_mode(Mode::PacketHf).len() <= 10);
    }

    #[test]
    fn empty_fields_are_left_out_and_lengths_count_characters() {
        let fields = vec![
            ("CALL".to_string(), "EA1AA".to_string()),
            ("NAME".to_string(), "José".to_string()),
            ("STATION_CALLSIGN".to_string(), String::new()),
        ];
        assert_eq!(wavelog_adif(&fields), "<CALL:5>EA1AA <NAME:4>José <EOR>");
    }

    #[test]
    fn station_ids_are_read_as_strings_or_numbers() {
        let text = r#"[
            {"station_id":"2","station_profile_name":"Portable","station_callsign":"ea5abc/p","station_gridsquare":"im98","station_active":null},
            {"station_id":1,"station_profile_name":"Home","station_callsign":"EA5ABC","station_gridsquare":"IM99","station_active":"1"}
        ]"#;
        let s = parse_stations(text).unwrap();
        assert_eq!(s.len(), 2);
        assert_eq!((s[0].id, s[0].name.as_str(), s[0].active), (1, "Home", true));
        assert_eq!(
            (s[1].callsign.as_str(), s[1].grid.as_str(), s[1].active),
            ("EA5ABC/P", "IM98", false)
        );
    }
}

/// The client against a real server on loopback: plain HTTP, HTTPS with a
/// self-signed certificate, and that certificate pinned — the three ways an
/// operator's Wavelog is reached. The server is a few lines over rustls that
/// answers each API path with a canned reply, which is all these need: what
/// is under test is the transport and the reading, not Wavelog.
#[cfg(test)]
mod loopback_tests {
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;
    use std::sync::Arc;

    use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};

    use super::*;

    const CERT: &[u8] = include_bytes!("../tests/fixtures/wavelog-selfsigned.crt.der");
    const KEY: &[u8] = include_bytes!("../tests/fixtures/wavelog-selfsigned.key.der");
    const KEY_ID: &str = "wl-test-key";

    /// The canned answer for a request path (and body, for the download).
    fn reply(path: &str, body: &str) -> (u16, String) {
        let p = path.trim_start_matches("/index.php/api/");
        if p == "get_contacts_adif" {
            return download_page(body);
        }
        if p == "version" {
            (200, r#"{"status":"ok","version":"2.1.2"}"#.into())
        } else if p == format!("station_info/{KEY_ID}") {
            (200, r#"[{"station_id":"4","station_profile_name":"Home","station_callsign":"EA5ABC","station_gridsquare":"IM99","station_active":"1"}]"#.into())
        } else if p == format!("check_auth/{KEY_ID}") {
            (200, r#"{"status":"valid","rights":"rw"}"#.into())
        } else if p == "radio" {
            let v: Value = serde_json::from_str(body).unwrap_or_default();
            if v["radio"] == "pace-test" {
                PACE_HEARD.lock().unwrap().push(v["frequency"].as_u64().unwrap_or(0));
                return (200, r#"{"status":"success"}"#.into());
            }
            let ok = v["radio"] == "IC-7300"
                && v["frequency"] == 14_074_000
                && v["mode"] == "FT8"
                && v.get("frequency_rx").is_none()
                && v["key"] == KEY_ID;
            if ok {
                (200, r#"{"status":"success"}"#.into())
            } else {
                (400, r#"{"status":"failed","reason":"bad payload"}"#.into())
            }
        } else if p == "qso" {
            (201, r#"{"status":"created","adif_count":1,"adif_errors":0,"messages":[""]}"#.into())
        } else {
            (404, "not found".into())
        }
    }

    /// Answer one HTTP/1.1 request on `stream`, closing after it.
    fn serve<S: Read + Write>(stream: S) {
        let mut reader = BufReader::new(stream);
        let mut request_line = String::new();
        if reader.read_line(&mut request_line).is_err() {
            return; // a refused handshake ends here
        }
        let path = request_line.split_whitespace().nth(1).unwrap_or("/").to_string();
        let mut length = 0usize;
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).unwrap_or(0) == 0 || line == "\r\n" {
                break;
            }
            if let Some(v) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                length = v.trim().parse().unwrap_or(0);
            }
        }
        let mut body = vec![0; length];
        let _ = reader.read_exact(&mut body);
        let (status, text) = reply(&path, &String::from_utf8_lossy(&body));
        let head = format!(
            "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\
             Connection: close\r\n\r\n",
            text.len()
        );
        let stream = reader.get_mut();
        let _ = stream.write_all(head.as_bytes());
        let _ = stream.write_all(text.as_bytes());
        let _ = stream.flush();
    }

    /// Start a server; returns its base URL. It serves until the test ends.
    fn server(tls: bool) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let config = tls.then(|| {
            let provider = Arc::new(rustls::crypto::ring::default_provider());
            let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(KEY.to_vec()));
            Arc::new(
                rustls::ServerConfig::builder_with_provider(provider)
                    .with_safe_default_protocol_versions()
                    .unwrap()
                    .with_no_client_auth()
                    .with_single_cert(vec![CertificateDer::from(CERT.to_vec())], key)
                    .unwrap(),
            )
        });
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                match &config {
                    Some(c) => {
                        let Ok(conn) = rustls::ServerConnection::new(c.clone()) else { continue };
                        serve(rustls::StreamOwned::new(conn, stream));
                    }
                    None => serve(stream),
                }
            }
        });
        let scheme = if tls { "https" } else { "http" };
        format!("{scheme}://127.0.0.1:{port}/")
    }

    /// A log of 4500 QSOs at location 4, ids 1..=4500, served the way
    /// Wavelog serves it: ascending from `fetchfromid`, at most `limit`.
    fn download_page(body: &str) -> (u16, String) {
        let v: Value = serde_json::from_str(body).unwrap();
        if v["station_id"] != 4 {
            return (
                401,
                r#"{"status":"failed","reason":"Station ID not accessible for this API key"}"#
                    .into(),
            );
        }
        let from = v["fetchfromid"].as_u64().unwrap();
        let limit = v["limit"].as_u64().unwrap();
        let ids: Vec<u64> = (from + 1..=4500).take(limit as usize).collect();
        let mut adif = String::from("<ADIF_VER:5>3.1.4 <EOH>\r\n");
        for id in &ids {
            let call = format!("EA{id}AA");
            adif.push_str(&format!(
                "<CALL:{}>{call} <BAND:3>20m <MODE:4>MFSK <SUBMODE:3>FT4 <QSO_DATE:8>20261011 \
                 <TIME_ON:6>120000 <STATION_CALLSIGN:6>EA5ABC <LOTW_QSL_RCVD:1>Y <EOR>\r\n",
                call.len()
            ));
        }
        let last = ids.last().copied().unwrap_or(from);
        let reply = json!({
            "status": "successful",
            "lastfetchedid": last,
            "exported_qsos": ids.len(),
            "adif": if ids.is_empty() { Value::Null } else { Value::String(adif) },
        });
        (200, reply.to_string())
    }

    #[test]
    fn the_radio_status_is_accepted() {
        let now = RadioNow { tx_hz: 14_074_000, rx_hz: 14_074_000, mode: "FT8".into() };
        send_radio(&cfg(server(false), ""), "IC-7300", &now).unwrap();
    }

    /// A dial spun through five frequencies in a moment is reported where it
    /// stopped, and not five times.
    #[test]
    fn a_spun_dial_is_reported_where_it_stops() {
        let mut cfg = cfg(server(false), "");
        cfg.radio_status = true;
        cfg.radio_name = "pace-test".into();
        let (tx, _rx) = crossbeam_channel::unbounded();
        let reporter = RadioReporter::start(tx);
        for khz in [7_000, 7_010, 7_020, 7_030, 7_040] {
            let hz = khz * 1000;
            reporter.update(&cfg, RadioNow { tx_hz: hz, rx_hz: hz, mode: "CW".into() });
            std::thread::sleep(Duration::from_millis(50));
        }
        std::thread::sleep(Duration::from_millis(2500));
        let heard = PACE_HEARD.lock().unwrap().clone();
        assert_eq!(heard.last(), Some(&7_040_000), "{heard:?}");
        assert!(heard.len() <= 3, "paced, not one per step: {heard:?}");
    }

    /// What the pacing test's radio was reported at, in order.
    static PACE_HEARD: Mutex<Vec<u64>> = Mutex::new(Vec::new());

    #[test]
    fn a_whole_log_comes_down_a_page_at_a_time() {
        let cfg = cfg(server(false), "");
        let mut pages = Vec::new();
        download(&cfg, 4, 0, false, |b| pages.push(b)).unwrap();
        // 2000 + 2000 + 500: the short page is the last.
        assert_eq!(
            pages.iter().map(|b| b.records.len()).collect::<Vec<_>>(),
            vec![2000, 2000, 500]
        );
        assert_eq!(
            pages.iter().map(|b| (b.from_id, b.last_id)).collect::<Vec<_>>(),
            vec![(0, 2000), (2000, 4000), (4000, 4500)]
        );
        let r = &pages[2].records[0];
        assert_eq!(
            (r.call.as_str(), r.mode.as_str(), r.my_call.as_str()),
            ("EA4001AA", "FT4", "EA5ABC")
        );
        assert!(r.wavelog_sent && r.lotw_rcvd);
        assert_eq!(r.wavelog_origin, 4);
    }

    #[test]
    fn an_up_to_date_location_is_one_empty_page() {
        let mut pages = Vec::new();
        download(&cfg(server(false), ""), 4, 4500, false, |b| pages.push(b)).unwrap();
        assert_eq!(pages.len(), 1);
        assert!(pages[0].records.is_empty());
        assert_eq!(pages[0].last_id, 4500);
    }

    #[test]
    fn a_location_the_key_cannot_read_stops_the_download() {
        let e = download(&cfg(server(false), ""), 9, 0, false, |_| {}).unwrap_err();
        assert!(e.contains("not accessible"), "{e}");
    }

    fn cfg(url: String, pin: &str) -> WavelogConfig {
        WavelogConfig { url, api_key: KEY_ID.into(), cert_sha256: pin.into(), ..Default::default() }
    }

    #[test]
    fn plain_http_on_the_lan_works() {
        let p = probe(&cfg(server(false), ""));
        assert!(p.ok, "{}", p.message);
        assert_eq!(p.version, "2.1.2");
        assert_eq!(p.read_write, Some(true));
        assert_eq!(p.stations.len(), 1);
        assert_eq!(p.stations[0].id, 4);
    }

    #[test]
    fn a_self_signed_certificate_is_refused_and_its_fingerprint_shown() {
        let p = probe(&cfg(server(true), ""));
        assert!(!p.ok);
        assert_eq!(p.untrusted_cert.as_deref(), Some(tls_pin::sha256_hex(CERT).as_str()));
        assert!(p.message.contains("not signed by a known authority"), "{}", p.message);
    }

    #[test]
    fn the_pinned_certificate_is_accepted_whatever_the_host_name() {
        // The certificate says `wavelog.test` and the server is reached as
        // 127.0.0.1: the pin is what is trusted, not the name.
        let url = server(true);
        let pin = sdroxide_types::fingerprint_display(&tls_pin::sha256_hex(CERT));
        let p = probe(&cfg(url.clone(), &pin));
        assert!(p.ok, "{}", p.message);
        assert_eq!(p.stations[0].callsign, "EA5ABC");
        assert_eq!(p.untrusted_cert, None);

        let adif = "<CALL:5>EA1AA <BAND:3>20m <MODE:3>FT8 <QSO_DATE:8>20261011 \
                    <TIME_ON:6>100000 <EOR>";
        assert_eq!(upload(&cfg(url, &pin), 4, adif).unwrap(), "Wavelog: logged");
    }

    #[test]
    fn a_certificate_other_than_the_pinned_one_is_refused() {
        let p = probe(&cfg(server(true), &"00".repeat(32)));
        assert!(!p.ok);
        assert!(p.message.contains("CHANGED"), "{}", p.message);
        assert_eq!(p.untrusted_cert.as_deref(), Some(tls_pin::sha256_hex(CERT).as_str()));
    }

    #[test]
    fn an_unreachable_server_is_worth_retrying() {
        // Bound and dropped: a port with certainly nothing listening on it.
        let port = TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
        let f = upload(&cfg(format!("http://127.0.0.1:{port}"), ""), 4, "<CALL:5>EA1AA <EOR>")
            .unwrap_err();
        assert!(f.retryable, "{}", f.message);
    }
}
