#[derive(Clone, Copy, Debug, Default)]
pub struct B2fSnoop {
    fs: bool,
    quit: bool,
}

impl B2fSnoop {
    pub fn observe(&mut self, block: &[u8]) {
        self.quit |= block == b"FQ\r";
        self.fs |= block.len() >= 5 && block.starts_with(b"FS ") && block.ends_with(b"\r");
    }

    pub fn fs(&self) -> bool {
        self.fs
    }

    pub fn clear_fs(&mut self) {
        self.fs = false;
    }

    pub fn take_quit(&mut self) -> bool {
        std::mem::take(&mut self.quit)
    }
}
