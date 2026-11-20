//! Two engines in one process — the multi-radio arrangement. What these hold:
//! the station's transmit interlock lets exactly one radio key at a time, a
//! shared-store write by one engine reaches the other without either UI
//! doing anything, and each radio's TCI server is its own.

use std::sync::Arc;
use std::time::{Duration, Instant};

use sdroxide_radio::{Complex32, EngineConfig, IqSource, Result, StoreSync, TxGate, start_engine};
use sdroxide_types::{Command, DeviceCaps, RadioEvent};

const RATE: f64 = 2_400_000.0;
const START_CENTER: f64 = 14_200_000.0;

/// A transmit-capable stand-in: silence on receive, and a `tx_begin` that
/// simply accepts, so the engine's rails — including the interlock under test
/// — decide whether keying succeeds.
struct MockTrx {
    center: f64,
}

impl IqSource for MockTrx {
    fn sample_rate(&self) -> f64 {
        RATE
    }
    fn center_hz(&self) -> f64 {
        self.center
    }
    fn set_center_hz(&mut self, hz: f64) -> Result<()> {
        self.center = hz;
        Ok(())
    }
    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        std::thread::sleep(Duration::from_millis(5));
        let n = buf.len().min(2048);
        buf[..n].fill(Complex32::new(0.0, 0.0));
        Ok(n)
    }
    fn describe(&self) -> String {
        "mock trx source".into()
    }
    fn tx_begin(&mut self, _center_hz: f64, rate: f64) -> Result<f64> {
        Ok(rate)
    }
    fn tx_write(&mut self, _samples: &[Complex32]) -> Result<()> {
        std::thread::sleep(Duration::from_millis(2));
        Ok(())
    }
    fn tx_end(&mut self) -> Result<()> {
        Ok(())
    }
}

fn caps() -> DeviceCaps {
    DeviceCaps {
        driver: "mock".into(),
        label: "mock".into(),
        rx_channels: 1,
        tx_channels: 1,
        sample_rates: vec![RATE],
        freq_ranges_rx: vec![(0.0, 1_000_000_000.0)],
        freq_ranges_tx: vec![(0.0, 1_000_000_000.0)],
        ..DeviceCaps::default()
    }
}

