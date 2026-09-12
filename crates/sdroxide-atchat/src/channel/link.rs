//! `Link` — a station's connection to the channel, with the **reader and
//! writer halves kept separate**. The `client.py` model: a single
//! `receive_loop` reads, `send_frame` (behind a `tx_lock`) writes — this split
//! maps onto that model exactly.
//!
//! Two implementations:
//!   - [`InProcConnector`]: connects directly to an in-process [`ChannelCore`]
//!     (every station in the GUI).
//!   - [`TcpConnector`]: line-JSON wire-compatible with `channel_server.py` /
//!     `atchat-channeld` (Python interop, a distributed setup).

// The `LinkTx`/`LinkRx` methods deliberately return `-> impl Future + Send + '_`:
// `Station` uses these futures on other tasks, so a generic `Send` bound is
// required, and `async fn` syntax cannot express it.
#![allow(clippy::manual_async_fn)]

use std::future::Future;
use std::sync::Arc;

use crate::netproto::{ClientMsg, ServerMsg};
use base64::Engine as _;
use tokio::io::{BufReader, BufWriter};
use tokio::net::TcpStream;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::mpsc;

use super::core::{ChannelCore, ClientId};

const BASE64: base64::engine::general_purpose::GeneralPurpose =
    base64::engine::general_purpose::STANDARD;

pub fn samples_to_b64(samples: &[i16]) -> String {
    let mut bytes = Vec::with_capacity(samples.len() * 2);
    for s in samples {
        bytes.extend_from_slice(&s.to_le_bytes());
    }
    BASE64.encode(&bytes)
}

pub fn b64_to_samples(b64: &str) -> anyhow::Result<Vec<i16>> {
    let bytes = BASE64.decode(b64.as_bytes())?;
    Ok(bytes.as_chunks::<2>().0.iter().map(|c| i16::from_le_bytes([c[0], c[1]])).collect())
}

// ------------------------------------------------------------------ //
// Traits
// ------------------------------------------------------------------ //

/// The client -> server direction (`send_json`).
pub trait LinkTx: Send + 'static {
    fn send(&mut self, msg: ClientMsg) -> impl Future<Output = anyhow::Result<()>> + Send + '_;
    /// Close the connection (the station's `/drop`). On InProc it removes the
    /// channel registration.
    fn close(&mut self) {}
}

/// The server -> client direction (`read_json`). `None` -> the connection dropped.
pub trait LinkRx: Send + 'static {
    fn recv(&mut self) -> impl Future<Output = Option<ServerMsg>> + Send + '_;
}

/// A connection factory that can produce a new (tx, rx) pair — for `/reconnect`.
pub trait Connector: Send + Sync + 'static {
    type Tx: LinkTx;
    type Rx: LinkRx;
    fn connect(&self) -> impl Future<Output = anyhow::Result<(Self::Tx, Self::Rx)>> + Send + '_;
}

// ------------------------------------------------------------------ //
// InProc
// ------------------------------------------------------------------ //

pub struct InProcTx {
    core: Arc<ChannelCore>,
    id: ClientId,
}

pub struct InProcRx {
    rx: mpsc::Receiver<ServerMsg>,
}

impl LinkTx for InProcTx {
    fn send(&mut self, msg: ClientMsg) -> impl Future<Output = anyhow::Result<()>> + Send + '_ {
        async move {
            match msg {
                ClientMsg::Hello { .. } => {} // done during connect()
                ClientMsg::TransmitAudio { audio_b64 } => {
                    let samples = b64_to_samples(&audio_b64)?;
                    self.core.transmit(self.id, samples);
                }
            }
            Ok(())
        }
    }

    fn close(&mut self) {
        self.core.deregister(self.id);
    }
}

impl LinkRx for InProcRx {
    fn recv(&mut self) -> impl Future<Output = Option<ServerMsg>> + Send + '_ {
        async move { self.rx.recv().await }
    }
}

/// Connects to an in-process [`ChannelCore`].
pub struct InProcConnector {
    pub core: Arc<ChannelCore>,
    pub callsign: String,
}

impl InProcConnector {
    pub fn new(core: Arc<ChannelCore>, callsign: impl Into<String>) -> Self {
        Self { core, callsign: callsign.into() }
    }
}

impl Connector for InProcConnector {
    type Tx = InProcTx;
    type Rx = InProcRx;

    fn connect(&self) -> impl Future<Output = anyhow::Result<(Self::Tx, Self::Rx)>> + Send + '_ {
        async move {
            let (id, rx) = self.core.register(&self.callsign);
            Ok((InProcTx { core: Arc::clone(&self.core), id }, InProcRx { rx }))
        }
    }
}

// ------------------------------------------------------------------ //
// TCP
// ------------------------------------------------------------------ //

pub struct TcpTx {
    writer: BufWriter<OwnedWriteHalf>,
}

pub struct TcpRx {
    reader: BufReader<OwnedReadHalf>,
}

impl LinkTx for TcpTx {
    fn send(&mut self, msg: ClientMsg) -> impl Future<Output = anyhow::Result<()>> + Send + '_ {
        async move {
            crate::netproto::write_json(&mut self.writer, &msg).await?;
            Ok(())
        }
    }
}

impl LinkRx for TcpRx {
    fn recv(&mut self) -> impl Future<Output = Option<ServerMsg>> + Send + '_ {
        async move {
            loop {
                match crate::netproto::read_json(&mut self.reader).await {
                    Ok(Some(v)) => {
                        if let Ok(m) = serde_json::from_value::<ServerMsg>(v) {
                            return Some(m);
                        }
                        // Unknown server message -> ignore it, keep reading.
                    }
                    _ => return None,
                }
            }
        }
    }
}

/// Connects to `atchat-channeld` / `channel_server.py` over TCP.
pub struct TcpConnector {
    pub addr: String,
    pub callsign: String,
}

impl TcpConnector {
    pub fn new(addr: impl Into<String>, callsign: impl Into<String>) -> Self {
        Self { addr: addr.into(), callsign: callsign.into() }
    }

    /// A one-shot connection (for tests that do not need a factory).
    pub async fn connect_once(addr: &str, callsign: &str) -> anyhow::Result<(TcpTx, TcpRx)> {
        let c = TcpConnector::new(addr, callsign);
        c.connect().await
    }
}

impl Connector for TcpConnector {
    type Tx = TcpTx;
    type Rx = TcpRx;

    fn connect(&self) -> impl Future<Output = anyhow::Result<(Self::Tx, Self::Rx)>> + Send + '_ {
        async move {
            let stream = TcpStream::connect(&self.addr).await?;
            stream.set_nodelay(true).ok();
            let (r, w) = stream.into_split();
            let mut writer = BufWriter::new(w);
            crate::netproto::write_json(
                &mut writer,
                &ClientMsg::Hello { callsign: self.callsign.clone() },
            )
            .await?;
            Ok((TcpTx { writer }, TcpRx { reader: BufReader::new(r) }))
        }
    }
}
