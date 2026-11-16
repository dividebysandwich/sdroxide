//! Where an ADS-B flight is going, from its callsign: the route the callsign is
//! known to fly, with the airports' positions for the map.
//!
//! ADS-B itself carries no flight plan — only who, where, how high, how fast.
//! What can be had freely is the *usual* route of a callsign, from the
//! community-kept "standing data" of Virtual Radar Server
//! (`github.com/vradarserver/standing-data`, CC0-1.0): `EWG1003 → EDDT-EDDL`.
//! It is what the callsign normally flies, not the plan filed today; a charter,
//! a diversion or a reused callsign will not be in it.
//!
//! The data comes in small files, one per airline code (`routes/schema-01/E/
//! EWG-all.csv`; an airline with more than 10,000 routes is split by the first
//! digit of the flight number, `RYR-1.csv`…), and the airports in one file per
//! two-letter ICAO prefix (`airports/schema-01/E/ED.csv`). Only the files for
//! the airlines and airports actually heard are fetched, off the UI thread,
//! and kept on disk for [`MAX_AGE_S`]; nothing is sent but the file name.
//! The browser build has no files and no direct network, so it shows no route.

use std::collections::HashMap;

/// One airport of a route, as the map needs it.
#[derive(Debug, Clone, PartialEq)]
pub struct Airport {
    pub icao: String,
    pub name: String,
    pub lat: f64,
    pub lon: f64,
}

/// Cached files are fetched again after a week: routes change with the
/// timetable, slowly.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
const MAX_AGE_S: u64 = 7 * 24 * 3600;

/// A failed fetch (no network, a timeout) is tried again after this long.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
const RETRY_S: u64 = 120;

/// At most this many files are asked for at once.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
const MAX_IN_FLIGHT: usize = 4;

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
const BASE: &str = "https://raw.githubusercontent.com/vradarserver/standing-data/main";

/// The callsign as the standing data keys it: `(code, number)`, the airline's
/// three-letter ICAO code and the flight number without leading zeros.
///
/// The data's own rule (`routes/schema-01/README.md`): the number is one to four
/// characters after trimming, the last one or two may be letters, and a number
/// that trims to nothing or to letters only gets one `0` back. A callsign that
/// is not of that shape — a registration such as `DEABC`, a two-letter IATA
/// code — has no route there.
pub fn normalize(callsign: &str) -> Option<(String, String)> {
    let cs = callsign.trim().to_ascii_uppercase();
    let b = cs.as_bytes();
    if b.len() < 4 || !b[..3].iter().all(u8::is_ascii_alphabetic) || !b[3].is_ascii_digit() {
        return None;
    }
    let code = &cs[..3];
    let num = cs[3..].trim_start_matches('0');
    let num = if num.is_empty() || !num.as_bytes()[0].is_ascii_digit() {
        format!("0{num}")
    } else {
        num.to_string()
    };
    let nb = num.as_bytes();
    let letters = nb.iter().rev().take_while(|c| c.is_ascii_alphabetic()).count();
    let ok = (1..=4).contains(&nb.len())
        && letters <= 2
        && nb[..nb.len() - letters].iter().all(u8::is_ascii_digit)
        && nb.len() > letters;
    ok.then(|| (code.to_string(), num))
}

/// `routes/schema-01/<c>/<file>.csv` for a route file.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
fn route_path(file: &str) -> String {
    format!("routes/schema-01/{}/{file}.csv", &file[..1])
}

/// `airports/schema-01/<c>/<prefix>.csv` for an airport file.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
fn airport_path(prefix: &str) -> String {
    format!("airports/schema-01/{}/{prefix}.csv", &prefix[..1])
}

/// The fields of one CSV line; quoted fields may hold commas.
fn fields(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if quoted && chars.peek() == Some(&'"') => {
                cur.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            ',' if !quoted => out.push(std::mem::take(&mut cur)),
            _ => cur.push(c),
        }
    }
    out.push(cur);
    out
}

/// A route file: normalised callsign → its airports, in order.
pub fn parse_routes(body: &str) -> HashMap<String, Vec<String>> {
    let mut lines = body.trim_start_matches('\u{feff}').lines();
    let head = fields(lines.next().unwrap_or_default());
    let col = |name: &str| head.iter().position(|h| h.trim() == name);
    let (Some(ci), Some(ai)) = (col("Callsign"), col("AirportCodes")) else {
        return HashMap::new();
    };
    lines
        .filter_map(|l| {
            let f = fields(l);
            let cs = f.get(ci)?.trim().to_ascii_uppercase();
            let ap: Vec<String> = f
                .get(ai)?
                .split('-')
                .map(|s| s.trim().to_ascii_uppercase())
                .filter(|s| !s.is_empty())
                .collect();
            (ap.len() >= 2).then_some((cs, ap))
        })
        .collect()
}

