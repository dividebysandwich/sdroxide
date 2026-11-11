//! Radiosondes other stations have heard, from SondeHub's public API: the
//! sondes within a radius of the station, drawn beside the ones this radio
//! decodes. Read-only: **nothing is uploaded**, and nothing is asked until the
//! operator switches it on.
//!
//! Who asks depends on where this window runs:
//!
//! * connected to a station (a remote client, or the web page the station
//!   serves): `GET {station}/sonde-online?lat=..&lon=..` — the station asks
//!   SondeHub once for all its clients (`sdroxide-server`), a native client
//!   signs in with the password it already holds, and the answer is a short
//!   JSON list. A browser cannot reach SondeHub itself, so this is the only
//!   way there for the web page;
//! * on the machine with the radio, no station in between: one `GET` of
//!   `https://api.v2.sondehub.org/sondes?lat=..&lon=..&distance=..`.
//!
//! Either way it is asked every [`REFRESH_S`] seconds while the switch is on,
//! with the station's locator rounded to a tenth of a degree (about 11 km) so
//! the exact place stays with the operator. The reply is a few kilobytes, kept
//! in memory while the switch is on and dropped when it goes off.

use std::sync::mpsc::{Receiver, channel};

pub use sdroxide_types::sonde_online::OnlineSonde;
#[cfg(not(target_arch = "wasm32"))]
use sdroxide_types::sonde_online::{flight_url, parse, parse_flight, url};

/// How often the list is asked for again.
pub const REFRESH_S: i64 = 60;
/// After a failed ask the next one comes this soon, not after a whole minute:
/// a hiccup (a busy station, a dropped connection) must not leave the list
/// and the cards dead until the minute is up.
pub const RETRY_S: i64 = 15;
/// A list older than this is asked for again when a sonde is picked.
pub const HURRY_AFTER_S: i64 = 20;
/// The most bytes read of a reply. A whole flight from SondeHub is every
/// frame of the sonde (about 7 MB after three hours), far more than the list.
#[cfg(not(target_arch = "wasm32"))]
const MAX_BYTES: u64 = 32 * 1024 * 1024;

/// The station's address for the list, the locator rounded to 0.1°.
pub fn station_url(peer: &str, lat: f64, lon: f64) -> Option<String> {
    let base = crate::login::http_base(&crate::login::station_key(peer))?;
    Some(format!("{base}/sonde-online?lat={lat:.1}&lon={lon:.1}"))
}

/// The station's address for one sonde's flight.
pub fn station_flight_url(peer: &str, serial: &str) -> Option<String> {
    if !sdroxide_types::sonde_online::serial_ok(serial) {
        return None;
    }
    let base = crate::login::http_base(&crate::login::station_key(peer))?;
    Some(format!("{base}/sonde-track?serial={serial}"))
}

/// The station's answer for a flight: `[[lat, lon], ...]`, or nothing usable.
pub fn parse_station_flight(body: &str) -> Option<Vec<(f64, f64)>> {
    let v: Vec<(f64, f64)> = serde_json::from_str(body).ok()?;
    v.iter().all(|(a, b)| sdroxide_types::sonde_online::place_ok(*a, *b)).then_some(v)
}

/// The station's answer: a JSON array of sondes, or nothing usable.
pub fn parse_station(body: &str) -> Option<Vec<OnlineSonde>> {
    let mut v: Vec<OnlineSonde> = serde_json::from_str(body).ok()?;
    v.sort_by_key(|s| std::cmp::Reverse(s.unix));
    Some(v)
}

/// Why an ask got no list.
#[derive(Debug, Clone, PartialEq)]
pub enum Why {
    /// The station wants the operator signed in.
    SignIn,
    /// The station is asking SondeHub as often as it may.
    Busy,
    /// The station could not reach SondeHub.
    Upstream,
    /// The answer was not a list.
    Unreadable,
    /// This window has no station and no way out of its own (the browser build).
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    NoStation,
    /// Any other status.
    Http(u16),
    /// The network library's own words (no connection, timeout…).
    Net(String),
}

