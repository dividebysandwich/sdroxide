//! What a TNC modem offers the controller that clocks it.

/// Which modem is behind a [`TncModem`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TncKind {
    Mercury,
    Data2g,
}

impl TncKind {
    pub fn label(self) -> &'static str {
        match self {
            TncKind::Mercury => "Mercury",
            TncKind::Data2g => "Data2G",
        }
    }
}

/// One modem, clocked by audio at [`crate::RATE`].
///
/// The controller owns the clock: while unkeyed it hands every received block
/// to [`process`](Self::process); while the engine is transmitting it calls
/// `process` with silence for exactly as many samples as the transmitter asked
/// for. The modem answers with as many transmit samples as it was given input,
/// silence while it has nothing to send.
///
/// Commands and data come from whoever holds the [`crate::HostMux`]; replies
/// and received payload go back to them through [`take_output`](Self::take_output).
pub trait TncModem: Send {
    fn kind(&self) -> TncKind;

    /// Run `rx.len()` samples through the modem; `tx` receives as many.
    fn process(&mut self, rx: &[f32], tx: &mut [f32]);

    /// Whether the modem wants the transmitter keyed.
    fn keyed(&self) -> bool;

    /// One command line from the host, without the CR.
    fn command(&mut self, line: &str);

    /// Payload from the host, for the link.
    fn data(&mut self, bytes: &[u8]);

    /// Reply lines (without the CR) and received payload for the host, since
    /// the last call.
    fn take_output(&mut self) -> (Vec<String>, Vec<u8>);

    /// The host went away (its command connection closed, or an in-process
    /// client was dropped). A link it owned goes with it.
    fn client_gone(&mut self);

    /// The operator's stop: any session under way is dropped at once.
    fn abort(&mut self);

    /// Key a steady tone for tuning. `false` when the modem cannot (none, or
    /// not now).
    fn tune(&mut self, _on: bool) -> bool {
        false
    }

    /// The radio's dial frequency, Hz, for a modem that reports it.
    fn set_dial_hz(&mut self, _hz: u64) {}

    /// A busy channel, as the modem's own detector sees it.
    fn busy(&self) -> bool {
        false
    }

    /// The peak amplitude of the transmit audio, so the engine can scale an
    /// over to full drive.
    fn tx_peak(&self) -> f32;

    /// The modem's version, for the panel.
    fn version(&self) -> String;
}