/// An airport file: ICAO code → airport.
pub fn parse_airports(body: &str) -> HashMap<String, Airport> {
    let mut lines = body.trim_start_matches('\u{feff}').lines();
    let head = fields(lines.next().unwrap_or_default());
    let col = |name: &str| head.iter().position(|h| h.trim() == name);
    let (Some(ii), Some(ni), Some(la), Some(lo)) =
        (col("ICAO"), col("Name"), col("Latitude"), col("Longitude"))
    else {
        return HashMap::new();
    };
    lines
        .filter_map(|l| {
            let f = fields(l);
            let icao = f.get(ii)?.trim().to_ascii_uppercase();
            let lat: f64 = f.get(la)?.trim().parse().ok()?;
            let lon: f64 = f.get(lo)?.trim().parse().ok()?;
            let ok = !icao.is_empty() && lat.abs() <= 90.0 && lon.abs() <= 180.0;
            ok.then(|| {
                let name = f.get(ni).map(|s| s.trim().to_string()).unwrap_or_default();
                (icao.clone(), Airport { icao, name, lat, lon })
            })
        })
        .collect()
}

/// What is known of one file.
enum Slot<T> {
    /// Asked for, not answered yet.
    Pending,
    /// There, possibly empty.
    Ready(T),
    /// The data has no such file.
    Absent,
    /// The fetch failed at this many seconds since the epoch; asked again
    /// after [`RETRY_S`].
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    Failed(u64),
}

/// One file to fetch.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Job {
    Routes(String),
    Airports(String),
}

/// The answer for a [`Job`]: the body, `None` for "no such file", or an error.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
type Done = (Job, Result<Option<String>, String>);

/// Routes and airports fetched so far, and the fetches under way.
pub struct RouteBook {
    routes: HashMap<String, Slot<HashMap<String, Vec<String>>>>,
    airports: HashMap<String, Slot<HashMap<String, Airport>>>,
    /// Airline codes whose routes are split by the flight number's first digit.
    split: std::collections::HashSet<String>,
    queue: std::collections::VecDeque<Job>,
    in_flight: usize,
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    tx: std::sync::mpsc::Sender<Done>,
    rx: std::sync::mpsc::Receiver<Done>,
}

impl Default for RouteBook {
    fn default() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        RouteBook {
            routes: HashMap::new(),
            airports: HashMap::new(),
            split: Default::default(),
            queue: Default::default(),
            in_flight: 0,
            tx,
            rx,
        }
    }
}

impl RouteBook {
    /// The route of `callsign`, airport codes in order, once its file is in.
    /// Asks for the file the first time; `None` until it has arrived, and for
    /// a callsign the data does not know.
    pub fn route(&mut self, callsign: &str) -> Option<Vec<String>> {
        let (code, num) = normalize(callsign)?;
        let file = if self.split.contains(&code) {
            format!("{code}-{}", &num[..1])
        } else {
            format!("{code}-all")
        };
        match self.routes.get(&file) {
            Some(Slot::Ready(m)) => m.get(&format!("{code}{num}")).cloned(),
            Some(Slot::Pending | Slot::Absent) => None,
            Some(Slot::Failed(at)) if now_s() < at + RETRY_S => None,
            _ => {
                self.routes.insert(file.clone(), Slot::Pending);
                self.queue.push_back(Job::Routes(file));
                None
            }
        }
    }

    /// The airport with this ICAO code, once its file is in; asks for it the
    /// first time.
    pub fn airport(&mut self, icao: &str) -> Option<Airport> {
        if icao.len() < 2 || !icao.is_ascii() {
            return None;
        }
        let prefix = icao[..2].to_ascii_uppercase();
        match self.airports.get(&prefix) {
            Some(Slot::Ready(m)) => m.get(icao).cloned(),
            Some(Slot::Pending | Slot::Absent) => None,
            Some(Slot::Failed(at)) if now_s() < at + RETRY_S => None,
            _ => {
                self.airports.insert(prefix.clone(), Slot::Pending);
                self.queue.push_back(Job::Airports(prefix));
                None
            }
        }
    }

