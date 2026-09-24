//! Saving decoded text and logs (issue #533).
//!
//! Every text panel has a **SAVE** chip beside its **CLEAR RX**; this is what
//! the chip writes. The FT8/FT4/FT2 DECODES header's CSV/ADIF buttons were the
//! program's only export, so a listener who had just copied a NAVTEX bulletin,
//! a CW run or an ACARS block had nowhere to put it. The formatters here turn
//! each mode's log into text — free-running text as itself, structured logs as
//! one line per item — and the panel supplies the file name.
//!
//! Nothing here touches the engine or the wire: the data is already in the
//! status the panels draw, and the file goes out through the same
//! [`crate::download::save`] the ADIF export uses.

use sdroxide_types::{
    AcarsStatus, DigiStatus, HfdlDecode, NavtexStatus, Pi4Spot, SkimmerSpot, Vdl2Message, WsprSpot,
};

/// A UTC timestamp as a log line wants it.
fn stamp(unix: i64) -> String {
    let (y, mo, d, h, mi, s) = sdroxide_types::utc_ymd_hms(unix);
    format!("{y:04}-{mo:02}-{d:02} {h:02}:{mi:02}:{s:02}Z")
}

/// A CSV field, quoted when it holds a comma, a quote or a newline.
fn csv(s: &str) -> String {
    if s.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// The mode's name lowercased and filesystem-safe, for the suggested filename.
fn slug(status: &DigiStatus) -> String {
    status.mode.label().to_ascii_lowercase().replace(['/', ' '], "-")
}

/// Whether [`digi_log`] has anything to write.
///
/// A cheap test so a panel can grey its SAVE chip every frame without building
/// the text; it must stay in step with `digi_log`, which it guards.
pub fn digi_has_log(status: &DigiStatus) -> bool {
    !status.text_rx.trim().is_empty()
        || status.acars.as_ref().is_some_and(|a| !a.messages.is_empty())
        || status.navtex.as_ref().is_some_and(|n| !n.messages.is_empty())
        || !status.fsq_messages.is_empty()
        || status.packet.as_ref().is_some_and(|p| !p.heard.is_empty())
        || status.js8.as_ref().is_some_and(|j| !j.messages.is_empty())
}

/// The current mode's decoded log as `(suggested file name, text)`, or `None`
/// when the mode keeps nothing to save yet.
///
/// One entry point for every mode whose data rides [`DigiStatus`]: the rolling
/// text the keyboard modes and CW accumulate, and the structured logs the
/// message modes keep. Modes with their own status (HFDL, VDL2, WSPR, PI4, the
/// skimmer) have their own formatter below.
pub fn digi_log(status: &DigiStatus) -> Option<(String, String)> {
    if !digi_has_log(status) {
        return None;
    }
    // Free-running text first: CW and every keyboard mode share it, and it is
    // what a listener most often wants to keep.
    if !status.text_rx.trim().is_empty() {
        return Some((format!("sdroxide-{}-rx.txt", slug(status)), status.text_rx.clone()));
    }
    if let Some(a) = &status.acars {
        return Some((format!("sdroxide-{}-log.csv", slug(status)), acars_csv(a)));
    }
    if let Some(n) = &status.navtex {
        return Some((format!("sdroxide-{}-log.txt", slug(status)), navtex_text(n)));
    }
    if !status.fsq_messages.is_empty() {
        let mut out = String::from("utc\tdirection\tfrom\tto\ttext\n");
        for m in &status.fsq_messages {
            out.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\n",
                stamp(0),
                if m.to_me { "to-me" } else { "all" },
                m.from,
                m.to,
                m.text
            ));
        }
        return Some((format!("sdroxide-{}-log.txt", slug(status)), out));
    }
    if let Some(p) = &status.packet
        && !p.heard.is_empty()
    {
        let mut out = String::from("utc\tfrom\tto\tvia\tkind\tsent\ttext\n");
        for h in &p.heard {
            out.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                stamp(h.at),
                csv(&h.from),
                csv(&h.to),
                csv(&h.via.join(",")),
                csv(&h.kind),
                if h.sent { "sent" } else { "heard" },
                csv(&h.text)
            ));
        }
        return Some((format!("sdroxide-{}-log.csv", slug(status)), out));
    }
    if let Some(j) = &status.js8
        && !j.messages.is_empty()
    {
        let mut out = String::from("utc\tfrom\tto\tsnr_db\taudio_hz\tcomplete\ttext\n");
        for m in &j.messages {
            out.push_str(&format!(
                "{}\t{}\t{}\t{}\t{:.0}\t{}\t{}\n",
                stamp(m.first_slot_utc),
                csv(&m.from),
                csv(&m.to),
                m.snr_db,
                m.audio_hz,
                if m.complete { "complete" } else { "partial" },
                csv(&m.text)
            ));
        }
        return Some((format!("sdroxide-{}-log.csv", slug(status)), out));
    }
    None
}

