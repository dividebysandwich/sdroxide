//! The weather-satellite lane remembers what it was told: its on/off and its
//! channel come back from `wxsat.json`, and a channel outside the 137 MHz band
//! is brought back into it where the file is read.
//!
//! Its own test binary, because `SDROXIDE_CONFIG_DIR` is process-global.

#[test]
fn the_wxsat_lane_remembers_its_channel_and_clamps_a_stray_one() {
    let dir = std::env::temp_dir().join(format!("sdroxide-wxsatcfg-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &dir) };

    let fresh = sdroxide_config::load_wxsat_config();
    assert!(!fresh.enabled, "a fresh configuration starts with the lane off");
    assert_eq!(fresh.frequency_hz, sdroxide_types::WXSAT_DEFAULT_HZ);

    let mine = sdroxide_types::WxsatSettings {
        enabled: true,
        frequency_hz: 137_900_000.0,
        kind: sdroxide_types::WxsatKind::Lrpt,
    };
    sdroxide_config::save_wxsat_config(&mine).unwrap();
    assert!(dir.join("wxsat.json").exists());
    assert_eq!(sdroxide_config::load_wxsat_config(), mine, "the lane forgot what it was told");

    let stray =
        sdroxide_types::WxsatSettings { enabled: true, frequency_hz: 6.0e9, ..Default::default() };
    sdroxide_config::save_wxsat_config(&stray).unwrap();
    assert_eq!(sdroxide_config::load_wxsat_config().frequency_hz, sdroxide_types::WXSAT_BAND_HZ.1);

    let _ = std::fs::remove_dir_all(&dir);
}
