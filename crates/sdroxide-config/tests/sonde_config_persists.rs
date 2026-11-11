//! The radiosonde lane remembers what it was told: its on/off and its channel
//! come back from `sonde.json`, and a channel outside the sonde band is
//! brought back into it where the file is read.
//!
//! Its own test binary, because `SDROXIDE_CONFIG_DIR` is process-global.

#[test]
fn the_sonde_lane_remembers_its_channel_and_clamps_a_stray_one() {
    let dir = std::env::temp_dir().join(format!("sdroxide-sondecfg-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &dir) };

    let fresh = sdroxide_config::load_sonde_config();
    assert!(!fresh.enabled, "a fresh configuration starts with the sonde lane off");
    assert_eq!(fresh.frequency_hz, sdroxide_types::SONDE_DEFAULT_HZ);

    let mine =
        sdroxide_types::SondeSettings { enabled: true, frequency_hz: 404_500_000.0, scan: true };
    sdroxide_config::save_sonde_config(&mine).unwrap();
    assert!(dir.join("sonde.json").exists());
    assert_eq!(sdroxide_config::load_sonde_config(), mine, "the lane forgot what it was told");

    // A converter offset saved as a channel lands at the top of the band
    // rather than tuning the lane to nowhere.
    let stray = sdroxide_types::SondeSettings { enabled: true, frequency_hz: 6.0e9, scan: false };
    sdroxide_config::save_sonde_config(&stray).unwrap();
    assert_eq!(sdroxide_config::load_sonde_config().frequency_hz, sdroxide_types::SONDE_BAND_HZ.1);

    let _ = std::fs::remove_dir_all(&dir);
}
