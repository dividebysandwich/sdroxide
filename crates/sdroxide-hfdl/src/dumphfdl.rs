//! dumphfdl's `decoded:json` format, from xng's decoded events.
//!
//! dumphfdl is the reference HFDL decoder, and its JSON is what the tools
//! around HFDL read: Airframes' HFDL ingest (TCP `feed.airframes.io:5556`),
//! acarshub, and anything built on them. So that is the format sdroxide sends.
//! The layout here follows dumphfdl's own formatters (`fmtr-json.c`,
//! `spdu.c`, `lpdu.c`, `hfnpdu.c`, `util.c`) and libacars' `acars.c` for the
//! ACARS object — read for the field names and nesting only (facts, not code,
//! as xng does). One JSON object per line, one line per LPDU or squitter:
//!
//! ```text
//! {"hfdl":{"app":{…},"station":…,"t":{"sec":…,"usec":…},"freq":…,"sig_level":…,
//!          "noise_level":…,"freq_skew":…,
//!          "spdu":{…}                                   (a squitter)
//!          "lpdu":{…, "hfnpdu":{…, "acars":{…}}}}}      (anything else)
//! ```
//!
//! Where it differs, and why:
//!
//! * `app.name` is `sdroxide`, not `dumphfdl` — it is not dumphfdl.
//! * `bit_rate` is present only when it is known. xng hands the burst's bit
//!   rate to its parser but reports it only on ACARS events; elsewhere it is
//!   left out rather than made up. `slot` (single/double) is never known here
//!   and is left out for the same reason.
//! * `sig_level` is the channel's burst level in dBFS and `noise_level` that
//!   less the burst's SNR — the same quantities, measured by a different demod.
//! * An LPDU carried in an *acknowledged* data LPDU (0x1D) is reported as
//!   unnumbered data (0x0D): xng's events do not say which carried them.
//! * An uplink to several aircraft names its destination only when it was to
//!   one: xng does not say which of them each LPDU was for.
//! * Frames dumphfdl would show as an unknown protocol (an HFNPDU that is not
//!   one, an ACARS block that does not parse) are not sent at all.

use sdroxide_types::HfdlStation;
use serde_json::{Map, Value, json};
use xng_mode_hfdl::pdu::{HfdlEvent, gs_name};

/// What the burst an event came from looked like, and who is sending it.
#[derive(Debug, Clone)]
pub struct FrameMeta {
    /// Decode time, Unix seconds and microseconds.
    pub sec: i64,
    pub usec: i64,
    /// The channel, Hz.
    pub freq_hz: u64,
    /// Burst level, dBFS.
    pub sig_level: f32,
    /// The station id the operator gave, when there is one.
    pub station: Option<String>,
}

/// How the serializer finds a ground station: its name and its channels, for
/// the frequency lists (`{"id": n, "freq": kHz}`).
pub type StationLookup<'a> = &'a dyn Fn(u8) -> Option<&'a HfdlStation>;

/// One event as a dumphfdl `decoded:json` line (without the newline), or
/// `None` for an event that has no place in that format.
pub fn format(e: &HfdlEvent, meta: &FrameMeta, gs: StationLookup<'_>) -> Option<Value> {
    let body = match e.kind.as_str() {
        "squitter" => ("spdu", spdu(e, gs)?),
        "logon-request" | "logon-resume" | "logon-confirm" | "logoff-request" | "logon-denied"
        | "lpdu" => ("lpdu", lpdu_only(e, gs)?),
        "performance-data" | "frequency-data" | "systable-partial" | "systable-request"
        | "delayed-echo" | "acars" | "hfnpdu" => ("lpdu", lpdu_with_hfnpdu(e, gs)?),
        // A reassembled system table is a by-product of its last partial (sent
        // as that); an unparseable HFNPDU is an unknown protocol to dumphfdl.
        _ => return None,
    };
    let mut h = Map::new();
    h.insert("app".into(), json!({ "name": "sdroxide", "ver": env!("CARGO_PKG_VERSION") }));
    if let Some(st) = meta.station.as_deref().filter(|s| !s.is_empty()) {
        h.insert("station".into(), json!(st));
    }
    h.insert("t".into(), json!({ "sec": meta.sec, "usec": meta.usec }));
    h.insert("freq".into(), json!(meta.freq_hz));
    if let Some(bps) = e.details.get("bps").and_then(Value::as_u64) {
        h.insert("bit_rate".into(), json!(bps));
    }
    h.insert("sig_level".into(), json!(round1(meta.sig_level)));
    if let Some(snr) = e.snr_db {
        h.insert("noise_level".into(), json!(round1(meta.sig_level - snr)));
    }
    if let Some(skew) = e.freq_skew_hz {
        h.insert("freq_skew".into(), json!(round1(skew)));
    }
    h.insert(body.0.into(), body.1);
    Some(json!({ "hfdl": Value::Object(h) }))
}

