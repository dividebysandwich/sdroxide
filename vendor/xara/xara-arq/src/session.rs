use crate::host::HostEvent;
use crate::link::Link;
use crate::rng::XorShift;
use crate::time::Time;
use std::collections::VecDeque;

#[derive(Clone, Debug, PartialEq)]
pub enum Output<T, L> {
    Tx(T),
    Listen(L),
    Host(HostEvent),
}

pub(crate) struct Core<T, L> {
    pub(crate) out: VecDeque<Output<T, L>>,
    pub(crate) rng: XorShift,
    pub(crate) transmitting: bool,
    pub(crate) tx_end: Time,
}

impl<T, L> Core<T, L> {
    pub(crate) fn new(seed: u64) -> Self {
        Self { out: VecDeque::new(), rng: XorShift::new(seed), transmitting: false, tx_end: 0 }
    }

    pub(crate) fn pop(&mut self) -> Option<Output<T, L>> {
        self.out.pop_front()
    }

    pub(crate) fn host(&mut self, e: HostEvent) {
        self.out.push_back(Output::Host(e));
    }

    pub(crate) fn next_deadline(&self, link: &Link) -> Option<Time> {
        if self.transmitting {
            return None;
        }
        link.next_deadline()
    }

    pub(crate) fn deliver(&mut self, link: &mut Link, data: &[u8], skip: usize) {
        let out = &mut self.out;
        link.deliver(data, skip, |d| out.push_back(Output::Host(HostEvent::Data(d))));
    }
}
