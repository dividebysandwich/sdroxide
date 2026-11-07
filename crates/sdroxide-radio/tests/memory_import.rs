//! Importing a channel list (issue #234).
//!
//! Somebody with four hundred local repeaters is not going to store them one at
//! a time, and the file they already have is a CHIRP CSV — that is what every
//! repeater directory exports. What is checked here is the engine's half of it:
//! the channels arrive, the engine — not the sender — decides their ids, and a
//! second import of the same list adds nothing.
//!
//! One test function on purpose: `SDROXIDE_CONFIG_DIR` is process-global, and
//! this one writes a real `memories.json` under it.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use sdroxide_radio::{Complex32, EngineConfig, IqSource, Result, start_engine};
use sdroxide_types::{Command, DeviceCaps, MemoryChannel, Mode, RadioEvent, RxId, Shift, Vfo};

const RATE: f64 = 192_000.0;

/// A repeater directory as CHIRP writes one, plus a channel that is already in
/// the list and a line that is not a channel at all.
const LIST: &str = "\
Location,Name,Frequency,Duplex,Offset,Tone,rToneFreq,cToneFreq,DtcsCode,DtcsPolarity,RxDtcsCode,CrossMode,Mode,TStep,Skip,Power,Comment
0,OE1XUU,145.612500,-,0.600000,Tone,123.0,88.5,023,NN,023,Tone->Tone,FM,12.50,,5.0W,Wienerberg
1,S20,145.500000,,0.000000,,88.5,88.5,023,NN,023,Tone->Tone,FM,12.50,,5.0W,calling
2,broken,not-a-frequency,,0.000000,,88.5,88.5,023,NN,023,Tone->Tone,FM,12.50,,5.0W,
3,OE3XOS,438.850000,-,7.600000,DTCS,88.5,88.5,131,NN,023,Tone->Tone,FM,12.50,,5.0W,
";

struct Rig {
    center: Arc<Mutex<f64>>,
}

impl IqSource for Rig {
    fn sample_rate(&self) -> f64 {
        RATE
    }
    fn center_hz(&self) -> f64 {
        *self.center.lock().unwrap()
    }
    fn set_center_hz(&mut self, hz: f64) -> Result<()> {
        *self.center.lock().unwrap() = hz;
        Ok(())
    }
    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        std::thread::sleep(Duration::from_millis(5));
        let n = buf.len().min(256);
        buf[..n].fill(Complex32::new(0.0, 0.0));
        Ok(n)
    }
    fn describe(&self) -> String {
        "import bench".into()
    }
}

fn caps() -> DeviceCaps {
    DeviceCaps {
        driver: "bench".into(),
        label: "bench rig".into(),
        rx_channels: 1,
        sample_rates: vec![RATE],
        freq_ranges_rx: vec![(1_000_000.0, 1_000_000_000.0)],
        ..DeviceCaps::default()
    }
}

/// The memory list as the engine last announced it, once it holds `want`
/// channels — the announcement is what says the command has been acted on.
fn memories(rx: &crossbeam_channel::Receiver<RadioEvent>, want: usize) -> Vec<MemoryChannel> {
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut last = Vec::new();
    while Instant::now() < deadline {
        if let Ok(RadioEvent::Memories(m)) = rx.recv_timeout(Duration::from_millis(100)) {
            last = m;
            if last.len() == want {
                return last;
            }
        }
    }
    panic!("the engine never announced {want} memories; last held {}", last.len());
}

#[test]
fn a_channel_list_is_imported_once_however_often_it_is_offered() {
    let root = std::env::temp_dir().join(format!("sdroxide-memory-import-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &root) };

    let center = Arc::new(Mutex::new(145_500_000.0));
    let mut h = start_engine(
        Box::new(Rig { center }),
        caps(),
        EngineConfig { remember_session: false, ..Default::default() },
    );
    let thread = h.thread.take();
    let send = |c: Command| h.cmd_tx.send(c).unwrap();

    // One channel stored by hand first, so the duplicate check has something
    // to find: 145.500 NFM is in the list about to be imported.
    send(Command::SetVfo { vfo: Vfo::A, hz: 145_500_000.0 });
    send(Command::SetMode { rx: RxId::Main, mode: Mode::Nfm });
    send(Command::StoreMemory { name: "calling".into() });
    let first = memories(&h.event_rx, 1);
    assert_eq!(first[0].name, "calling");

    // The import. Three of the file's four lines are channels, and one of
    // those three is the one already stored.
    let (parsed, skipped) = sdroxide_types::chirp_csv_to_memories(LIST);
    assert_eq!((parsed.len(), skipped), (3, 1), "the unreadable line costs only itself");
    send(Command::ImportMemories(parsed.clone()));
    let after = memories(&h.event_rx, 3);

    // Ids are the engine's, not the file's — every imported channel arrived
    // with id 0, and two channels sharing an id would be one channel to
    // everything that recalls, edits or deletes by it.
    let mut ids: Vec<u32> = after.iter().map(|m| m.id).collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), 3, "every channel has an id of its own");
    assert!(ids.iter().all(|&i| i > 0));

    // The repeater set-up came with the channel, which is the whole point: a
    // frequency without the shift and the tone is a channel you still have to
    // look up.
    let rpt = after.iter().find(|m| m.name == "OE1XUU").expect("the 2 m machine was imported");
    let r = rpt.repeater.expect("a repeater memory carries its set-up");
    assert_eq!((r.shift, r.offset_hz), (Shift::Minus, 600_000));
    assert_eq!(r.ctcss_tenths, 1230);
    let uhf = after.iter().find(|m| m.name == "OE3XOS").unwrap();
    assert_eq!(uhf.repeater.unwrap().offset_hz, 7_600_000);

    // Offered again — the operator re-downloading an updated directory — it
    // adds nothing, and the hand-stored channel keeps the name it was given
    // rather than being replaced by the file's.
    send(Command::ImportMemories(parsed));
    send(Command::StoreMemory { name: "marker".into() });
    let again = memories(&h.event_rx, 4);
    assert_eq!(again.iter().filter(|m| m.name == "marker").count(), 1);
    assert!(
        again.iter().any(|m| m.name == "calling"),
        "the channel that was already stored is left exactly as it was"
    );
    assert!(!again.iter().any(|m| m.name == "S20"), "and the file's copy of it was not added");

    // Cleared, the list starts over: the one stored afterwards is all of it.
    send(Command::ClearMemories);
    send(Command::StoreMemory { name: "fresh".into() });
    let cleared = memories(&h.event_rx, 1);
    assert_eq!(cleared[0].name, "fresh");

    // And it is on disk, not merely announced.
    drop(h);
    let _ = thread.map(|t| t.join());
    let saved = sdroxide_config::load_memories();
    assert_eq!(saved.len(), 1);
    let _ = std::fs::remove_dir_all(&root);
}