fn round1(x: f32) -> f64 {
    (f64::from(x) * 10.0).round() / 10.0
}

/// A ground station, as dumphfdl names one: `{"type","id","name"}`.
fn gs_obj(id: u8, gs: StationLookup<'_>) -> Value {
    let name = gs(id).map(|s| s.name.clone()).or_else(|| gs_name(id).map(str::to_owned));
    let mut o = json!({ "type": "Ground station", "id": id });
    if let Some(n) = name {
        o["name"] = json!(n);
    }
    o
}

/// An aircraft, as dumphfdl names one: `{"type","id","ac_info":{"icao"}}`.
fn ac_obj(id: u64, icao: Option<&str>) -> Value {
    let mut o = json!({ "type": "Aircraft", "id": id });
    if let Some(i) = icao {
        o["ac_info"] = json!({ "icao": i });
    }
    o
}

/// A 20-bit frequency bitmap as dumphfdl lists it: bit *n* set gives
/// `{"id": n, "freq": kHz}`, the frequency where the station's table has it.
fn freq_list(gs_id: u8, bits: u64, gs: StationLookup<'_>) -> Value {
    let table = gs(gs_id);
    let mut out = Vec::new();
    for i in 0..20 {
        if bits >> i & 1 == 1 {
            let mut o = json!({ "id": i });
            if let Some(f) = table.and_then(|s| s.freqs.get(i as usize)) {
                o["freq"] = json!(f64::from(f.freq_hz) / 1000.0);
            }
            out.push(o);
        }
    }
    Value::Array(out)
}

/// A squitter, re-read from its 66 octets the way dumphfdl reads it: the three
/// ground-station status entries (this station and two others), each with the
/// channels it has in use.
fn spdu(e: &HfdlEvent, gs: StationLookup<'_>) -> Option<Value> {
    let b = &e.raw;
    if b.len() < 64 {
        return None;
    }
    const CHANGE_NOTE: [&str; 4] =
        ["None", "Channel down", "Upcoming frequency change", "Ground station down"];
    let status = |id: u8, utc: bool, freqs: u32| json!({ "gs": gs_obj(id, gs), "utc_sync": utc, "freqs": freq_list(id, u64::from(freqs), gs) });
    let src = b[1] & 0x7F;
    let gs0 = status(
        src,
        b[1] & 0x80 != 0,
        u32::from(b[54] >> 4) | u32::from(b[55]) << 4 | u32::from(b[56]) << 12,
    );
    let gs1 = status(
        b[57] & 0x7F,
        b[57] & 0x80 != 0,
        u32::from(b[58]) | u32::from(b[59]) << 8 | u32::from(b[60] & 0x0F) << 16,
    );
    let gs2 = status(
        b[60] >> 4 | (b[61] & 0x07) << 4,
        b[61] & 0x08 != 0,
        u32::from(b[61] >> 4) | u32::from(b[62]) << 4 | u32::from(b[63]) << 12,
    );
    Some(json!({
        "err": false,
        "src": gs_obj(src, gs),
        "spdu_version": (b[0] >> 2) & 3,
        "rls": b[0] & 0x02 != 0,
        "iso": b[0] & 0x20 != 0,
        "change_note": CHANGE_NOTE[usize::from(b[0] >> 6)],
        "frame_index": u16::from(b[2]) | u16::from(b[3] & 0x0F) << 8,
        "frame_offset": b[3] >> 4,
        "min_priority": b[52] & 0x0F,
        "systable_version": u16::from(b[53]) | u16::from(b[54] & 0x0F) << 8,
        "gs_status": [gs0, gs1, gs2],
    }))
}

