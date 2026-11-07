//! Each radio's WSPR beacon settings are its own (issue #615).
//!
//! One test per file on purpose: `SDROXIDE_CONFIG_DIR` is process-global, and
//! setting it from a `#[test]` that shares a binary with others would race them.

use sdroxide_config::Store;

/// Setting radio 1's duty to 33 % used to start radio 2 beaconing as well,
/// because the duty lived in the shared `digi.json` and every radio reloaded
/// it. Now a radio's beacon follows only its own saves, while the operator's
/// shared settings still reach every radio.
#[test]
fn one_radios_beacon_does_not_switch_on_another() {
    let root = std::env::temp_dir().join(format!("sdroxide-wspr-radio-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("scratch dir");
    // SAFETY: this is the only test in this binary; nothing races the setter.
    unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &root) };

    let (first, second) = (Store::radio(0), Store::radio(1));

    // An installation from before the split: the beacon setting is in the
    // shared file, and radio 0 keeps it. A radio added since starts silent.
    let mut legacy = sdroxide_config::load_digi_config();
    legacy.wspr_tx_percent = 20;
    sdroxide_config::save_digi_config(&legacy).unwrap();
    assert_eq!(first.load_digi_config().wspr_tx_percent, 20);
    assert_eq!(second.load_digi_config().wspr_tx_percent, 0);

    // Radio 0 goes to 33 % and changes the shared callsign while it is at it.
    let mut cfg = first.load_digi_config();
    cfg.wspr_tx_percent = 33;
    cfg.my_call = "N0CALL".into();
    first.save_digi_config(&cfg).unwrap();

    // Radio 1 hears the callsign and not the beacon.
    let seen = second.load_digi_config();
    assert_eq!(seen.my_call, "N0CALL");
    assert_eq!(seen.wspr_tx_percent, 0);

    // Radio 1 saving its own (silent) settings leaves radio 0 beaconing,
    // including through the shared file an older build reads.
    let mut cfg = second.load_digi_config();
    cfg.wspr_power_dbm = 30;
    second.save_digi_config(&cfg).unwrap();
    assert_eq!(first.load_digi_config().wspr_tx_percent, 33);
    assert_eq!(first.load_digi_config().wspr_power_dbm, legacy.wspr_power_dbm);
    assert_eq!(sdroxide_config::load_digi_config().wspr_tx_percent, 33);
    assert_eq!(second.load_digi_config().wspr_power_dbm, 30);

    let _ = std::fs::remove_dir_all(&root);
}