struct Radio {
    h: sdroxide_radio::EngineHandles,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Radio {
    fn start(instance: u32, gate: &Arc<TxGate>, sync: &Arc<StoreSync>) -> Radio {
        let mut h = start_engine(
            Box::new(MockTrx { center: START_CENTER }),
            caps(),
            EngineConfig {
                tx_ham_only: false,
                instance,
                primary: instance == 0,
                store: sdroxide_config::Store::radio(instance),
                tx_gate: Some(gate.clone()),
                store_sync: Some(sync.clone()),
                ..Default::default()
            },
        );
        let thread = h.thread.take();
        Radio { h, thread }
    }

    fn send(&self, cmd: Command) {
        self.h.cmd_tx.send(cmd).unwrap();
    }

    /// Drain events until `pred` matches one, or panic at the deadline.
    fn wait_for<T>(&self, what: &str, mut pred: impl FnMut(&RadioEvent) -> Option<T>) -> T {
        let deadline = Instant::now() + Duration::from_secs(3);
        while Instant::now() < deadline {
            while let Ok(ev) = self.h.event_rx.try_recv() {
                if let Some(t) = pred(&ev) {
                    return t;
                }
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!("timed out waiting for {what}");
    }

    fn stop(mut self) {
        drop(self.h.cmd_tx);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

/// One test rather than several: it redirects the config directory through the
/// environment (process-global), and the interlock scenario feeds the
/// store-sync scenario the same pair of running engines.
#[test]
fn two_engines_share_one_transmitter_and_one_set_of_stores() {
    let root = std::env::temp_dir().join(format!("sdroxide-multi-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("scratch dir");
    // SAFETY: this is the only test in this binary; nothing races the setter.
    unsafe { std::env::set_var("SDROXIDE_CONFIG_DIR", &root) };

    // Neither engine may bind the (default-enabled) TCI server port: this is
    // a test, and the operator may have a real sdroxide running.
    let tci_off = sdroxide_types::TciServerConfig { enabled: false, ..Default::default() };
    sdroxide_config::Store::radio(0).save_tci_server_config(&tci_off).unwrap();
    sdroxide_config::Store::radio(1).save_tci_server_config(&tci_off).unwrap();

    let gate = Arc::new(TxGate::new());
    let sync = Arc::new(StoreSync::new());
    let a = Radio::start(0, &gate, &sync);
    let b = Radio::start(1, &gate, &sync);

    // Let both engines publish their opening state.
    a.wait_for("radio A first state", |ev| matches!(ev, RadioEvent::State(_)).then_some(()));
    b.wait_for("radio B first state", |ev| matches!(ev, RadioEvent::State(_)).then_some(()));

    // ── The interlock ────────────────────────────────────────────────────
    a.send(Command::SetPtt(true));
    a.wait_for("radio A keyed", |ev| match ev {
        RadioEvent::State(s) if s.tx.ptt => Some(()),
        _ => None,
    });

    // B keying while A is on the air: refused with a notice naming the holder,
    // and B's own state stays unkeyed.
    b.send(Command::SetPtt(true));
    let notice = b.wait_for("radio B refusal notice", |ev| match ev {
        RadioEvent::Notice(Some(n)) if n.contains("transmit refused") => Some(n.clone()),
        _ => None,
    });
    assert!(notice.contains("radio 1"), "the refusal should name the holder: {notice}");
    b.wait_for("radio B stayed unkeyed", |ev| match ev {
        RadioEvent::State(s) if !s.tx.ptt => Some(()),
        _ => None,
    });

    // A unkeys; the gate is free; B may transmit now.
    a.send(Command::SetPtt(false));
    a.wait_for("radio A unkeyed", |ev| match ev {
        RadioEvent::State(s) if !s.tx.ptt => Some(()),
        _ => None,
    });
    b.send(Command::SetPtt(true));
    b.wait_for("radio B keyed after the gate freed", |ev| match ev {
        RadioEvent::State(s) if s.tx.ptt => Some(()),
        _ => None,
    });
    b.send(Command::SetPtt(false));
    b.wait_for("radio B unkeyed", |ev| match ev {
        RadioEvent::State(s) if !s.tx.ptt => Some(()),
        _ => None,
    });

    // ── The shared stores ────────────────────────────────────────────────
    // A memory stored on radio A appears on radio B with no UI involved: A's
    // save bumps the shared generation, B's loop notices and reloads.
    a.send(Command::StoreMemory { name: "interlock test".into() });
    a.wait_for("radio A memory list", |ev| match ev {
        RadioEvent::Memories(m) if m.iter().any(|m| m.name == "interlock test") => Some(()),
        _ => None,
    });
    b.wait_for("radio B reloaded the memories another engine saved", |ev| match ev {
        RadioEvent::Memories(m) if m.iter().any(|m| m.name == "interlock test") => Some(()),
        _ => None,
    });

    // ── Each radio's own TCI server ──────────────────────────────────────
    // Settings → Servers sends `SetTciServerConfig` down the edited radio's own
    // controller. It has to land in that radio's scope and bind there, leaving
    // the other radio's server and file alone; a second radio set to the same
    // port has to report the failed bind rather than say nothing.
    let port = {
        let probe = std::net::TcpListener::bind("127.0.0.1:0").expect("free port");
        probe.local_addr().unwrap().port()
    };
    let on = sdroxide_types::TciServerConfig { enabled: true, port, ..Default::default() };
    b.send(Command::SetTciServerConfig(on.clone()));
    let at = format!(":{port}");
    b.wait_for("radio B's TCI server listening", |ev| match ev {
        RadioEvent::TciServerStatus { running: true, addr, .. } if addr.ends_with(&at) => Some(()),
        _ => None,
    });
    assert_eq!(sdroxide_config::Store::radio(1).load_tci_server_config(), on);
    assert_eq!(
        sdroxide_config::Store::radio(0).load_tci_server_config(),
        tci_off,
        "radio B's server is configured in radio B's scope only"
    );
    a.send(Command::SetTciServerConfig(on));
    let refused = a.wait_for("radio A's bind refused", |ev| match ev {
        RadioEvent::TciServerStatus { running: false, error: Some(e), .. } => Some(e.clone()),
        _ => None,
    });
    assert!(refused.contains(&port.to_string()), "the failure names the port: {refused}");
    a.send(Command::SetTciServerConfig(tci_off.clone()));
    b.send(Command::SetTciServerConfig(tci_off));

    a.stop();
    b.stop();
    unsafe { std::env::remove_var("SDROXIDE_CONFIG_DIR") };
    let _ = std::fs::remove_dir_all(&root);
}
