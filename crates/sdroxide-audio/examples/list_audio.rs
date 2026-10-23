//! **Which sound cards this program actually sees** — the question behind more
//! than one "the radio is not working" report, because `--probe` lists SDR
//! hardware and nothing else, and a device that the sound server knows about may
//! be one cpal never enumerates (the reverse is also true).
//!
//! It prints the names in the form the **settings pickers** use and the engine
//! logs, which is the form a saved `radio.json` holds — so a name in a report can
//! be compared with what is here, character for character, including the card id
//! and the USB id at the end. A card whose id moved (`Audio` → `Audio_1`) after
//! another USB sound device enumerated first is visible in exactly that suffix.
//!
//! `cargo run -p sdroxide-audio --release --example list_audio`

fn main() {
    println!("INPUT devices:");
    for n in sdroxide_audio::input_device_names() {
        println!("  {n:?}");
    }
    println!("OUTPUT devices:");
    for n in sdroxide_audio::output_device_names() {
        println!("  {n:?}");
    }
}
