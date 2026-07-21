//! Manual CAT smoke test: `cargo run -p sdroxide-cat --example catprobe -- /dev/ttyACM0 19200`
//! Polls the rig and prints reported freq/mode; also sets a test frequency.

use sdroxide_types::{CatConfig, CatFamily, SerialConfig};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path = args.get(1).cloned().unwrap_or_else(|| "/dev/ttyACM1".into());
    let baud: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(19200);
    println!("ports: {:?}", sdroxide_cat::available_ports());
    println!("opening {path} @ {baud}");

    let cfg = CatConfig {
        family: CatFamily::Xiegu,
        serial: SerialConfig { path, baud, ..Default::default() },
        icom_radio_id: 0x70,
        poll_hz: 4.0,
        ..Default::default()
    };
    let h = sdroxide_cat::spawn(cfg);
    for i in 0..40 {
        std::thread::sleep(std::time::Duration::from_millis(250));
        for u in h.poll() {
            println!("[{i}] update: {u:?}");
        }
        if i == 8 {
            println!("--- setting 14.074000 MHz ---");
            h.set_freq(14_074_000.0);
        }
    }
}
