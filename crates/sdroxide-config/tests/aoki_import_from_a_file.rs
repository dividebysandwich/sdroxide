//! **A schedule the downloader cannot reach is still a schedule.**
//!
//! Aoki publishes its short-wave list on a personal site whose path changes with
//! the season, so the automatic download is the part that can break — and when
//! it does, the program is the one that cannot do what it says. `import_schedule_file`
//! takes the same file from disk: the `.zip` Aoki publishes, or the `.txt` from
//! inside it, saved by hand.
//!
//! Two things this pins, and both are about the **cache** rather than the parser:
//! a hand-imported file lands where the downloader puts its own, so the reader
//! finds it with no second code path; and importing a second file removes the
//! first, because the reader takes the newest `aoki-*.txt` **by name** and a
//! season whose name sorts below last year's would otherwise lose to it and make
//! the import look like it did nothing.
//!
//! The rows are the published file's own shapes, copied from `xta26.txt` (header
//! and one row, fixed columns and all) and repeated: the import refuses fewer
//! than 500 transmissions, and what is under test here is the path the file
//! takes, not the parser, which has its own tests.

use sdroxide_config::{cached_schedule_for, import_schedule_file};
use sdroxide_types::ScheduleSource;

/// Aoki's own title line, copied byte for byte — misspelling and all.
const TITLE: &str = "A26 Shortwave Frequecy List  October 18  2026,  1400 UTC   Day 1 = Sunday";

/// Aoki's column header, copied byte for byte.
const HEADER: &str = "FRE   STATION                         UTC       Su-W-Sa Language             Pow Azi Location                ADM L/L             Remarks";

/// One real row, in the 49 m band — which matters: the parser keeps only the
/// short-wave bands, so a medium-wave row would parse to nothing and the fixture
/// would be testing the filter instead of the import.
const ROW: &str = " 5900 BIBLE VOICE BCN Radio Dardasha7 1930-1945 1234567 Arabic               250 126 Kostinbrod(Sofia)       BUL 424833N0231113E BVBN a26 MBR";

/// A whole file's worth: the title, the header and enough rows to clear the
/// 500-row floor.
fn schedule_text() -> String {
    let mut text = String::with_capacity((TITLE.len() + HEADER.len() + ROW.len() + 3) * 600);
    text.push_str(TITLE);
    text.push('\n');
    text.push_str(HEADER);
    text.push('\n');
    for _ in 0..600 {
        text.push_str(ROW);
        text.push('\n');
    }
    text
}

/// A stored (uncompressed) zip, which is the one layout this test can build byte
/// for byte and the reader accepts — decompressing is the crate's business and
/// already covered by its own unit tests.
fn stored_zip(name: &str, body: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut dir = Vec::new();
    let lho = 0u32;

    out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
    out.extend_from_slice(&20u16.to_le_bytes()); // version needed
    out.extend_from_slice(&0u16.to_le_bytes()); // flags
    out.extend_from_slice(&0u16.to_le_bytes()); // method: stored
    out.extend_from_slice(&[0, 0, 0, 0]); // time, date
    out.extend_from_slice(&0u32.to_le_bytes()); // crc (not read back)
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(&(name.len() as u16).to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes()); // extra length
    out.extend_from_slice(name.as_bytes());
    out.extend_from_slice(body);

    dir.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
    dir.extend_from_slice(&20u16.to_le_bytes()); // version made by
    dir.extend_from_slice(&20u16.to_le_bytes()); // version needed
    dir.extend_from_slice(&0u16.to_le_bytes()); // flags
    dir.extend_from_slice(&0u16.to_le_bytes()); // method
    dir.extend_from_slice(&[0, 0, 0, 0]); // time, date
    dir.extend_from_slice(&0u32.to_le_bytes()); // crc
    dir.extend_from_slice(&(body.len() as u32).to_le_bytes());
    dir.extend_from_slice(&(body.len() as u32).to_le_bytes());
    dir.extend_from_slice(&(name.len() as u16).to_le_bytes());
    dir.extend_from_slice(&[0u8; 8]); // extra, comment, disk-start, internal attrs
    dir.extend_from_slice(&0u32.to_le_bytes()); // external attributes
    dir.extend_from_slice(&lho.to_le_bytes());
    dir.extend_from_slice(name.as_bytes());

    let cen = out.len() as u32;
    let cen_len = dir.len() as u32;
    out.extend_from_slice(&dir);
    out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes()); // disk number
    out.extend_from_slice(&0u16.to_le_bytes()); // disk with the directory
    out.extend_from_slice(&1u16.to_le_bytes()); // entries on this disk
    out.extend_from_slice(&1u16.to_le_bytes()); // entries in total
    out.extend_from_slice(&cen_len.to_le_bytes());
    out.extend_from_slice(&cen.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes()); // comment length
    out
}