/// The LPDU envelope's source and destination, from xng's `who`.
fn src_dst(e: &HfdlEvent, gs: StationLookup<'_>) -> Option<(Value, Option<Value>)> {
    let who = e.details.get("who")?;
    let gs_id = who.get("gs_id")?.as_u64()? as u8;
    match who.get("dir")?.as_str()? {
        "downlink" => {
            let ac = who.get("aircraft_id")?.as_u64()?;
            let icao = who.get("icao").and_then(Value::as_str);
            Some((ac_obj(ac, icao), Some(gs_obj(gs_id, gs))))
        }
        _ => {
            // An uplink names its aircraft only when it was to just one.
            let dst = who
                .get("aircraft")
                .and_then(Value::as_array)
                .filter(|a| a.len() == 1)
                .and_then(|a| a[0].get("aircraft_id")?.as_u64())
                .map(|id| ac_obj(id, None));
            Some((gs_obj(gs_id, gs), dst))
        }
    }
}

fn lpdu_head(e: &HfdlEvent, type_id: u8, gs: StationLookup<'_>) -> Option<Map<String, Value>> {
    let (src, dst) = src_dst(e, gs)?;
    let mut l = Map::new();
    l.insert("err".into(), json!(false));
    l.insert("src".into(), src);
    if let Some(d) = dst {
        l.insert("dst".into(), d);
    }
    l.insert("type".into(), json!({ "id": type_id, "name": lpdu_type_name(type_id) }));
    Some(l)
}

fn lpdu_type_name(t: u8) -> &'static str {
    match t {
        0x0D => "Unnumbered data",
        0x1D => "Unnumbered ack'ed data",
        0x2F => "Logon denied",
        0x3F => "Logoff request",
        0x5F => "Logon resume confirm",
        0x4F => "Logon resume",
        0x8F => "Logon request (normal)",
        0x9F => "Logon confirm",
        0xBF => "Logon request (DLS)",
        _ => "unknown",
    }
}

/// An LPDU that carries no HFNPDU: the logon exchange, and the rest by type.
fn lpdu_only(e: &HfdlEvent, gs: StationLookup<'_>) -> Option<Value> {
    let type_id = *e.raw.first()?;
    let mut l = lpdu_head(e, type_id, gs)?;
    let icao = e.details.get("icao").and_then(Value::as_str);
    match e.kind.as_str() {
        "logon-request" | "logon-resume" => {
            l.insert("ac_info".into(), json!({ "icao": icao? }));
        }
        "logon-confirm" => {
            l.insert("ac_info".into(), json!({ "icao": icao? }));
            l.insert("assigned_ac_id".into(), json!(e.details.get("assigned_id")?.as_u64()?));
        }
        "logoff-request" | "logon-denied" => {
            l.insert("ac_info".into(), json!({ "icao": icao? }));
            l.insert(
                "reason".into(),
                json!({
                    "code": e.details.get("reason")?.as_u64()?,
                    "descr": e.details.get("reason_text").and_then(Value::as_str).unwrap_or("Reserved"),
                }),
            );
        }
        _ => {}
    }
    Some(Value::Object(l))
}

/// An unnumbered-data LPDU and the HFNPDU in it.
fn lpdu_with_hfnpdu(e: &HfdlEvent, gs: StationLookup<'_>) -> Option<Value> {
    let mut l = lpdu_head(e, 0x0D, gs)?;
    l.insert("hfnpdu".into(), hfnpdu(e, gs)?);
    Some(Value::Object(l))
}

fn hfnpdu_type(id: u8) -> Value {
    let name = match id {
        0xD0 => "System table (partial)",
        0xD1 => "Performance data",
        0xD2 => "System table request",
        0xD5 => "Frequency data",
        0xDE => "Delayed echo",
        0xFF => "Enveloped data",
        _ => "unknown",
    };
    json!({ "id": id, "name": name })
}

