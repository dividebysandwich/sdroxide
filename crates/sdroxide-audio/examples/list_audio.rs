fn main() {
    println!("INPUT devices:");
    for n in sdroxide_audio::input_device_names() { println!("  {n:?}"); }
    println!("OUTPUT devices:");
    for n in sdroxide_audio::output_device_names() { println!("  {n:?}"); }
}