impl Why {
    /// The line for the hover text.
    pub fn text(&self) -> String {
        use crate::i18n::tr;
        match self {
            Why::SignIn => tr!("sonde-online-why-signin"),
            Why::Busy => tr!("sonde-online-why-busy"),
            Why::Upstream => tr!("sonde-online-why-upstream"),
            Why::Unreadable => tr!("sonde-online-why-unreadable"),
            Why::NoStation => tr!("sonde-online-why-no-station"),
            Why::Http(s) => tr!("sonde-online-why-http", n = *s),
            Why::Net(e) => e.clone(),
        }
    }

    fn of_status(s: u16) -> Why {
        match s {
            401 => Why::SignIn,
            429 => Why::Busy,
            502 => Why::Upstream,
            s => Why::Http(s),
        }
    }
}

/// What an ask came back with.
enum Reply {
    Sondes(Vec<OnlineSonde>),
    Failed(Why),
}

/// The switch, the list and the asking.
#[derive(Default)]
pub struct Online {
    pub on: bool,
    pub sondes: Vec<OnlineSonde>,
    /// The last ask got no usable answer.
    pub failed: bool,
    /// Why, for the hover text.
    pub why: Option<Why>,
    last_ask: i64,
    inflight: Option<Receiver<Reply>>,
}

impl Online {
    /// Called every frame: collect an answer, and ask again when it is time.
    /// With the switch off, or no station position, nothing is asked and
    /// nothing is kept. `peer` is the station this window is connected to.
    pub fn tick(
        &mut self,
        ctx: &eframe::egui::Context,
        home: Option<(f64, f64)>,
        now: i64,
        peer: Option<&str>,
    ) {
        if let Some(rx) = &self.inflight {
            match rx.try_recv() {
                Ok(Reply::Sondes(v)) => {
                    self.sondes = v;
                    self.failed = false;
                    self.why = None;
                    self.inflight = None;
                }
                Ok(Reply::Failed(why)) => {
                    self.failed = true;
                    self.why = Some(why);
                    self.inflight = None;
                    self.last_ask = now - REFRESH_S + RETRY_S;
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.failed = true;
                    self.why = Some(Why::Net("the request stopped".into()));
                    self.inflight = None;
                    self.last_ask = now - REFRESH_S + RETRY_S;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
            }
        }
        let Some((lat, lon)) = home.filter(|_| self.on) else {
            self.sondes.clear();
            self.failed = false;
            self.why = None;
            self.inflight = None;
            self.last_ask = 0;
            return;
        };
        if self.inflight.is_none() && now - self.last_ask >= REFRESH_S {
            self.last_ask = now;
            self.ask(ctx, lat, lon, peer);
        }
    }

    /// Ask again soon when the list is older than [`HURRY_AFTER_S`]: a sonde was
    /// just picked, and its card should not wait for the minute to be up.
    pub fn hurry(&mut self, now: i64) {
        if self.on && self.inflight.is_none() && now - self.last_ask >= HURRY_AFTER_S {
            self.last_ask = 0;
        }
    }

    fn ask(&mut self, ctx: &eframe::egui::Context, lat: f64, lon: f64, peer: Option<&str>) {
        let (tx, rx) = channel();
        self.inflight = Some(rx);
        let ctx = ctx.clone();
        let via =
            peer.and_then(|p| station_url(p, lat, lon).map(|u| (u, crate::login::station_key(p))));
        #[cfg(not(target_arch = "wasm32"))]
        std::thread::spawn(move || {
            let reply = match via {
                Some((address, station)) => fetch_via_station(&address, &station),
                None => fetch(&url(lat, lon), None).map(|b| parse(&b)),
            };
            let _ = tx.send(match reply {
                Ok(v) => Reply::Sondes(v),
                Err(why) => Reply::Failed(why),
            });
            ctx.request_repaint();
        });
        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(async move {
            let reply = match via {
                Some((address, _)) => web_fetch(&address).await,
                None => Err(Why::NoStation),
            };
            let _ = tx.send(match reply {
                Ok(v) => Reply::Sondes(v),
                Err(why) => Reply::Failed(why),
            });
            ctx.request_repaint();
        });
    }
}

/// What a flight ask came back with: the places, oldest first, or why not.
type FlightReply = Result<Vec<(f64, f64)>, Why>;

/// The flight so far of the sonde whose card is open, from SondeHub: where a
/// sonde heard only by other stations has been, drawn like a radio's own track.
/// Asked once when the card opens, then again every [`REFRESH_S`] while it stays
/// open; dropped when the card closes.
#[derive(Default)]
pub struct Flight {
    serial: Option<String>,
    /// The places, oldest first.
    pub points: Vec<(f64, f64)>,
    last_ask: i64,
    inflight: Option<Receiver<FlightReply>>,
}

impl Flight {
    /// The places for `serial`, or none when it is not the open card's.
    pub fn of(&self, serial: &str) -> &[(f64, f64)] {
        if self.serial.as_deref() == Some(serial) { &self.points } else { &[] }
    }