/// `SDROXIDE_CONFIG_DIR` is process-global, and tests in one binary run in
/// parallel — so the sandboxes take turns. Without this the second test to run
/// finds the third one's directory and every assertion that reads it is a lie.
static ONE_AT_A_TIME: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn one_at_a_time() -> std::sync::MutexGuard<'static, ()> {
    ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner())
}

/// A configuration directory of our own.
fn sandbox(tag: &str) -> std::path::PathBuf {
    let root =
        std::env::temp_dir().join(format!("sdroxide-aoki-import-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("scratch dir");
    // SAFETY: one test binary, one process, set before anything reads it.
    unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &root) };
    root
}

/// The `.txt` out of the published file — what an operator has after unzipping
/// it, or after saving the page — and the reader must find it afterwards.
#[test]
fn an_aoki_txt_imported_by_hand_is_the_schedule_after_a_restart() {
    let _turn = one_at_a_time();
    let root = sandbox("txt");
    let file = root.join("xta26.txt");
    std::fs::write(&file, schedule_text()).expect("fixture");

    let got = import_schedule_file(&file).expect("the file is a schedule");
    assert_eq!(got.source, ScheduleSource::Aoki, "read as Aoki's list");
    assert_eq!(got.season, "xta26", "the name it carries is the season it is filed under");
    assert_eq!(got.stations, 600, "every row parses: {got:?}");

    let cache = root.join("broadcast").join("aoki-xta26.txt");
    assert!(cache.exists(), "the import lands where a download would: {}", cache.display());

    // The proof that matters is not the file but the **reader**: the same call
    // the app makes at startup must now return this schedule.
    // `merge` lays the built-in time-signal and utility rows over any schedule,
    // so the count is "at least what was imported" rather than exactly it.
    let read = cached_schedule_for(ScheduleSource::Aoki).expect("the cache is read back");
    assert!(read.len() >= 600, "and it is the file that was imported: {}", read.len());

    let _ = std::fs::remove_dir_all(&root);
}

/// A second file replaces the first, rather than sitting beside it and losing the
/// lookup, which picks by name.
#[test]
fn importing_another_season_leaves_one_aoki_file_to_find() {
    let _turn = one_at_a_time();
    let root = sandbox("replace");
    let first = root.join("xta26.txt");
    std::fs::write(&first, schedule_text()).expect("fixture");
    import_schedule_file(&first).expect("the first file");

    // `ncb24` sorts **below** `xta26`, and the reader takes the newest name, so
    // left in place it would win and the import would look like nothing
    // happened.
    let second = root.join("ncb24.txt");
    std::fs::write(&second, schedule_text()).expect("fixture");
    let got = import_schedule_file(&second).expect("the second file");
    assert_eq!(got.season, "ncb24");

    let mut left: Vec<String> = std::fs::read_dir(root.join("broadcast"))
        .expect("cache dir")
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with("aoki-"))
        .collect();
    left.sort();
    assert_eq!(left, vec!["aoki-ncb24.txt"], "one Aoki file, and it is the new one");

    let _ = std::fs::remove_dir_all(&root);
}

/// Aoki's published `.zip`, which is the file the downloader would have fetched.
#[test]
fn the_published_zip_is_accepted_as_it_comes() {
    let _turn = one_at_a_time();
    let root = sandbox("zip");
    let file = root.join("xta26.zip");
    std::fs::write(&file, stored_zip("xta26.txt", schedule_text().as_bytes())).expect("fixture");

    let got = import_schedule_file(&file).expect("the zip is a schedule");
    assert_eq!(got.source, ScheduleSource::Aoki);
    assert_eq!(got.season, "xta26");
    assert_eq!(got.stations, 600);
    assert!(
        cached_schedule_for(ScheduleSource::Aoki).is_some_and(|s| s.len() >= 600),
        "and the cache reader unpacks it, which is the path it will take at startup"
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// A file that is neither says so, and says what it expected — a dialog that
/// accepts anything and then reports zero stations would leave the operator
/// wondering which of the two files they saved was the wrong one.
#[test]
fn a_file_that_is_not_a_schedule_is_refused_by_name() {
    let _turn = one_at_a_time();
    let root = sandbox("refuse");
    let file = root.join("notes.txt");
    std::fs::write(&file, "shopping list\nnothing to see here\n").expect("fixture");

    let err = import_schedule_file(&file).expect_err("not a schedule");
    assert!(err.contains("not a schedule this program knows"), "{err}");
    assert!(err.contains("notes.txt"), "the message names the file: {err}");
    assert!(
        !root.join("broadcast").join("aoki-notes.txt").exists(),
        "and nothing is cached from it"
    );

    let _ = std::fs::remove_dir_all(&root);
}
