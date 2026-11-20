//! PSK Reporter reception-report retrieval. Queries the current band's activity
//! (`retrieve.pskreporter.info/query`), returning the *senders* others are
//! hearing as tunable spots. XML parsed with `roxmltree`.
//!
//! PSK Reporter asks clients to query no more than once every few minutes; the
//! poll interval default (300 s) respects that.

use sdroxide_types::{Spot, SpotKind};

use crate::http;

/// Fetch reception reports for the band containing `dial_hz` over the last
/// 15 minutes.
pub fn fetch(dial_hz: f64, now_utc: i64) -> Result<Vec<Spot>, String> {
    let (lo, hi) = band_edges(dial_hz);
    let url = format!(
        "https://retrieve.pskreporter.info/query?flowStartSeconds=-900&rronly=1&frange={}-{}",
        lo as i64, hi as i64
    );
    let body = http::get(&url)?;
    parse(&body, now_utc)
}

fn parse(body: &str, now: i64) -> Result<Vec<Spot>, String> {
    let doc = roxmltree::Document::parse(body).map_err(|e| e.to_string())?;
    let mut out: Vec<Spot> = Vec::new();
    for node in doc.descendants().filter(|n| n.has_tag_name("receptionReport")) {
        let attr = |k: &str| node.attribute(k).unwrap_or("").trim();
        let freq_hz: f64 = match attr("frequency").parse() {
            Ok(f) if f > 0.0 => f,
            _ => continue,
        };
        let call = attr("senderCallsign").to_ascii_uppercase();
        if call.is_empty() {
            continue;
        }
        let grid = {
            let g = attr("senderLocator");
            (!g.is_empty()).then(|| g.to_ascii_uppercase())
        };
        let snr_db = attr("sNR").parse::<i16>().ok();
        let spot = Spot {
            id: Spot::make_id(SpotKind::PskReporter, &call, freq_hz),
            kind: SpotKind::PskReporter,
            freq_hz,
            call,
            spotter: attr("receiverCallsign").to_ascii_uppercase(),
            mode: attr("mode").to_ascii_uppercase(),
            comment: String::new(),
            reference: None,
            grid,
            loc: None,
            when_utc: now,
            snr_db,
            color: None,
        };
        // Keep the strongest report per (call, freq bucket).
        if let Some(existing) = out.iter_mut().find(|s| s.id == spot.id) {
            if spot.snr_db.unwrap_or(-99) > existing.snr_db.unwrap_or(-99) {
                *existing = spot;
            }
        } else {
            out.push(spot);
        }
    }
    Ok(out)
}

/// Amateur-band edges (Hz) for the band containing `hz`, else a ±100 kHz window.
fn band_edges(hz: f64) -> (f64, f64) {
    const BANDS: &[(f64, f64)] = &[
        (1_800_000.0, 2_000_000.0),
        (3_500_000.0, 4_000_000.0),
        (5_250_000.0, 5_450_000.0),
        (7_000_000.0, 7_300_000.0),
        (10_100_000.0, 10_150_000.0),
        (14_000_000.0, 14_350_000.0),
        (18_068_000.0, 18_168_000.0),
        (21_000_000.0, 21_450_000.0),
        (24_890_000.0, 24_990_000.0),
        (28_000_000.0, 29_700_000.0),
        (50_000_000.0, 54_000_000.0),
        (144_000_000.0, 148_000_000.0),
    ];
    for &(lo, hi) in BANDS {
        if (lo..=hi).contains(&hz) {
            return (lo, hi);
        }
    }
    ((hz - 100_000.0).max(0.0), hz + 100_000.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_reception_reports() {
        let body = r#"<?xml version="1.0"?>
        <receptionReports>
          <receptionReport receptionTimestamp="1700000000" senderCallsign="K1ABC"
            senderLocator="FN42" frequency="14074123" mode="FT8" sNR="-8"
            receiverCallsign="W9XYZ" receiverLocator="EN52"/>
          <receptionReport senderCallsign="K1ABC" senderLocator="FN42"
            frequency="14074120" mode="FT8" sNR="-3" receiverCallsign="N0AAA"/>
        </receptionReports>"#;
        let spots = parse(body, 42).unwrap();
        assert_eq!(spots.len(), 1); // deduped by (call, freq bucket)
        assert_eq!(spots[0].call, "K1ABC");
        assert_eq!(spots[0].snr_db, Some(-3)); // strongest kept
        assert_eq!(spots[0].grid.as_deref(), Some("FN42"));
    }

    #[test]
    fn band_edges_for_20m() {
        assert_eq!(band_edges(14_074_000.0), (14_000_000.0, 14_350_000.0));
    }
}
