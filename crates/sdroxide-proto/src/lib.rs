//! WebSocket wire protocol between the sdroxide server and remote clients.
//!
//! Framing: every binary WS message is `[PROTO_VERSION_BYTE][postcard bytes]`.
//! The version byte is a fast sanity check; the real version negotiation
//! happens in `Hello`/`HelloAck`.
//!
//! Compiles for native and `wasm32-unknown-unknown`.

use serde::{Deserialize, Serialize};

use sdroxide_types::{
    Command, Decode, DeviceCaps, DigiStatus, MemoryChannel, Meters, QsoRecord, RadioState,
    SkimmerSpot, SpectrumFrame,
};

/// Bump on any incompatible change to the message enums (this includes the
/// payload structs from `sdroxide-types` that ride the wire, e.g. `QsoRecord`).
/// v3: `QsoRecord` gained `id` + `comment` fields.
/// v4: added `ServerMsg::SkimmerSpots` + `Command::SetSkimmerEnabled` + a
/// `RadioState.skimmer_enabled` field.
pub const PROTO_VERSION: u16 = 4;
const VERSION_BYTE: u8 = 0x04;

#[derive(Debug, thiserror::Error)]
pub enum ProtoError {
    #[error("empty message")]
    Empty,
    #[error("unsupported protocol version byte {0:#x}")]
    Version(u8),
    #[error("decode error: {0}")]
    Decode(#[from] postcard::Error),
}

/// Audio codec for one stream direction, negotiated at Hello time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioCodec {
    /// 20 ms Opus frames, 48 kHz mono.
    Opus48kMono,
    /// Little-endian PCM16, 48 kHz mono (fallback when WebCodecs is missing).
    Pcm16_48k,
}

/// What the client can encode/decode (browser WebCodecs availability).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioCaps {
    pub opus_decode: bool,
    pub opus_encode: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ClientMsg {
    Hello { proto: u16, audio: AudioCaps },
    Command(Command),
    /// 20 ms mic frame in the codec negotiated at Hello.
    MicFrame { seq: u32, payload: Vec<u8> },
    Ping(u64),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ServerMsg {
    HelloAck {
        proto: u16,
        caps: DeviceCaps,
        state: RadioState,
        /// Codec of server→client RX audio.
        rx_codec: AudioCodec,
        /// Codec expected for client→server mic frames.
        tx_codec: AudioCodec,
    },
    State(RadioState),
    Spectrum(SpectrumFrame),
    Meters(Meters),
    Memories(Vec<MemoryChannel>),
    RxAudio { seq: u32, payload: Vec<u8> },
    Pong(u64),
    /// Another client already holds the (single) session.
    Busy,
    Error(String),
    // FT8/FT4 digital modes.
    Ft8Decodes(Vec<Decode>),
    Ft8Status(DigiStatus),
    Ft8QsoLogged(QsoRecord),
    // Skimmers (CW etc.).
    SkimmerSpots(Vec<SkimmerSpot>),
}

pub fn encode<T: Serialize>(msg: &T) -> Result<Vec<u8>, ProtoError> {
    Ok(postcard::to_extend(msg, vec![VERSION_BYTE])?)
}

pub fn decode<'a, T: Deserialize<'a>>(bytes: &'a [u8]) -> Result<T, ProtoError> {
    match bytes {
        [] => Err(ProtoError::Empty),
        [VERSION_BYTE, rest @ ..] => Ok(postcard::from_bytes(rest)?),
        [v, ..] => Err(ProtoError::Version(*v)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_client_and_server_msgs() {
        let msgs = [
            ClientMsg::Hello {
                proto: PROTO_VERSION,
                audio: AudioCaps { opus_decode: true, opus_encode: false },
            },
            ClientMsg::Command(Command::SetPtt(true)),
            ClientMsg::MicFrame { seq: 7, payload: vec![1, 2, 3] },
        ];
        for m in &msgs {
            let bytes = encode(m).unwrap();
            let back: ClientMsg = decode(&bytes).unwrap();
            assert_eq!(&back, m);
        }

        let m = ServerMsg::State(RadioState::default());
        let bytes = encode(&m).unwrap();
        let back: ServerMsg = decode(&bytes).unwrap();
        assert_eq!(back, m);
    }

    #[test]
    fn rejects_wrong_version_byte() {
        assert!(matches!(decode::<ClientMsg>(&[0x7f, 0, 0]), Err(ProtoError::Version(0x7f))));
        assert!(matches!(decode::<ClientMsg>(&[]), Err(ProtoError::Empty)));
    }
}