    /// Called every frame with the serial of the open card (`None` when no card
    /// is open or the switch is off): collect an answer, ask when it is time.
    pub fn tick(
        &mut self,
        ctx: &eframe::egui::Context,
        want: Option<&str>,
        now: i64,
        peer: Option<&str>,
    ) {
        if self.serial.as_deref() != want {
            self.serial = want.map(str::to_string);
            self.points.clear();
            self.inflight = None;
            self.last_ask = 0;
        }
        if let Some(rx) = &self.inflight {
            match rx.try_recv() {
                Ok(Ok(v)) => {
                    // An empty answer keeps what is drawn: the flight may be
                    // gone from SondeHub while the radio's own track is not.
                    if !v.is_empty() {
                        self.points = v;
                    }
                    self.inflight = None;
                }
                Ok(Err(_)) | Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.inflight = None;
                    self.last_ask = now - REFRESH_S + RETRY_S;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
            }
        }
        let Some(serial) = want else { return };
        if self.inflight.is_none() && now - self.last_ask >= REFRESH_S {
            self.last_ask = now;
            self.ask(ctx, serial, peer);
        }
    }

    fn ask(&mut self, ctx: &eframe::egui::Context, serial: &str, peer: Option<&str>) {
        let (tx, rx) = channel();
        self.inflight = Some(rx);
        let ctx = ctx.clone();
        let serial = serial.to_string();
        let via = peer.and_then(|p| {
            station_flight_url(p, &serial).map(|u| (u, crate::login::station_key(p)))
        });
        #[cfg(not(target_arch = "wasm32"))]
        std::thread::spawn(move || {
            let reply = match via {
                Some((address, station)) => station_body(&address, &station)
                    .and_then(|b| parse_station_flight(&b).ok_or(Why::Unreadable)),
                None => match flight_url(&serial) {
                    Some(address) => fetch(&address, None).map(|b| parse_flight(&b)),
                    None => Err(Why::Unreadable),
                },
            };
            let _ = tx.send(reply);
            ctx.request_repaint();
        });
        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(async move {
            let reply = match via {
                Some((address, _)) => web_text(&address)
                    .await
                    .and_then(|t| parse_station_flight(&t).ok_or(Why::Unreadable)),
                None => Err(Why::NoStation),
            };
            let _ = tx.send(reply);
            ctx.request_repaint();
        });
    }
}

/// The station's cookie for each station address, for the length of the run.
#[cfg(not(target_arch = "wasm32"))]
static COOKIES: std::sync::LazyLock<std::sync::Mutex<std::collections::BTreeMap<String, String>>> =
    std::sync::LazyLock::new(Default::default);

#[cfg(not(target_arch = "wasm32"))]
fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_connect(Some(std::time::Duration::from_secs(10)))
        .timeout_global(Some(std::time::Duration::from_secs(30)))
        .http_status_as_error(false)
        .user_agent(concat!("sdroxide/", env!("CARGO_PKG_VERSION")))
        .build()
        .into()
}

