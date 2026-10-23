//! What a `memories.json` the operator cannot afford to lose has to survive.
//!
//! One test per file on purpose: `SDROXIDE_CONFIG_DIR` is process-global, and
//! setting it from a `#[test]` that shares a binary with others would race them.
//!
//! Issue #269: an operator upgraded, found every stored channel gone and the
//! folders still there — the two live in separate files, so whatever happened
//! happened to the list alone. Two paths could do that and both are closed
//! here.

use std::fs;

use sdroxide_types::{MemoryChannel, Mode};

fn chan(id: u32, name: &str, hz: f64) -> MemoryChannel {
    MemoryChannel {
        id,
        name: name.into(),
        freq_hz: hz,
        mode: Mode::Nfm,
        filter_lo: -8000.0,
        filter_hi: 8000.0,
        folder: None,
        rtty: None,
        repeater: None,
        antenna: None,
        color: None,
    }
}

#[test]
fn a_memory_list_is_never_lost_wholesale() {
    let root = std::env::temp_dir().join(format!("sdroxide-memlist-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("scratch dir");
    // SAFETY: this is the only test in this binary; nothing races the setter.
    unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &root) };
    let path = root.join("memories.json");

    // A first run: no file, no list, nothing to report.
    assert!(sdroxide_config::load_memories().is_empty());
    assert!(sdroxide_config::take_load_alerts().is_empty(), "an absent file is not a fault");

    // The operator's list, written and read back as it always was.
    let mine = vec![chan(1, "Airband", 118_500_000.0), chan(2, "Repeater", 145_600_000.0)];
    sdroxide_config::save_memories(&mine).expect("save");
    assert_eq!(sdroxide_config::load_memories(), mine);

    // ── One bad channel must not cost the other two ──────────────────────
    //
    // `serde_json` fails a `Vec<T>` whole, so a single row carrying something
    // this build cannot read used to take every other row with it — the file
    // was quarantined and the operator opened the MEM window on nothing. Here
    // the middle row names a mode that does not exist (which is what a
    // downgrade, or a hand edit, looks like from the loader's side).
    let text = format!(
        "[{},{},{}]",
        serde_json::to_string(&chan(1, "Airband", 118_500_000.0)).unwrap(),
        serde_json::to_string(&chan(2, "From the future", 145_600_000.0))
            .unwrap()
            .replace("\"Nfm\"", "\"Telepathy\""),
        serde_json::to_string(&chan(3, "Weather", 162_400_000.0)).unwrap(),
    );
    fs::write(&path, &text).unwrap();
    let kept = sdroxide_config::load_memories();
    assert_eq!(kept.len(), 2, "the readable channels have to survive: {kept:?}");
    assert_eq!(kept[0].name, "Airband");
    assert_eq!(kept[1].name, "Weather");
    // The file as it was is kept, so the one dropped row is recoverable by
    // hand — and copied rather than renamed, because the survivors are live and
    // the next save has to land somewhere.
    assert_eq!(fs::read_to_string(root.join("memories.json.bak")).unwrap(), text);
    assert!(path.exists(), "the live file stays where the next save expects it");
    let alerts = sdroxide_config::take_load_alerts();
    assert_eq!(alerts.len(), 1, "{alerts:?}");
    assert!(alerts[0].contains("1 of 3"), "the alert has to count them: {}", alerts[0]);
    assert!(alerts[0].contains("MEM window"), "...and say where to look: {}", alerts[0]);

    // Saving the survivors is allowed: nothing is being hidden, the file that
    // held the third one is beside it.
    sdroxide_config::save_memories(&kept).expect("the survivors are saveable");
    assert_eq!(sdroxide_config::load_memories(), kept);
    assert!(sdroxide_config::take_load_alerts().is_empty(), "a clean file says nothing");

    // ── A file that cannot be *read* is not a file that says nothing ─────
    //
    // The other half of #269, and the one that destroyed data: a read that
    // failed for any reason other than "no such file" used to be treated
    // exactly like a first run. sdroxide came up with an empty list, said
    // nothing, and the next memory the operator stored wrote that empty list
    // back over everything they had. A directory in the file's place is the
    // portable way to make a read fail — it is what a locked file, a
    // permission problem or a bad sector looks like from here.
    fs::remove_file(&path).unwrap();
    fs::create_dir(&path).unwrap();
    assert!(sdroxide_config::load_memories().is_empty(), "there is nothing it can return");
    let alerts = sdroxide_config::take_load_alerts();
    assert_eq!(alerts.len(), 1, "the operator has to be told: {alerts:?}");
    assert!(alerts[0].contains("could not be read"), "{}", alerts[0]);

    // ...and the save is refused rather than completed, because what is on the
    // disk is the operator's list and what is in hand is the empty default.
    let err = sdroxide_config::save_memories(&[]).expect_err("this save must not go through");
    assert!(err.to_string().contains("memories.json"), "{err}");
    // Said once per file, not once per load: the alert is not re-queued by the
    // loads that keep finding it unreadable.
    let _ = sdroxide_config::load_memories();
    assert!(sdroxide_config::take_load_alerts().is_empty());

    // A readable file is still not a writable one until something has read it.
    // The list in memory is the empty default this session was forced to run
    // on, and writing *that* is the whole of what the refusal exists to stop —
    // so a backup tool letting go of the file must not, by itself, hand the
    // empty list a way through.
    fs::remove_dir(&path).unwrap();
    fs::write(&path, serde_json::to_string(&mine).unwrap()).unwrap();
    sdroxide_config::save_memories(&[]).expect_err("nothing has read the file yet");

    // The load is what clears it, because the load is what puts the operator's
    // channels back in memory. From there everything works as before.
    assert_eq!(sdroxide_config::load_memories(), mine, "the list is back");
    sdroxide_config::save_memories(&mine).expect("a file that has been read may be written");
    assert_eq!(sdroxide_config::load_memories(), mine);

    unsafe { std::env::remove_var("SDROXIDE_CONFIG_DIR") };
    let _ = fs::remove_dir_all(&root);
}
