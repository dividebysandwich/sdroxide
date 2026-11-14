//! No backend yet (macOS among them): nothing is ever found.

use std::io;

use super::{Device, Found};

pub fn find() -> Vec<Found> {
    Vec::new()
}

pub fn open(_key: &str) -> io::Result<Box<dyn Device>> {
    Err(io::Error::new(io::ErrorKind::Unsupported, "TMate 2 is not supported on this platform yet"))
}
