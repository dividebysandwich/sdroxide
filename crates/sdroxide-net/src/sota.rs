//! Summits On The Air (SOTA) spot feed: `api-db2.sota.org.uk/api/spots/<n>/all`
//! returns a JSON array of recent spots. SOTA frequencies are MHz strings.

use sdroxide_types::{Spot, SpotKind};

use crate::http;

const URL: &str = "https://api-db2.sota.org.uk/api/spots/50/all";

pub fn fetch(now_utc: i64) -> Result<Vec<Spot>, String> {
    let body = http::get(URL)?;
    parse(&body, now_utc)
}

fn parse(body: &str, now: i64) -> Result<Vec<Spot>, String> {
    let v: serde_json::Value = serde_json::from_str(body).map_err(|e| e.to_string())?;
    let arr = v.as_array().ok_or("unexpected SOTA response")?;
    let mut out = Vec::with_capacity(arr.len());
    for e in arr {
        // frequency is MHz as a string, e.g. "14.062".
        let freq_mhz: f64 = e.get("frequency").and_then(json_num).unwrap_or(0.0);
        if freq_mhz <= 0.0 {
            continue;
        }
        let freq_hz = freq_mhz * 1_000_000.0;
        let call = str_field(e, "activatorCallsign").to_ascii_uppercase();
        if call.is_empty() {
            continue;
        }
        let summit = str_field(e, "summitCode");
        let details = str_field(e, "summitDetails");
        let comment = if details.is_empty() { summit.clone() } else { details };
        out.push(Spot {
            id: Spot::make_id(SpotKind::Sota, &call, freq_hz),
            kind: SpotKind::Sota,
            freq_hz,
            call,
            spotter: str_field(e, "callsign"),
            mode: str_field(e, "mode").to_ascii_uppercase(),
            comment,
            reference: (!summit.is_empty()).then_some(summit),
            grid: None,
            loc: None,
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

fn json_num(v: &serde_json::Value) -> Option<f64> {
    v.as_f64().or_else(|| v.as_str().and_then(|s| s.trim().parse().ok()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sota_json() {
        let body = r#"[
          {"activatorCallsign":"G0ABC","frequency":"14.062","mode":"cw",
           "summitCode":"G/LD-001","summitDetails":"Scafell Pike","callsign":"M0XYZ"}
        ]"#;
        let spots = parse(body, 10).unwrap();
        assert_eq!(spots.len(), 1);
        assert_eq!(spots[0].call, "G0ABC");
        assert_eq!(spots[0].freq_hz, 14_062_000.0);
        assert_eq!(spots[0].reference.as_deref(), Some("G/LD-001"));
        assert_eq!(spots[0].mode, "CW");
    }
}
