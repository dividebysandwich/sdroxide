//! A round trip through a real Wavelog: check the connection, upload a QSO,
//! upload it again, have a mismatched one refused, download it back, merge it
//! into a log, and report a radio status. Every QSO it writes carries the
//! comment "sdroxide test", so they are easy to find and delete afterwards.
//!
//! Skipped unless pointed at a server — one you do not mind writing test QSOs
//! to, such as a throwaway container:
//!
//!   WAVELOG_URL=http://127.0.0.1:8086 WAVELOG_KEY=wl… WAVELOG_STATION=1 \
//!   WAVELOG_CALL=EA5ABC WAVELOG_GRID=IM99TL \
//!   cargo test -p sdroxide-net --test wavelog_live -- --ignored --nocapture

use sdroxide_net::{
    WavelogRadioNow, wavelog_download, wavelog_probe, wavelog_send_radio, wavelog_upload,
};
use sdroxide_types::{QsoRecord, WavelogConfig, merge_from_wavelog, qso_log_to_adif};

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.trim().is_empty())
}

#[test]
#[ignore = "writes test QSOs to a real Wavelog (needs WAVELOG_URL, _KEY, _STATION, _CALL, _GRID)"]
fn wavelog_round_trip() {
    let (Some(url), Some(key), Some(station), Some(call), Some(grid)) = (
        env("WAVELOG_URL"),
        env("WAVELOG_KEY"),
        env("WAVELOG_STATION"),
        env("WAVELOG_CALL"),
        env("WAVELOG_GRID"),
    ) else {
        eprintln!("WAVELOG_* not set — skipping");
        return;
    };
    let station: u32 = station.parse().expect("WAVELOG_STATION is a number");
    let cfg = WavelogConfig {
        url,
        api_key: key,
        cert_sha256: env("WAVELOG_PIN").unwrap_or_default(),
        station_id: station,
        ..Default::default()
    };

    // The connection, the key, and the location.
    let probe = wavelog_probe(&cfg);
    eprintln!("probe: {}", probe.message);
    assert!(probe.ok, "{}", probe.message);
    assert_eq!(probe.read_write, Some(true), "the test needs a read/write key");
    assert!(probe.stations.iter().any(|s| s.id == station), "{:?}", probe.stations);

    // Where the download starts, so only this run's QSO is looked at.
    let mut before = 0;
    wavelog_download(&cfg, station, 0, false, |b| before = before.max(b.last_id)).unwrap();

    // A contact at a time no earlier run used: now, to the second.
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
        as i64;
    // And a callsign no earlier run used: Wavelog's duplicate check is to the
    // minute, so two runs inside one would otherwise collide.
    let suffix: String =
        [now / 26 % 26, now % 26].iter().map(|n| (b'A' + *n as u8) as char).collect();
    let (worked, refused_call) = (format!("EA1T{suffix}"), format!("EA2T{suffix}"));
    let rec = QsoRecord {
        call: worked.clone(),
        name: "José".into(),
        rst_sent: Some(-10),
        rst_rcvd: Some(-12),
        freq_hz: 14_080_000.0,
        mode: "FT4".into(),
        band: "20m".into(),
        start_utc: now,
        end_utc: now,
        my_call: call.clone(),
        my_grid: grid.clone(),
        comment: "sdroxide test".into(),
        ..Default::default()
    };
    let adif = qso_log_to_adif(std::slice::from_ref(&rec));

    let first = wavelog_upload(&cfg, station, &adif).map_err(|f| f.message).unwrap();
    eprintln!("upload: {first}");
    assert_eq!(first, "Wavelog: logged");
    let again = wavelog_upload(&cfg, station, &adif).map_err(|f| f.message).unwrap();
    eprintln!("again: {again}");
    assert_eq!(again, "Wavelog: already in the log");

    // A station callsign that is not the location's is refused, for good.
    let portable =
        QsoRecord { my_call: format!("{call}/P"), call: refused_call.clone(), ..rec.clone() };
    let refused = wavelog_upload(&cfg, station, &qso_log_to_adif(std::slice::from_ref(&portable)))
        .unwrap_err();
    eprintln!("refused: {}", refused.message);
    assert!(!refused.retryable);
    assert!(refused.message.contains("/P"), "{}", refused.message);

    // Back down again, as the startup sync would fetch it.
    let mut got = Vec::new();
    let mut last = before;
    wavelog_download(&cfg, station, before, false, |b| {
        last = last.max(b.last_id);
        got.extend(b.records);
    })
    .unwrap();
    eprintln!("downloaded {} new, cursor {before} → {last}", got.len());
    let mine = got.iter().find(|q| q.call == worked).expect("the uploaded QSO comes back");
    assert_eq!(mine.band, "20m");
    assert_eq!(mine.mode, "FT4", "MFSK/FT4 is read back as FT4");
    assert_eq!(mine.name, "José");
    assert_eq!(mine.start_utc, now);
    assert_eq!(mine.my_call, call);
    assert_eq!(mine.wavelog_origin, station);
    assert!(mine.wavelog_sent);
    assert!(!got.iter().any(|q| q.call == refused_call), "the refused QSO is not there");

    // Nothing more past the new cursor.
    let mut more = 0;
    wavelog_download(&cfg, station, last, false, |b| more += b.records.len()).unwrap();
    assert_eq!(more, 0);

    // Merged into a log that already holds it: recognised, not duplicated.
    let mut log = vec![QsoRecord { id: 1, ..rec.clone() }];
    let m = merge_from_wavelog(&mut log, got, 2);
    assert_eq!(log.iter().filter(|q| q.call == worked).count(), 1);
    assert!(log[0].wavelog_sent);
    eprintln!("merge: {m:?}");

    // The live radio status.
    let radio = WavelogRadioNow { tx_hz: 14_080_000, rx_hz: 14_080_000, mode: "FT4".into() };
    wavelog_send_radio(&cfg, "sdroxide-test", &radio).unwrap();
}
