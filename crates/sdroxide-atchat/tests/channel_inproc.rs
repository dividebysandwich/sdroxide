//! The in-process channel: half-duplex access, delayed delivery, the monitor tap.

use std::time::Duration;

use sdroxide_atchat::channel::{ChannelConfig, ChannelCore};
use sdroxide_atchat::modem::{Mode, Modem};
use sdroxide_atchat::netproto::ServerMsg;
use tokio::sync::mpsc::Receiver;
use tokio::time::timeout;

async fn wait_rx_audio(rx: &mut Receiver<ServerMsg>) -> Vec<i16> {
    loop {
        match timeout(Duration::from_secs(5), rx.recv()).await {
            Ok(Some(ServerMsg::RxAudio { audio_b64 })) => {
                return sdroxide_atchat::channel::b64_to_samples(&audio_b64).unwrap();
            }
            Ok(Some(_)) => continue,
            other => panic!("while waiting for RX_AUDIO: {other:?}"),
        }
    }
}

#[tokio::test]
async fn half_duplex_grant_deny_and_delivery() {
    let core = ChannelCore::spawn(ChannelConfig::default());
    let (id_a, mut rx_a) = core.register("TA1ABC");
    let (id_b, mut rx_b) = core.register("TA2DEF");

    let modem = Modem::new();
    let payload = br#"{"type":"CHAT","src":"TA1ABC","dst":"ALL","text":"hi"}"#;
    let wave = modem.modulate(payload, Mode::Qpsk);

    core.transmit(id_a, wave.clone());

    // The sender gets TX_GRANTED right away.
    let g = timeout(Duration::from_secs(1), rx_a.recv()).await.unwrap().unwrap();
    assert!(matches!(g, ServerMsg::TxGranted { .. }), "expected TX_GRANTED, got {g:?}");

    // While the channel is busy the second station is refused.
    core.transmit(id_b, wave.clone());
    let d = timeout(Duration::from_secs(1), rx_b.recv()).await.unwrap().unwrap();
    assert!(matches!(d, ServerMsg::ChannelBusy { retry_after } if retry_after > 0.0));

    // At the end of the airtime both stations (the sender included) receive and decode the audio.
    let a_audio = wait_rx_audio(&mut rx_a).await;
    let b_audio = wait_rx_audio(&mut rx_b).await;
    assert_eq!(modem.demodulate(&a_audio).as_deref(), Some(&payload[..]));
    assert_eq!(modem.demodulate(&b_audio).as_deref(), Some(&payload[..]));
}

#[tokio::test]
async fn monitor_tap_streams_burst_then_silence() {
    let core = ChannelCore::spawn(ChannelConfig::default());
    let mut mon = core.subscribe_monitor();
    let (id_a, mut rx_a) = core.register("TA1ABC");

    let modem = Modem::new();
    let wave = modem.modulate(b"monitor tap test", Mode::Bpsk);
    core.transmit(id_a, wave);
    let _ = timeout(Duration::from_secs(1), rx_a.recv()).await;

    let mut peak = 0i32;
    let mut chunks = 0;
    let deadline = tokio::time::Instant::now() + Duration::from_millis(900);
    while tokio::time::Instant::now() < deadline {
        if let Ok(Ok(chunk)) = timeout(Duration::from_millis(120), mon.recv()).await {
            assert_eq!(chunk.len(), 160, "a 20 ms hop should be 160 samples");
            peak = peak.max(chunk.iter().map(|s| (*s as i32).abs()).max().unwrap_or(0));
            chunks += 1;
        }
    }
    assert!(chunks >= 10, "the monitor pacer is not flowing ({chunks} chunks)");
    assert!(peak > 500, "the monitor tap stayed silent (peak={peak})");
}

#[tokio::test]
async fn awgn_config_still_delivers_and_decodes() {
    let core = ChannelCore::spawn(ChannelConfig { snr_db: Some(20.0), ..Default::default() });
    let (id_a, mut rx_a) = core.register("TA1ABC");
    let (_id_b, mut rx_b) = core.register("TA2DEF");

    let modem = Modem::new();
    let payload = b"noisy but decodable";
    core.transmit(id_a, modem.modulate(payload, Mode::Bpsk));

    let _ = wait_rx_audio(&mut rx_a).await; // the sender's own echo
    let b_audio = wait_rx_audio(&mut rx_b).await;
    assert_eq!(modem.demodulate(&b_audio).as_deref(), Some(&payload[..]));
}
