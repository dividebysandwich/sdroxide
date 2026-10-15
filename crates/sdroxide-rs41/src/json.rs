//! Map rs1729 `*mod --json` lines into [`Package`].

use sdroxide_types::SondeKind;

use crate::package::Package;

/// Parse one JSON object from a `*mod` stdout line.
///
/// Returns `None` when the line is not usable telemetry (empty serial, DFM
/// placeholder serial, missing id, or non-object JSON).
pub fn parse_mod_json(line: &str, kind: SondeKind) -> Option<Package> {
    let data: serde_json::Value = serde_json::from_str(line.trim()).ok()?;
    let obj = data.as_object()?;

    let serial = obj
        .get("id")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())?
        .to_string();
    if kind.is_dfm() && serial.eq_ignore_ascii_case("xxxxxxxx") {
        return None;
    }

    let subtype = obj
        .get("subtype")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    let frame = obj.get("frame").and_then(|v| {
        v.as_u64()
            .map(|n| n as u32)
            .or_else(|| v.as_f64().map(|n| n as u32))
            .or_else(|| v.as_i64().map(|n| n.max(0) as u32))
    });

    Some(Package {
        serial,
        kind,
        subtype,
        manufacturer: kind.manufacturer().to_string(),
        frame,
        lat: num_f64(obj.get("lat")),
        lon: num_f64(obj.get("lon")),
        altitude_m: num_f32(obj.get("alt")),
        ascent_rate_mps: num_f32(obj.get("vel_v")),
        vel_h_mps: num_f32(obj.get("vel_h")),
        heading_deg: num_f32(obj.get("heading")),
        temperature_c: num_f32(obj.get("temp")),
        humidity_pct: num_f32(obj.get("humidity")),
        pressure_hpa: num_f32(obj.get("pressure")),
        battery_v: num_f32(obj.get("batt")),
        sats: obj.get("sats").and_then(|v| {
            v.as_u64()
                .map(|n| n.min(255) as u8)
                .or_else(|| v.as_i64().map(|n| n.clamp(0, 255) as u8))
        }),
        datetime: obj.get("datetime").and_then(|v| v.as_str()).map(str::to_string),
    })
}

fn num_f64(v: Option<&serde_json::Value>) -> Option<f64> {
    v.and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|n| n as f64)))
}

fn num_f32(v: Option<&serde_json::Value>) -> Option<f32> {
    num_f64(v).map(|n| n as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rs41_json() {
        let line = r#"{"type":"RS41","frame":4024,"id":"X2922759","lat":52.73,"lon":21.30,"alt":15107.17,"temp":-52.7,"humidity":18.7,"pressure":119.23,"subtype":"RS41-SGP","vel_v":5.0,"vel_h":10.0,"heading":63.1,"sats":5,"batt":2.8}"#;
        let p = parse_mod_json(line, SondeKind::Rs41).unwrap();
        assert_eq!(p.serial, "X2922759");
        assert_eq!(p.frame, Some(4024));
        assert_eq!(p.subtype.as_deref(), Some("RS41-SGP"));
        assert!((p.lat.unwrap() - 52.73).abs() < 1e-6);
        assert_eq!(p.sats, Some(5));
    }

    #[test]
    fn skips_dfm_placeholder_serial() {
        let line = r#"{"type":"DFM","id":"xxxxxxxx","lat":1.0,"lon":2.0,"alt":100.0}"#;
        assert!(parse_mod_json(line, SondeKind::Dfm9).is_none());
    }

    #[test]
    fn skips_empty_id() {
        let line = r#"{"type":"RS41","id":"","lat":1.0,"lon":2.0}"#;
        assert!(parse_mod_json(line, SondeKind::Rs41).is_none());
    }
}