/// One `GET`: the body of a 200, or why there is none.
#[cfg(not(target_arch = "wasm32"))]
fn fetch(url: &str, cookie: Option<&str>) -> Result<String, Why> {
    get(&agent(), url, cookie).and_then(|(status, body)| match status {
        200 => Ok(body),
        s => Err(Why::of_status(s)),
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn get(agent: &ureq::Agent, url: &str, cookie: Option<&str>) -> Result<(u16, String), Why> {
    let mut req = agent.get(url).header("Accept", "application/json");
    if let Some(c) = cookie {
        req = req.header("Cookie", c);
    }
    let mut r = req.call().map_err(|e| Why::Net(e.to_string()))?;
    let status = r.status().as_u16();
    let body = r.body_mut().with_config().limit(MAX_BYTES).read_to_string().unwrap_or_default();
    Ok((status, body))
}

/// The same trade a browser makes: the password this client already signed in
/// with for the station's cookie.
#[cfg(not(target_arch = "wasm32"))]
fn sign_in(agent: &ureq::Agent, station: &str) -> Option<String> {
    let access = crate::login::session_access(station)?;
    let url = format!("{}/signin", crate::login::http_base(station)?);
    let body = serde_json::json!({
        "username": access.username,
        "password": access.password,
        "remember": false,
    })
    .to_string();
    let resp = agent.post(&url).header("content-type", "application/json").send(body).ok()?;
    if resp.status().as_u16() != 200 {
        return None;
    }
    let set = resp.headers().get("set-cookie")?.to_str().ok()?;
    Some(set.split(';').next()?.trim().to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn fetch_via_station(address: &str, station: &str) -> Result<Vec<OnlineSonde>, Why> {
    station_body(address, station).and_then(|b| parse_station(&b).ok_or(Why::Unreadable))
}

/// The body of the station's 200 for `address`, signing in once if it asks.
#[cfg(not(target_arch = "wasm32"))]
fn station_body(address: &str, station: &str) -> Result<String, Why> {
    let agent = agent();
    let cookie = COOKIES.lock().unwrap_or_else(|e| e.into_inner()).get(station).cloned();
    let mut got = get(&agent, address, cookie.as_deref())?;
    if got.0 == 401
        && let Some(c) = sign_in(&agent, station)
    {
        COOKIES.lock().unwrap_or_else(|e| e.into_inner()).insert(station.to_string(), c.clone());
        got = get(&agent, address, Some(&c))?;
    }
    match got {
        (200, body) => Ok(body),
        (s, _) => Err(Why::of_status(s)),
    }
}

/// The browser's ask: same origin, with the cookie it already holds.
#[cfg(target_arch = "wasm32")]
async fn web_fetch(address: &str) -> Result<Vec<OnlineSonde>, Why> {
    web_text(address).await.and_then(|t| parse_station(&t).ok_or(Why::Unreadable))
}

/// The text of the same-origin answer to `address`.
#[cfg(target_arch = "wasm32")]
async fn web_text(address: &str) -> Result<String, Why> {
    use wasm_bindgen::JsCast;
    let window = web_sys::window().ok_or(Why::NoStation)?;
    let init = web_sys::RequestInit::new();
    init.set_method("GET");
    init.set_mode(web_sys::RequestMode::SameOrigin);
    let request =
        web_sys::Request::new_with_str_and_init(address, &init).map_err(|_| Why::Unreadable)?;
    let response = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|_| Why::Net("no answer".into()))?
        .dyn_into::<web_sys::Response>()
        .map_err(|_| Why::Unreadable)?;
    match response.status() {
        200 => {}
        s => return Err(Why::of_status(s)),
    }
    let text = wasm_bindgen_futures::JsFuture::from(response.text().map_err(|_| Why::Unreadable)?)
        .await
        .ok()
        .and_then(|t| t.as_string())
        .unwrap_or_default();
    if text.len() as u64 > 4 * 1024 * 1024 {
        return Err(Why::Unreadable);
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sonde(serial: &str, unix: i64) -> OnlineSonde {
        OnlineSonde {
            serial: serial.into(),
            model: "RS41".into(),
            lat: 48.0,
            lon: 2.0,
            alt_m: Some(1000.0),
            climb_ms: None,
            speed_ms: None,
            freq_mhz: Some(405.8),
            unix,
            ..Default::default()
        }
    }

    #[test]
    fn the_stations_address_names_the_rounded_place_and_nothing_else() {
        let u = station_url("wss://radio.example/ws/2", 48.8566, 2.3522).unwrap();
        assert_eq!(u, "https://radio.example/sonde-online?lat=48.9&lon=2.4");
        assert_eq!(
            station_url("ws://192.168.1.5:8080/ws", 1.0, -2.0).unwrap(),
            "http://192.168.1.5:8080/sonde-online?lat=1.0&lon=-2.0"
        );
        assert!(station_url("not an address", 1.0, 2.0).is_none());
    }

    #[test]
    fn the_stations_list_is_read_newest_first_and_nonsense_is_refused() {
        let json =
            serde_json::to_string(&vec![sonde("A", 10), sonde("B", 30), sonde("C", 20)]).unwrap();
        let v = parse_station(&json).unwrap();
        assert_eq!(v.iter().map(|s| s.serial.as_str()).collect::<Vec<_>>(), ["B", "C", "A"]);
        assert!(parse_station("").is_none() && parse_station("{}").is_none());
        assert_eq!(parse_station("[]"), Some(Vec::new()));
    }

    #[test]
    fn with_the_switch_off_nothing_is_kept() {
        let mut o = Online { sondes: vec![sonde("A", 1)], failed: true, ..Default::default() };
        o.tick(&eframe::egui::Context::default(), Some((48.0, 2.0)), 1_000, None);
        assert!(o.sondes.is_empty() && !o.failed, "off clears the list and the failure");
        assert!(o.inflight.is_none(), "and asks nothing");
    }

    #[test]
    fn picking_a_sonde_hurries_a_stale_list_but_not_a_fresh_one() {
        let mut o = Online { on: true, last_ask: 1_000, ..Default::default() };
        o.hurry(1_010);
        assert_eq!(o.last_ask, 1_000, "asked 10 s ago: left alone");
        o.hurry(1_000 + HURRY_AFTER_S);
        assert_eq!(o.last_ask, 0, "old enough: the next tick asks");
        let mut off = Online { last_ask: 5, ..Default::default() };
        off.hurry(1_000);
        assert_eq!(off.last_ask, 5, "switched off: nothing to hurry");
    }

    #[test]
    fn after_a_failed_ask_the_next_comes_soon_not_after_a_minute() {
        let (tx, rx) = channel();
        let mut o = Online { on: true, last_ask: 1_000, inflight: Some(rx), ..Default::default() };
        tx.send(Reply::Failed(Why::Busy)).unwrap();
        o.tick(&eframe::egui::Context::default(), Some((48.0, 2.0)), 1_010, None);
        assert!(o.failed && o.inflight.is_none());
        assert_eq!(o.last_ask, 1_010 - REFRESH_S + RETRY_S);
    }

    #[test]
    fn a_flight_belongs_to_the_open_card_and_goes_with_it() {
        let mut f = Flight::default();
        let ctx = eframe::egui::Context::default();
        f.tick(&ctx, None, 1_000, None);
        assert!(f.of("A").is_empty());
        f.serial = Some("A".into());
        f.points = vec![(1.0, 2.0)];
        assert_eq!(f.of("A"), &[(1.0, 2.0)]);
        assert!(f.of("B").is_empty(), "another sonde's card gets nothing");
        f.tick(&ctx, None, 1_001, None);
        assert!(f.points.is_empty() && f.of("A").is_empty(), "closing the card drops it");
    }

    #[test]
    fn the_stations_flight_is_read_and_nonsense_is_refused() {
        assert_eq!(
            parse_station_flight("[[48.0,4.0],[49.0,5.0]]"),
            Some(vec![(48.0, 4.0), (49.0, 5.0)])
        );
        assert!(parse_station_flight("[[95.0,4.0]]").is_none());
        assert!(parse_station_flight("{}").is_none());
        assert_eq!(
            station_flight_url("wss://radio.example/ws/2", "W4871102").unwrap(),
            "https://radio.example/sonde-track?serial=W4871102"
        );
        assert!(station_flight_url("wss://radio.example/ws/2", "a/b").is_none());
    }

    #[test]
    fn with_no_station_position_nothing_is_asked() {
        let mut o = Online { on: true, ..Default::default() };
        o.tick(&eframe::egui::Context::default(), None, 1_000, Some("ws://x/ws"));
        assert!(o.inflight.is_none());
    }

    /// Needs the network: `cargo test -p sdroxide-ui --lib sonde_online -- --ignored`.
    #[test]
    #[ignore]
    fn the_real_api_answers() {
        let body = fetch(&url(48.86, 2.35), None).expect("SondeHub answers");
        assert!(!parse(&body).is_empty());
    }
}
