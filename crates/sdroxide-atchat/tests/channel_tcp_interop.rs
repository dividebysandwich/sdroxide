//! The TCP wire layer: `TcpConnector` <-> `tcp_server::serve` end to end.
//! (Python `client.py` <-> Rust server interop is checked by hand: `rust/README.md`.)

use std::time::Duration;

use sdroxide_atchat::channel::{ChannelConfig, ChannelCore, LinkRx, LinkTx, TcpConnector};
use sdroxide_atchat::modem::{Mode, Modem};
use sdroxide_atchat::netproto::{ClientMsg, ServerMsg};
use tokio::net::TcpListener;
use tokio::time::timeout;

#[tokio::test]
async fn tcp_link_transmit_reaches_peer_and_decodes() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap().to_string();
    let core = ChannelCore::spawn(ChannelConfig::default());
    tokio::spawn(sdroxide_atchat::channel::tcp_server::serve(listener, core));

    let (mut a_tx, mut _a_rx) = TcpConnector::connect_once(&addr, "TA1ABC").await.unwrap();
    let (mut _b_tx, mut b_rx) = TcpConnector::connect_once(&addr, "TA2DEF").await.unwrap();
    tokio::time::sleep(Duration::from_millis(80)).await;

    let modem = Modem::new();
    let payload = b"real OFDM over tcp";
    let wave = modem.modulate(payload, Mode::Qpsk);
    a_tx.send(ClientMsg::TransmitAudio {
        audio_b64: sdroxide_atchat::channel::samples_to_b64(&wave),
    })
    .await
    .unwrap();

    let audio_b64 = loop {
        match timeout(Duration::from_secs(5), b_rx.recv()).await.unwrap() {
            Some(ServerMsg::RxAudio { audio_b64 }) => break audio_b64,
            Some(_) => continue,
            None => panic!("the connection dropped"),
        }
    };
    let samples = sdroxide_atchat::channel::b64_to_samples(&audio_b64).unwrap();
    assert_eq!(modem.demodulate(&samples).as_deref(), Some(&payload[..]));
}

#[tokio::test]
async fn tcp_second_transmitter_gets_channel_busy() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap().to_string();
    let core = ChannelCore::spawn(ChannelConfig::default());
    tokio::spawn(sdroxide_atchat::channel::tcp_server::serve(listener, core));

    let (mut a_tx, mut a_rx) = TcpConnector::connect_once(&addr, "TA1ABC").await.unwrap();
    let (mut b_tx, mut b_rx) = TcpConnector::connect_once(&addr, "TA2DEF").await.unwrap();
    tokio::time::sleep(Duration::from_millis(80)).await;

    let modem = Modem::new();
    let big: Vec<u8> = (0..1500).map(|i| (i * 7) as u8).collect();
    let wave = modem.modulate(&big, Mode::Qpsk);
    a_tx.send(ClientMsg::TransmitAudio {
        audio_b64: sdroxide_atchat::channel::samples_to_b64(&wave),
    })
    .await
    .unwrap();
    match timeout(Duration::from_secs(2), a_rx.recv()).await.unwrap() {
        Some(ServerMsg::TxGranted { .. }) => {}
        other => panic!("while waiting for TX_GRANTED {other:?}"),
    }

    b_tx.send(ClientMsg::TransmitAudio {
        audio_b64: sdroxide_atchat::channel::samples_to_b64(&modem.modulate(b"small", Mode::Bpsk)),
    })
    .await
    .unwrap();
    match timeout(Duration::from_secs(2), b_rx.recv()).await.unwrap() {
        Some(ServerMsg::ChannelBusy { retry_after }) => assert!(retry_after > 0.0),
        other => panic!("while waiting for CHANNEL_BUSY {other:?}"),
    }
}