fn hfnpdu(e: &HfdlEvent, gs: StationLookup<'_>) -> Option<Value> {
    let d = &e.details;
    let utc = || -> Option<Value> {
        let u = d.get("utc")?;
        Some(json!({ "hour": u.get("hour")?, "min": u.get("min")?, "sec": u.get("sec")? }))
    };
    let pos = || -> Option<Value> { Some(json!({ "lat": d.get("lat")?, "lon": d.get("lon")? })) };
    let flight = || d.get("flight").and_then(Value::as_str).unwrap_or("").to_owned();
    let out = match e.kind.as_str() {
        "performance-data" => {
            let gs_id = d.get("gs_id")?.as_u64()? as u8;
            let freq_id = d.get("freq_id")?.as_u64()?;
            let mut frequency = json!({ "id": 1u64 << freq_id.min(31) });
            if let Some(f) = gs(gs_id).and_then(|s| s.freqs.get(freq_id as usize)) {
                frequency["freq"] = json!(f64::from(f.freq_hz) / 1000.0);
            }
            let leg = |k: &str, a: &str, b: &str| -> Option<Value> {
                let o = d.get(k)?;
                Some(json!({ a: o.get("cur_leg")?, b: o.get("prev_leg")? }))
            };
            json!({
                "err": false,
                "type": hfnpdu_type(0xD1),
                "version": d.get("version")?,
                "flight_id": flight(),
                "pos": pos()?,
                "time": utc()?,
                "flight_leg_num": d.get("flight_leg")?,
                "gs": gs_obj(gs_id, gs),
                "frequency": frequency,
                "freq_search_cnt": leg("freq_search_cnt", "cur_leg", "prev_leg")?,
                "hfdl_disabled_duration": leg("hfdl_disabled_duration", "this_leg", "prev_leg")?,
                "pdu_stats": {
                    "mpdus_rx_ok_cnt": d.get("mpdus_rx")?,
                    "mpdus_rx_err_cnt": d.get("mpdus_rx_errs")?,
                    "mpdus_tx_cnt": d.get("mpdus_tx")?,
                    "mpdus_delivered_cnt": d.get("mpdus_delivered")?,
                    "spdus_rx_ok_cnt": d.get("spdus_rx")?,
                    "spdus_missed_cnt": d.get("spdus_rx_errs")?,
                },
                "last_freq_change_cause": {
                    "code": d.get("freq_change_code")?,
                    "descr": d.get("freq_change_cause").and_then(Value::as_str).unwrap_or("unknown"),
                },
            })
        }
        "frequency-data" => {
            let freq_data: Vec<Value> = d
                .get("freq_data")?
                .as_array()?
                .iter()
                .filter_map(|f| {
                    let id = f.get("gs_id")?.as_u64()? as u8;
                    Some(json!({
                        "gs": gs_obj(id, gs),
                        "listening_on_freqs": freq_list(id, f.get("tuned_freqs")?.as_u64()?, gs),
                        "heard_on_freqs": freq_list(id, f.get("prop_freqs")?.as_u64()?, gs),
                    }))
                })
                .collect();
            json!({
                "err": false,
                "type": hfnpdu_type(0xD5),
                "flight_id": flight(),
                "pos": pos()?,
                "utc_time": utc()?,
                "freq_data": freq_data,
            })
        }
        "systable-partial" => json!({
            "err": false,
            "type": hfnpdu_type(0xD0),
            "version": d.get("version")?,
            "systable_partial": {
                "part_num": d.get("seq")?.as_u64()? + 1,
                "parts_cnt": d.get("total")?,
            },
        }),
        "systable-request" => json!({
            "err": false,
            "type": hfnpdu_type(0xD2),
            "request_data": d.get("request_data")?,
        }),
        "delayed-echo" => json!({ "err": false, "type": hfnpdu_type(0xDE) }),
        "hfnpdu" => {
            json!({ "err": false, "type": hfnpdu_type(d.get("type")?.as_u64()? as u8) })
        }
        "acars" => {
            let mut h = json!({ "err": false, "type": hfnpdu_type(0xFF) });
            h["acars"] = acars(e)?;
            h
        }
        _ => return None,
    };
    Some(out)
}