    /// Take in what has arrived and start what is queued. Once a frame.
    pub fn poll(&mut self, ctx: &eframe::egui::Context) {
        while let Ok((job, res)) = self.rx.try_recv() {
            self.in_flight = self.in_flight.saturating_sub(1);
            self.take(job, res);
        }
        #[cfg(not(target_arch = "wasm32"))]
        while self.in_flight < MAX_IN_FLIGHT {
            let Some(job) = self.queue.pop_front() else { break };
            self.in_flight += 1;
            let tx = self.tx.clone();
            let ctx = ctx.clone();
            let spawned =
                std::thread::Builder::new().name("sdroxide-adsb-route".into()).spawn(move || {
                    let res = fetch(&job);
                    let _ = tx.send((job, res));
                    ctx.request_repaint();
                });
            if spawned.is_err() {
                self.in_flight -= 1;
                break;
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = ctx;
            self.queue.clear();
        }
    }

    fn take(&mut self, job: Job, res: Result<Option<String>, String>) {
        match job {
            Job::Routes(file) => {
                let slot = match res {
                    Ok(Some(body)) => Slot::Ready(parse_routes(&body)),
                    Ok(None) => {
                        // No `<code>-all`: either the airline is unknown, or it
                        // has so many routes they are split by first digit.
                        // Pitfall: the split files are only tried after the
                        //          `-all` file is known to be missing, so a big
                        //          airline costs one extra request per run.
                        if let Some(code) = file.strip_suffix("-all") {
                            self.split.insert(code.to_string());
                        }
                        Slot::Absent
                    }
                    Err(e) => {
                        tracing::debug!("ADS-B route file {file}: {e}");
                        Slot::Failed(now_s())
                    }
                };
                self.routes.insert(file, slot);
            }
            Job::Airports(prefix) => {
                let slot = match res {
                    Ok(Some(body)) => Slot::Ready(parse_airports(&body)),
                    Ok(None) => Slot::Absent,
                    Err(e) => {
                        tracing::debug!("ADS-B airport file {prefix}: {e}");
                        Slot::Failed(now_s())
                    }
                };
                self.airports.insert(prefix, slot);
            }
        }
    }
}

fn now_s() -> u64 {
    crate::time::now_unix().max(0) as u64
}

/// Where a file is kept between runs.
#[cfg(not(target_arch = "wasm32"))]
fn cache_file(job: &Job) -> Option<std::path::PathBuf> {
    let dir = sdroxide_config::config_dir().ok()?.join("adsb-routes");
    Some(match job {
        Job::Routes(f) => dir.join("routes").join(format!("{f}.csv")),
        Job::Airports(p) => dir.join("airports").join(format!("{p}.csv")),
    })
}

/// The file from the disk if fresh enough, else from the network (and then
/// onto the disk). An empty file on disk stands for "the data has none".
#[cfg(not(target_arch = "wasm32"))]
fn fetch(job: &Job) -> Result<Option<String>, String> {
    let cached = cache_file(job);
    if let Some(p) = &cached
        && let Ok(meta) = std::fs::metadata(p)
        && meta.modified().ok().and_then(|m| m.elapsed().ok()).map(|a| a.as_secs())
            < Some(MAX_AGE_S)
    {
        let body = std::fs::read_to_string(p).map_err(|e| e.to_string())?;
        return Ok((!body.is_empty()).then_some(body));
    }
    let path = match job {
        Job::Routes(f) => route_path(f),
        Job::Airports(p) => airport_path(p),
    };
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_connect(Some(std::time::Duration::from_secs(10)))
        .timeout_global(Some(std::time::Duration::from_secs(30)))
        .http_status_as_error(false)
        .user_agent(concat!("sdroxide/", env!("CARGO_PKG_VERSION")))
        .build()
        .into();
    let mut r = agent.get(&format!("{BASE}/{path}")).call().map_err(|e| e.to_string())?;
    let body = match r.status().as_u16() {
        200 => Some(
            r.body_mut()
                .with_config()
                .limit(16 * 1024 * 1024)
                .read_to_string()
                .map_err(|e| e.to_string())?,
        ),
        404 => None,
        s => return Err(format!("HTTP {s}")),
    };
    if let Some(p) = &cached {
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(p, body.as_deref().unwrap_or_default());
    }
    Ok(body)
}

/// Points along the great circle from `a` to `b`, both ends included, as
/// `(lat, lon)` in degrees.
pub fn great_circle(a: (f64, f64), b: (f64, f64), steps: usize) -> Vec<(f64, f64)> {
    let to_v = |(lat, lon): (f64, f64)| {
        let (la, lo) = (lat.to_radians(), lon.to_radians());
        [la.cos() * lo.cos(), la.cos() * lo.sin(), la.sin()]
    };
    let (va, vb) = (to_v(a), to_v(b));
    let dot = (va[0] * vb[0] + va[1] * vb[1] + va[2] * vb[2]).clamp(-1.0, 1.0);
    let omega = dot.acos();
    let steps = steps.max(1);
    if omega < 1e-9 {
        return vec![a, b];
    }
    (0..=steps)
        .map(|k| {
            let t = k as f64 / steps as f64;
            let (sa, sb) = (((1.0 - t) * omega).sin(), (t * omega).sin());
            let s = omega.sin();
            let v: Vec<f64> = (0..3).map(|i| (sa * va[i] + sb * vb[i]) / s).collect();
            let lat = v[2].atan2((v[0] * v[0] + v[1] * v[1]).sqrt()).to_degrees();
            let lon = v[1].atan2(v[0]).to_degrees();
            (lat, lon)
        })
        .collect()
}

/// The route as the table shows it: `EDDK›GCLP`, or `EDDK›…›GCLP` with stops.
pub fn short(route: &[String]) -> String {
    match route {
        [] => String::new(),
        [a] => a.clone(),
        [a, b] => format!("{a}›{b}"),
        [a, .., b] => format!("{a}›…›{b}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn callsigns_normalise_as_the_standing_data_says() {
        let n = |s: &str| normalize(s).map(|(c, n)| format!("{c}{n}"));
        assert_eq!(n("EZY1200").as_deref(), Some("EZY1200"));
        assert_eq!(n("EZY0001").as_deref(), Some("EZY1"));
        assert_eq!(n("EZY0000").as_deref(), Some("EZY0"));
        assert_eq!(n("EZY00AB").as_deref(), Some("EZY0AB"));
        assert_eq!(n("ewg33f ").as_deref(), Some("EWG33F"));
        // A registration, an IATA code, a number too long: no route there.
        assert_eq!(n("DEABC"), None);
        assert_eq!(n("U21234"), None);
        assert_eq!(n("DLH12345"), None);
        assert_eq!(n("DLH1ABC"), None);
    }

    #[test]
    fn a_route_file_maps_callsign_to_airports() {
        let body = "\u{feff}Callsign,Code,Number,AirlineCode,AirportCodes\n\
                    EWG1,EWG,1,EWG,EDDK-GCLP\n\
                    EWG7,EWG,7,EWG,EDDL-LEPA-GCLP\n\
                    EWG9,EWG,9,EWG,EDDL\n";
        let m = parse_routes(body);
        assert_eq!(m["EWG1"], vec!["EDDK", "GCLP"]);
        assert_eq!(m["EWG7"], vec!["EDDL", "LEPA", "GCLP"]);
        assert!(!m.contains_key("EWG9"), "one airport is no route");
    }

    #[test]
    fn an_airport_file_gives_name_and_place() {
        let body = "\u{feff}Code,Name,ICAO,IATA,Location,CountryISO2,Latitude,Longitude,AltitudeFeet\n\
                    EDDL,\"Düsseldorf, International\",EDDL,DUS,Düsseldorf,DE,51.289453,6.766775,147\n";
        let m = parse_airports(body);
        let a = &m["EDDL"];
        assert_eq!(a.name, "Düsseldorf, International");
        assert!((a.lat - 51.289453).abs() < 1e-9 && (a.lon - 6.766775).abs() < 1e-9);
    }

    #[test]
    fn files_live_where_the_standing_data_keeps_them() {
        assert_eq!(route_path("EWG-all"), "routes/schema-01/E/EWG-all.csv");
        assert_eq!(route_path("RYR-1"), "routes/schema-01/R/RYR-1.csv");
        assert_eq!(airport_path("ED"), "airports/schema-01/E/ED.csv");
    }

    #[test]
    fn the_great_circle_ends_at_both_airports_and_bows_toward_the_pole() {
        let (fra, jfk) = ((50.03, 8.57), (40.64, -73.78));
        let p = great_circle(fra, jfk, 32);
        assert_eq!(p.len(), 33);
        assert!((p[0].0 - fra.0).abs() < 1e-9 && (p[32].1 - jfk.1).abs() < 1e-9);
        // Frankfurt–New York goes north of both ends, over the Atlantic near 52°+.
        let top = p.iter().map(|q| q.0).fold(f64::MIN, f64::max);
        assert!(top > 51.0, "highest point {top}");
    }

    #[test]
    fn the_table_shortens_a_route_with_stops() {
        let r = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(short(&r(&["EDDK", "GCLP"])), "EDDK›GCLP");
        assert_eq!(short(&r(&["EDDL", "LEPA", "GCLP"])), "EDDL›…›GCLP");
    }

    #[test]
    fn a_missing_all_file_marks_the_airline_as_split() {
        let mut b = RouteBook::default();
        assert_eq!(b.route("RYR123"), None);
        assert_eq!(b.queue.pop_front(), Some(Job::Routes("RYR-all".into())));
        b.take(Job::Routes("RYR-all".into()), Ok(None));
        assert_eq!(b.route("RYR123"), None);
        assert_eq!(b.queue.pop_front(), Some(Job::Routes("RYR-1".into())));
        b.take(
            Job::Routes("RYR-1".into()),
            Ok(Some(
                "Callsign,Code,Number,AirlineCode,AirportCodes\nRYR123,RYR,123,RYR,EDDW-LEMG\n"
                    .into(),
            )),
        );
        assert_eq!(b.route("RYR123"), Some(vec!["EDDW".to_string(), "LEMG".to_string()]));
    }
}
