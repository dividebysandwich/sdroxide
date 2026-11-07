use crate::time::{SECOND, Time};
use std::collections::VecDeque;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Compression {
    #[default]
    Off,
    Text,
    Files,
}

impl Compression {
    pub(crate) fn bits(self) -> u8 {
        match self {
            Compression::Off => 0,
            Compression::Text => 1,
            Compression::Files => 2,
        }
    }

    pub(crate) fn from_bits(b: u8) -> Self {
        match b {
            1 => Compression::Text,
            2 => Compression::Files,
            _ => Compression::Off,
        }
    }
}

const FILES_PAUSE: Time = SECOND * 3 / 20;
const FILES_CHUNK: usize = 32_500;
const MAX_INFLATED: usize = 50_000;

pub fn files_chunk(data: &[u8]) -> Vec<u8> {
    let z = miniz_oxide::deflate::compress_to_vec(data, 6);
    let (d, body) = if z.len() < data.len() { (0x80, z.as_slice()) } else { (0, data) };
    let mut out = vec![d | (body.len() >> 8) as u8, body.len() as u8];
    out.extend_from_slice(body);
    out
}

pub fn chunk_queue(buf: &mut VecDeque<u8>) {
    if !buf.is_empty() {
        let raw: Vec<u8> = buf.drain(..).collect();
        for c in raw.chunks(FILES_CHUNK) {
            buf.extend(files_chunk(c));
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct FilesStager {
    stage: Vec<u8>,
    flush_at: Option<Time>,
}

impl FilesStager {
    pub fn len(&self) -> usize {
        self.stage.len()
    }

    pub fn is_empty(&self) -> bool {
        self.stage.is_empty()
    }

    pub fn deadline(&self) -> Option<Time> {
        self.flush_at
    }

    pub fn due(&self, now: Time) -> bool {
        self.flush_at.is_some_and(|t| now >= t)
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn push(&mut self, now: Time, data: &[u8], out: &mut VecDeque<u8>) {
        self.stage.extend_from_slice(data);
        self.flush_at = Some(now + FILES_PAUSE);
        while self.stage.len() >= FILES_CHUNK {
            let rest = self.stage.split_off(FILES_CHUNK);
            let c = std::mem::replace(&mut self.stage, rest);
            out.extend(files_chunk(&c));
        }
    }

    pub fn flush(&mut self, out: &mut VecDeque<u8>) {
        self.flush_at = None;
        if !self.stage.is_empty() {
            let c = std::mem::take(&mut self.stage);
            out.extend(files_chunk(&c));
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct FilesReader {
    buf: Vec<u8>,
}

impl FilesReader {
    pub fn push(&mut self, data: Vec<u8>) -> Vec<Vec<u8>> {
        self.buf.extend(data);
        let mut blocks = Vec::new();
        loop {
            let f = &self.buf;
            if f.len() < 2 {
                return blocks;
            }
            let len = usize::from(f[0] & 0x7f) << 8 | usize::from(f[1]);
            if f.len() < 2 + len {
                return blocks;
            }
            let deflated = f[0] & 0x80 != 0;
            let body: Vec<u8> = self.buf.drain(..2 + len).skip(2).collect();
            let out = if deflated {
                miniz_oxide::inflate::decompress_to_vec_with_limit(&body, MAX_INFLATED).unwrap_or_default()
            } else {
                body
            };
            if !out.is_empty() {
                blocks.push(out);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk(deflated: bool, body: &[u8]) -> Vec<u8> {
        [vec![u8::from(deflated) << 7 | (body.len() >> 8) as u8, body.len() as u8], body.to_vec()].concat()
    }

    #[test]
    fn round_trip() {
        let text = b"the quick brown fox jumps over the lazy dog ".repeat(40);
        let mut r = FilesReader::default();
        assert_eq!(r.push(files_chunk(&text)), vec![text.clone()]);
        assert_eq!(r.push(files_chunk(b"xy")), vec![b"xy".to_vec()]);
    }

    #[test]
    fn bad_deflate_is_dropped() {
        let mut r = FilesReader::default();
        let mut s = chunk(true, &[0xFF; 20]);
        s.extend(chunk(false, b"next"));
        assert_eq!(r.push(s), vec![b"next".to_vec()]);
    }

    #[test]
    fn inflate_is_capped() {
        let fits = miniz_oxide::deflate::compress_to_vec(&vec![0; MAX_INFLATED], 6);
        let bomb = miniz_oxide::deflate::compress_to_vec(&vec![0; MAX_INFLATED + 1], 6);
        let mut r = FilesReader::default();
        assert_eq!(r.push(chunk(true, &fits)), vec![vec![0; MAX_INFLATED]]);
        assert!(r.push(chunk(true, &bomb)).is_empty());
        assert_eq!(r.push(chunk(false, b"ok")), vec![b"ok".to_vec()]);
    }
}