/// The ACARS block as libacars puts it in JSON.
fn acars(e: &HfdlEvent) -> Option<Value> {
    let b = e.acars.as_ref()?;
    let c = &b.core;
    let ch = |c: Option<char>| c.map(String::from).unwrap_or_default();
    // The registration as it was on air: seven characters, dot-padded on the
    // left — xng strips the dots, libacars keeps them.
    let reg = format!("{:.>7}", c.tail.as_deref().unwrap_or(""));
    let mut o = json!({
        "err": false,
        "crc_ok": b.crc_ok,
        "more": c.more_to_come,
        "reg": reg,
        "mode": c.mode.to_string(),
        "label": c.label,
        "blk_id": ch(c.block_id),
        // A NAK is shown as '!', as libacars does.
        "ack": c.ack.map_or_else(|| "!".to_string(), String::from),
    });
    if b.downlink {
        o["flight"] = json!(c.flight.as_deref().unwrap_or(""));
        match &b.min {
            Some(m) => {
                o["msg_num"] = json!(m.msg_num);
                o["msg_num_seq"] = json!(m.msg_num_seq.to_string());
            }
            None => {
                let raw = c.msg_num.as_deref().unwrap_or("");
                o["msg_num"] = json!(raw.chars().take(3).collect::<String>());
                o["msg_num_seq"] = json!(raw.chars().nth(3).map(String::from).unwrap_or_default());
            }
        }
    }
    if let Some(s) = c.sublabel.as_deref().filter(|s| !s.is_empty()) {
        o["sublabel"] = json!(s);
    }
    if let Some(m) = c.mfi.as_deref().filter(|s| !s.is_empty()) {
        o["mfi"] = json!(m);
    }
    o["msg_text"] = json!(c.text);
    Some(o)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdroxide_types::hfdl_builtin_stations;
    use xng_mode_hfdl::pdu::{
        PduParser, build_lpdu_acars, build_lpdu_hfnpdu, build_lpdu_logon_confirm,
        build_mpdu_downlink, build_mpdu_uplink, build_spdu,
    };

    fn meta() -> FrameMeta {
        FrameMeta {
            sec: 1_700_000_000,
            usec: 250_000,
            freq_hz: 8_942_000,
            sig_level: -40.0,
            station: Some("XX-TEST-HFDL".into()),
        }
    }

    /// Parse `pdu` with xng's own parser, as the worker does, and serialize the
    /// event of `kind`.
    fn one(pdu: &[u8], kind: &str) -> Value {
        let table = hfdl_builtin_stations();
        let lookup = |id: u8| table.iter().find(|s| s.gs_id == id);
        let mut p = PduParser::new();
        let mut e = p.parse(pdu, 600).into_iter().find(|e| e.kind == kind).unwrap_or_else(|| {
            panic!(
                "no {kind} in {:?}",
                p.parse(pdu, 600).iter().map(|e| e.kind.clone()).collect::<Vec<_>>()
            )
        });
        e.snr_db = Some(14.0);
        e.freq_skew_hz = Some(-1.25);
        format(&e, &meta(), &lookup).expect("serializable")
    }

    #[test]
    fn the_envelope_is_dumphfdls() {
        let v = one(&build_spdu(7, 1234, 52), "squitter");
        let h = &v["hfdl"];
        assert_eq!(h["app"]["name"], "sdroxide");
        assert_eq!(h["station"], "XX-TEST-HFDL");
        assert_eq!(h["t"]["sec"], 1_700_000_000);
        assert_eq!(h["t"]["usec"], 250_000);
        assert_eq!(h["freq"], 8_942_000);
        assert_eq!(h["sig_level"], -40.0);
        assert_eq!(h["noise_level"], -54.0);
        assert_eq!(h["freq_skew"], -1.3);
        assert!(h.get("bit_rate").is_none(), "unknown for a squitter: left out, not made up");
        assert!(h.get("slot").is_none());
    }

    #[test]
    fn a_squitter_is_an_spdu_with_its_channels_named() {
        let v = one(&build_spdu(7, 1234, 52), "squitter");
        let s = &v["hfdl"]["spdu"];
        assert_eq!(s["err"], false);
        assert_eq!(
            s["src"],
            json!({ "type": "Ground station", "id": 7, "name": "Shannon, Ireland" })
        );
        assert_eq!(s["frame_index"], 1234);
        assert_eq!(s["systable_version"], 52);
        assert_eq!(s["change_note"], "None");
        let gs0 = &s["gs_status"][0];
        assert_eq!(gs0["gs"]["id"], 7);
        assert_eq!(gs0["utc_sync"], true);
        // The builder sets bit 0: Shannon's first channel, 11 384 kHz.
        assert_eq!(gs0["freqs"], json!([{ "id": 0, "freq": 11384.0 }]));
        assert_eq!(s["gs_status"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn an_acars_downlink_is_lpdu_hfnpdu_acars() {
        let block = xng_acars::block::build(
            '2',
            "N471XG",
            None,
            "H1",
            '4',
            Some("M11A"),
            Some("UA0042"),
            "HELLO",
            false,
        );
        let v = one(&build_mpdu_downlink(7, 0xC7, &[build_lpdu_acars(&block)]), "acars");
        let h = &v["hfdl"];
        assert_eq!(h["bit_rate"], 600, "known on ACARS events");
        let l = &h["lpdu"];
        assert_eq!(l["src"], json!({ "type": "Aircraft", "id": 0xC7 }));
        assert_eq!(l["dst"]["id"], 7);
        assert_eq!(l["type"], json!({ "id": 13, "name": "Unnumbered data" }));
        let n = &l["hfnpdu"];
        assert_eq!(n["type"], json!({ "id": 255, "name": "Enveloped data" }));
        let a = &n["acars"];
        assert_eq!(a["reg"], ".N471XG", "dot-padded to seven, as on air");
        assert_eq!(a["label"], "H1");
        assert_eq!(a["mode"], "2");
        assert_eq!(a["blk_id"], "4");
        assert_eq!(a["ack"], "!");
        assert_eq!(a["flight"], "UA0042");
        assert_eq!(a["msg_num"], "M11");
        assert_eq!(a["msg_num_seq"], "A");
        assert_eq!(a["msg_text"], "HELLO");
        assert_eq!(a["crc_ok"], true);
        assert_eq!(a["more"], false);
    }

    #[test]
    fn a_logon_confirm_carries_the_icao_and_the_assigned_id() {
        let v = one(
            &build_mpdu_uplink(7, 0xC7, &[build_lpdu_logon_confirm(0xA1B2C3, 42)]),
            "logon-confirm",
        );
        let l = &v["hfdl"]["lpdu"];
        assert_eq!(l["src"]["type"], "Ground station");
        assert_eq!(l["dst"], json!({ "type": "Aircraft", "id": 0xC7 }));
        assert_eq!(l["type"], json!({ "id": 0x9F, "name": "Logon confirm" }));
        assert_eq!(l["ac_info"]["icao"], "A1B2C3");
        assert_eq!(l["assigned_ac_id"], 42);
    }

    #[test]
    fn a_performance_report_names_its_station_and_frequency() {
        // A performance-data HFNPDU (0xD1), 47 octets: flight, position, UTC,
        // version, leg, the GS and its frequency index, counters.
        let mut h = vec![0xFF, 0xD1];
        h.extend(b"UAL12 ");
        h.extend([0x00, 0x40, 0x07, 0xE0, 0xF6]); // a non-zero lat/lon
        h.extend([0x10, 0x27]); // UTC counter
        h.extend([1, 3, 7, 2]); // version, flight leg, GS 7, freq id 2
        h.resize(47, 0);
        let v = one(&build_mpdu_downlink(7, 0xC7, &[build_lpdu_hfnpdu(&h)]), "performance-data");
        let n = &v["hfdl"]["lpdu"]["hfnpdu"];
        assert_eq!(n["type"], json!({ "id": 0xD1, "name": "Performance data" }));
        assert_eq!(n["flight_id"], "UAL12");
        assert_eq!(n["gs"]["name"], "Shannon, Ireland");
        // Shannon's third channel, 8 942 kHz; the id is a bit, as dumphfdl has it.
        assert_eq!(n["frequency"], json!({ "id": 4, "freq": 8942.0 }));
        assert_eq!(n["flight_leg_num"], 3);
        assert!(n["pos"]["lat"].is_number() && n["time"]["hour"].is_number());
        assert!(n["pdu_stats"]["mpdus_rx_ok_cnt"]["300bps"].is_number());
        assert!(n["last_freq_change_cause"]["descr"].is_string());
    }

    #[test]
    fn a_reassembled_table_is_not_sent_on_its_own() {
        let e = HfdlEvent {
            kind: "systable-complete".into(),
            details: json!({}),
            acars: None,
            fec_corrected: None,
            freq_skew_hz: None,
            snr_db: None,
            raw: Vec::new(),
        };
        assert!(format(&e, &meta(), &|_| None).is_none());
    }

    #[test]
    fn without_a_station_id_there_is_no_station_field() {
        let table = hfdl_builtin_stations();
        let lookup = |id: u8| table.iter().find(|s| s.gs_id == id);
        let e = PduParser::new().parse(&build_spdu(4, 1, 52), 300).remove(0);
        let v = format(&e, &FrameMeta { station: None, ..meta() }, &lookup).unwrap();
        assert!(v["hfdl"].get("station").is_none());
    }
}
