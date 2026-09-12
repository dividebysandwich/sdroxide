//! A port of `channel_server.py::handle_client` — bridges TCP sockets onto
//! [`ChannelCore`]. Because the wire format matches `channel_server.py`
//! exactly, the Python `client.py` / `monitor.py` can connect to this server.

use std::sync::Arc;

use anyhow::{anyhow, bail};
use tokio::io::{BufReader, BufWriter};
use tokio::net::{TcpListener, TcpStream};

use super::core::ChannelCore;
use super::link::b64_to_samples;

pub async fn serve(listener: TcpListener, core: Arc<ChannelCore>) -> anyhow::Result<()> {
    loop {
        let (stream, peer) = listener.accept().await?;
        stream.set_nodelay(true).ok();
        let core = Arc::clone(&core);
        tokio::spawn(async move {
            if let Err(e) = handle_conn(stream, core).await {
                tracing::debug!("connection ended ({peer}): {e}");
            }
        });
    }
}

async fn handle_conn(stream: TcpStream, core: Arc<ChannelCore>) -> anyhow::Result<()> {
    let (r, w) = stream.into_split();
    let mut reader = BufReader::new(r);
    let mut writer = BufWriter::new(w);

    let hello = crate::netproto::read_json(&mut reader)
        .await?
        .ok_or_else(|| anyhow!("EOF before HELLO"))?;
    if hello.get("cmd").and_then(|v| v.as_str()) != Some("HELLO") {
        bail!("the first message is not HELLO");
    }
    let callsign = hello
        .get("callsign")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("no callsign in HELLO"))?
        .to_string();

    let (id, mut rx) = core.register(&callsign);

    // core -> socket
    let writer_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if crate::netproto::write_json(&mut writer, &msg).await.is_err() {
                break;
            }
        }
    });

    // socket -> core
    let pump: anyhow::Result<()> = async {
        loop {
            let Some(v) = crate::netproto::read_json(&mut reader).await? else {
                break;
            };
            if v.get("cmd").and_then(|c| c.as_str()) == Some("TRANSMIT_AUDIO")
                && let Some(b64) = v.get("audio_b64").and_then(|a| a.as_str())
            {
                let samples = b64_to_samples(b64)?;
                core.transmit(id, samples);
            }
        }
        Ok(())
    }
    .await;

    core.deregister(id);
    writer_task.abort();
    pump
}
