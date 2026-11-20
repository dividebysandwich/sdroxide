//! Parks On The Air (POTA) activator spot feed: `api.pota.app/spot/activator`
//! returns a JSON array of current activator spots.

use sdroxide_types::{Spot, SpotKind};

use crate::http;

const URL: &str = "https://api.pota.app/spot/activator";

/// Fetch and parse the current POTA activator spots.
pub fn fetch(now_utc: i64) -> Result<Vec<Spot>, String> {
    let body = http::get(URL)?;
    parse(&body, now_utc)
}

fn parse(body: &str, now: i64) -> Result<Vec<Spot>, String> {
    let v: serde_json::Value = serde_json::from_str(body).map_err(|e| e.to_string())?;
    let arr = v.as_array().ok_or("unexpected POTA response")?;
    let mut out = Vec::with_capacity(arr.len());
    for e in arr {
        // frequency is kHz as a string, e.g. "14074".
        let freq_khz: f64 = e.get("frequency").and_then(json_num).unwrap_or(0.0);
        if freq_khz <= 0.0 {
            continue;
        }
        let freq_hz = freq_khz * 1000.0;
        let call = str_field(e, "activator").to_ascii_uppercase();
        if call.is_empty() {
            continue;
        }
        let reference = {
            let r = str_field(e, "reference");
            (!r.is_empty()).then_some(r)
        };
        let park = str_field(e, "name");
        let park = if park.is_empty() { str_field(e, "parkName") } else { park };
        let loc_desc = str_field(e, "locationDesc");
        let comment =
            if loc_desc.is_empty() { park.clone() } else { format!("{park} ({loc_desc})") };
        let grid = {
            let g = str_field(e, "grid6");
            let g = if g.is_empty() { str_field(e, "grid4") } else { g };
            (!g.is_empty()).then_some(g)
        };
        let spotter = str_field(e, "spotter");
        // POTA gives the park's lat/lon directly — use it for the map.
        let loc =
            match (e.get("latitude").and_then(json_num), e.get("longitude").and_then(json_num)) {
                (Some(lat), Some(lon)) if lat != 0.0 || lon != 0.0 => Some((lat, lon)),
                _ => None,
            };
        out.push(Spot {
            id: Spot::make_id(SpotKind::Pota, &call, freq_hz),
            kind: SpotKind::Pota,
            freq_hz,
            call,
            spotter,
            mode: str_field(e, "mode").to_ascii_uppercase(),
            comment,
            reference,
            grid,
            loc,
            when_utc: now,
            snr_db: None,
            color: None,
        });
    }
    Ok(out)
}

fn str_field(e: &serde_json::Value, key: &str) -> String {
    e.get(key).and_then(|v| v.as_str()).unwrap_or("").trim().to_string()
}

/// A JSON value that may be a number or a numeric string.
fn json_num(v: &serde_json::Value) -> Option<f64> {
    v.as_f64().or_else(|| v.as_str().and_then(|s| s.trim().parse().ok()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_pota_json() {
        let body = r#"[
          {"activator":"K1ABC","frequency":"14074","mode":"FT8","reference":"K-1234",
           "name":"Some Park","locationDesc":"US-CA","grid6":"CM97","spotter":"W9DEF"}
        ]"#;
        let spots = parse(body, 500).unwrap();
        assert_eq!(spots.len(), 1);
        assert_eq!(spots[0].call, "K1ABC");
        assert_eq!(spots[0].freq_hz, 14_074_000.0);
        assert_eq!(spots[0].reference.as_deref(), Some("K-1234"));
        assert_eq!(spots[0].mode, "FT8");
    }
}
