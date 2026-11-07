//! Carrying a station's settings to another installation, and what a bundle
//! from somewhere else is allowed to do when it gets there (issue #356).
//!
//! One test in the binary, because `SDROXIDE_CONFIG_DIR` is process-global and
//! setting it from a `#[test]` that shares a binary with others would race them
//! — the same rule `quarantine.rs` next door follows.

use std::fs;

use sdroxide_config::transfer;

#[test]
fn settings_travel_between_stations_and_a_bundle_cannot_write_outside_the_config_dir() {
    let root = std::env::temp_dir().join(format!("sdroxide-transfer-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let (from, to) = (root.join("from"), root.join("to"));
    fs::create_dir_all(from.join("radio-1")).expect("scratch dir");
    fs::create_dir_all(&to).expect("scratch dir");
    let at = |dir: &std::path::Path| {
        // SAFETY: this is the only test in this binary; nothing races the setter.
        unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", dir) };
    };

    // ── The station being copied from ──
    at(&from);
    fs::write(from.join("config.toml"), "callsign = \"KG4O\"\n").unwrap();
    fs::write(from.join("memories.json"), "[1,2,3]").unwrap();
    fs::write(from.join("radio-1/radio.json"), "{\"backend\":\"HackRf\"}").unwrap();
    fs::write(from.join("bandstacks-radio.json"), "{}").unwrap();
    fs::write(from.join("radio-1/bandstacks-radio.json"), "{}").unwrap();
    // Three things that are in the directory and are not settings.
    fs::write(from.join("qso_log.json"), "[{\"call\":\"W1AW\"}]").unwrap();
    fs::write(from.join("remote_login.json"), "{\"password\":\"hunter2\"}").unwrap();
    fs::write(from.join("radio.json.bak"), "quarantined").unwrap();

    let bundle = transfer::export().expect("exports");
    let names: Vec<&str> = bundle.files.keys().map(String::as_str).collect();
    assert_eq!(
        names,
        [
            "bandstacks-radio.json",
            "config.toml",
            "memories.json",
            "radio-1/bandstacks-radio.json",
            "radio-1/radio.json",
        ]
    );
    assert_eq!(
        bundle.files["config.toml"], "callsign = \"KG4O\"\n",
        "carried verbatim, so a setting this version has never heard of survives the trip"
    );
    let json = transfer::export_json().expect("serialises");

    // ── The station being copied to ──
    at(&to);
    fs::write(to.join("config.toml"), "callsign = \"W1AW\"\n").unwrap();
    fs::write(to.join("bandstacks.json"), "[9]").unwrap();

    let report = transfer::import(&json).expect("imports");
    assert_eq!(
        report.written,
        [
            "bandstacks-radio.json",
            "config.toml",
            "memories.json",
            "radio-1/bandstacks-radio.json",
            "radio-1/radio.json",
        ]
    );
    assert_eq!(fs::read_to_string(to.join("bandstacks-radio.json")).unwrap(), "{}");
    assert_eq!(fs::read_to_string(to.join("radio-1/bandstacks-radio.json")).unwrap(), "{}");
    assert!(report.skipped.is_empty(), "{:?}", report.skipped);
    assert_eq!(fs::read_to_string(to.join("config.toml")).unwrap(), "callsign = \"KG4O\"\n");
    assert_eq!(
        fs::read_to_string(to.join("radio-1/radio.json")).unwrap(),
        "{\"backend\":\"HackRf\"}"
    );
    assert_eq!(
        fs::read_to_string(to.join("bandstacks.json")).unwrap(),
        "[9]",
        "a file the bundle never mentioned is left exactly as it was"
    );
    assert!(!to.join("qso_log.json").exists(), "the logbook does not travel");
    assert!(!to.join("remote_login.json").exists(), "and neither does a saved password");

    // ── A bundle is a file people email each other ──
    //
    // Every path here is somebody else's string, and none of them may reach the
    // filesystem. `escaped` is the one that would matter: two directories above
    // the config directory is the home directory on every platform sdroxide
    // runs on.
    let escaped = root.join("escaped.toml");
    let hostile = format!(
        "{{\"kind\":\"sdroxide-settings\",\"format\":1,\"version\":\"x\",\"exported_unix\":0,\
          \"files\":{{{}}}}}",
        [
            "../../escaped.toml",
            "radio-1/../../escaped.toml",
            "/etc/passwd",
            "radio-1/sub/radio.json",
            "notes/radio.json",
            ".ssh/authorized_keys",
            "run.sh",
            "qso_log.json",
        ]
        .map(|p| format!("{p:?}:\"pwned\""))
        .join(",")
    );
    let report = transfer::import(&hostile).expect("a hostile bundle is reported, not applied");
    assert!(report.written.is_empty(), "nothing in it may be written: {:?}", report.written);
    assert_eq!(report.skipped.len(), 8, "and every entry is reported: {:?}", report.skipped);
    assert!(!escaped.exists(), "nothing may land outside the config directory");
    assert!(!to.join("run.sh").exists());
    assert!(!to.join("qso_log.json").exists());

    // A file that is not a bundle at all, or is from a newer sdroxide than can
    // read it, is refused outright rather than half-applied.
    assert!(transfer::import("not json at all").is_err());
    let with = |kind: &str, format: u32| {
        format!(
            "{{\"kind\":\"{kind}\",\"format\":{format},\"version\":\"x\",\
              \"exported_unix\":0,\"files\":{{}}}}"
        )
    };
    assert!(transfer::import(&with("something-else", 1)).is_err());
    assert!(transfer::import(&with("sdroxide-settings", 99)).is_err());
    assert!(transfer::import(&with("sdroxide-settings", 1)).is_ok());

    unsafe { std::env::remove_var("SDROXIDE_CONFIG_DIR") };
}