fn acars_csv(a: &AcarsStatus) -> String {
    let mut out = String::from("utc,mode,address,label,block_id,crc_ok,text\n");
    for m in &a.messages {
        out.push_str(&format!(
            "{},{},{},{},{},{},{}\n",
            stamp(m.at),
            csv(&m.mode),
            csv(&m.address),
            csv(&m.label),
            csv(&m.block_id),
            if m.crc_ok { "ok" } else { "bad" },
            csv(&m.text)
        ));
    }
    out
}

fn navtex_text(n: &NavtexStatus) -> String {
    let mut out = String::from("utc\tstation\tkind\tserial\tlost\ttext\n");
    for m in &n.messages {
        out.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\n",
            stamp(m.at),
            m.station,
            m.kind,
            m.serial,
            m.lost,
            m.text.replace('\n', " ")
        ));
    }
    out
}

/// The WSPR spot list as CSV.
pub fn wspr_spots_csv(spots: &[WsprSpot]) -> String {
    let mut out = String::from("utc,call,grid,power_dbm,freq_mhz,snr_db,dt\n");
    for s in spots {
        out.push_str(&format!(
            "{},{},{},{},{:.6},{},{:.1}\n",
            stamp(s.slot_utc),
            csv(&s.call),
            csv(s.grid.as_deref().unwrap_or("")),
            s.power_dbm,
            s.freq_hz / 1e6,
            s.snr_db,
            s.dt
        ));
    }
    out
}

/// The PI4 spot list as CSV.
pub fn pi4_spots_csv(spots: &[Pi4Spot]) -> String {
    let mut out = String::from("utc,text,variant,dt_sec,snr_db\n");
    for s in spots {
        out.push_str(&format!(
            "{},{},{},{:.2},{:.1}\n",
            stamp(s.slot_utc),
            csv(&s.text),
            csv(&s.variant),
            s.dt_sec,
            s.snr_db
        ));
    }
    out
}

/// The HFDL decode log as text.
pub fn hfdl_log_text(log: &[HfdlDecode]) -> String {
    let mut out = String::from("utc\tkind\tgs\tfreq_khz\tsnr_db\tdetails\n");
    for d in log {
        out.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\n",
            stamp(d.unix),
            csv(&d.kind),
            csv(d.gs.as_deref().unwrap_or("")),
            d.freq_khz,
            d.snr_db.map_or(String::new(), |v| format!("{v:.1}")),
            d.details.replace(['\n', '\t'], " ")
        ));
    }
    out
}

/// The VDL2 message log as text.
pub fn vdl2_log_text(messages: &[Vdl2Message]) -> String {
    let mut out = String::from("utc\tfreq_mhz\tsnr_db\tsummary\n");
    for m in messages {
        out.push_str(&format!(
            "{}\t{:.3}\t{:.1}\t{}\n",
            stamp(m.at),
            m.freq_hz / 1e6,
            m.snr_db,
            m.summary().replace(['\n', '\t'], " ")
        ));
    }
    out
}

/// The skimmer's spot list as text, newest first.
pub fn skimmer_text(spots: &[SkimmerSpot]) -> String {
    let mut out = String::from("kind\tfreq_mhz\tcallsign\tsnr_db\twpm\ttext\n");
    for s in spots {
        out.push_str(&format!(
            "{}\t{:.4}\t{}\t{}\t{}\t{}\n",
            s.kind.label(),
            s.freq_hz / 1e6,
            csv(s.callsign.as_deref().unwrap_or("")),
            s.snr_db,
            s.wpm,
            s.text.replace(['\n', '\t'], " ")
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_csv_field_is_quoted_only_when_it_has_to_be() {
        assert_eq!(csv("plain"), "plain");
        assert_eq!(csv("a,b"), "\"a,b\"");
        assert_eq!(csv("say \"hi\""), "\"say \"\"hi\"\"\"");
        assert_eq!(csv("two\nlines"), "\"two\nlines\"");
    }

    #[test]
    fn wspr_spots_export_one_row_a_spot() {
        let spots = vec![WsprSpot {
            slot_utc: 1_700_000_000,
            call: "W1ABC".into(),
            grid: Some("FN42".into()),
            power_dbm: 37,
            freq_hz: 14_097_100.0,
            snr_db: -12,
            dt: 0.3,
            drift_hz: 0.0,
            reporter: None,
            reporter_grid: None,
        }];
        let out = wspr_spots_csv(&spots);
        let mut lines = out.lines();
        assert_eq!(lines.next().unwrap(), "utc,call,grid,power_dbm,freq_mhz,snr_db,dt");
        let row = lines.next().unwrap();
        assert!(row.contains("W1ABC,FN42,37,14.097100,-12,0.3"), "{row}");
    }

    #[test]
    fn a_cw_run_saves_as_its_own_text() {
        let mut st = DigiStatus::idle(sdroxide_types::DigiConfig::default());
        st.mode = sdroxide_types::Mode::Cw;
        st.text_rx = "CQ DE W1ABC".into();
        let (name, text) = digi_log(&st).expect("something to save");
        assert_eq!(name, "sdroxide-cw-rx.txt");
        assert_eq!(text, "CQ DE W1ABC");
    }

    #[test]
    fn an_empty_log_saves_nothing() {
        let st = DigiStatus::idle(sdroxide_types::DigiConfig::default());
        assert!(digi_log(&st).is_none());
    }
}
