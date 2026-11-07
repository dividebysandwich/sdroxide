// The message files are embedded by rust-embed, which only notices a file it
// has already seen. Watching the folder makes a new language, or a new file
// in one, rebuild the crate.
fn main() {
    println!("cargo:rerun-if-changed=i18n");
}
