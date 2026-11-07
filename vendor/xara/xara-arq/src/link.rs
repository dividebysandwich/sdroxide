use crate::b2f::B2fSnoop;
use crate::compression::{Compression, FilesReader, FilesStager, chunk_queue};
use crate::textcomp::decompress;
use crate::time::Time;
use std::collections::VecDeque;

#[derive(Default, Debug)]
pub(crate) struct RxAsm {
    pub(crate) slots: Reorder,
    pub(crate) turn_compressed: Option<bool>,
}

#[derive(Default, Debug)]
pub(crate) struct Link {
    pub(crate) rx: RxAsm,
    pub(crate) comp: Compression,
    pub(crate) files: FilesStager,
    pub(crate) b2f: B2fSnoop,
    pub(crate) deadline: Option<Time>,
    pub(crate) watchdog: Option<Time>,
    pub(crate) idle_timer: Option<Time>,
    pub(crate) idle_ticks: u32,
    pub(crate) disc_now: bool,
    pub(crate) disc_after_tx: bool,
    pub(crate) disc_timeouts: u32,
    reader: FilesReader,
}

fn fired(t: &mut Option<Time>, now: Time) -> bool {
    let due = t.is_some_and(|t| now >= t);
    if due {
        *t = None;
    }
    due
}

impl Link {
    pub(crate) fn next_deadline(&self) -> Option<Time> {
        [self.deadline, self.watchdog, self.idle_timer, self.files.deadline()].into_iter().flatten().min()
    }

    pub(crate) fn watchdog_fired(&mut self, now: Time) -> bool {
        fired(&mut self.watchdog, now)
    }

    pub(crate) fn deadline_fired(&mut self, now: Time) -> bool {
        fired(&mut self.deadline, now)
    }

    pub(crate) fn idle_fired(&mut self, now: Time) -> Option<u32> {
        fired(&mut self.idle_timer, now).then(|| {
            self.idle_ticks += 1;
            self.idle_ticks
        })
    }

    pub(crate) fn text(&self) -> bool {
        self.comp == Compression::Text
    }

    pub(crate) fn queue(&mut self, now: Time, connected: bool, data: &[u8], buf: &mut VecDeque<u8>) {
        self.b2f.observe(data);
        if connected && self.comp == Compression::Files {
            self.files.push(now, data, buf);
        } else {
            buf.extend(data);
        }
    }

    pub(crate) fn start_files(&self, buf: &mut VecDeque<u8>) {
        if self.comp == Compression::Files {
            chunk_queue(buf);
        }
    }

    pub(crate) fn inflate(&self, second: bool, compressed: bool, data: Vec<u8>) -> Option<Vec<u8>> {
        if self.text() && !second && compressed { decompress(&data).map(|d| d.0) } else { Some(data) }
    }

    pub(crate) fn turn_quit(&mut self, buf: &mut VecDeque<u8>) {
        self.b2f.clear_fs();
        if self.b2f.take_quit() {
            self.disc_after_tx = true;
            buf.clear();
        }
    }

    pub(crate) fn wants_qrt(&self, queued: usize) -> bool {
        self.disc_now || self.disc_after_tx && queued + self.files.len() == 0
    }

    pub(crate) fn deliver(&mut self, data: &[u8], skip: usize, host: impl FnMut(Vec<u8>)) {
        let out = self.rx.slots.put(data, skip);
        if !out.is_empty() {
            self.emit(out, host);
        }
    }

    fn emit(&mut self, data: Vec<u8>, mut host: impl FnMut(Vec<u8>)) {
        if self.comp != Compression::Files {
            host(data);
            return;
        }
        for out in self.reader.push(data) {
            host(out);
        }
    }
}

pub type Margins = [f64; 5];

pub(crate) type HintRow<'a> = (u8, u8, &'a [(usize, u8)]);

pub(crate) fn queue_hint_by(
    l: usize,
    n: usize,
    rows: [HintRow; 2],
    max: u8,
    fits: impl Fn(usize, usize) -> bool,
) -> u8 {
    for ((down, same, up), k) in rows.into_iter().zip(1..) {
        if l > 1 && fits(l - 1, k) {
            return down;
        }
        if fits(l, k) {
            return same;
        }
        if let Some(&(_, h)) = up.iter().find(|&&(d, _)| l + d <= n && fits(l + d, k)) {
            return h;
        }
    }
    max
}

#[derive(Default, Debug)]
pub(crate) struct Reorder(VecDeque<Option<u8>>);

impl Reorder {
    pub(crate) fn put(&mut self, data: &[u8], skip: usize) -> Vec<u8> {
        let slots = &mut self.0;
        let (mut i, mut free, mut it) = (0, 0, data.iter().peekable());
        while it.peek().is_some() {
            if i == slots.len() {
                slots.push_back(None);
            }
            if slots[i].is_none() {
                if free >= skip {
                    slots[i] = it.next().copied();
                }
                free += 1;
            }
            i += 1;
        }
        let n = slots.iter().take_while(|s| s.is_some()).count();
        slots.drain(..n).flatten().collect()
    }

    pub(crate) fn clear(&mut self) {
        self.0.clear();
    }
}
