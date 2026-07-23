//! Server mode: HTTP (static WASM client) + a single-client WebSocket
//! session speaking `sdroxide-proto`.
//!
//! Lanes into the socket (per the project plan):
//! - control/state/memories/meters: reliable queue, never intentionally dropped
//! - spectrum: latest-wins watch channel (slow socket → lower fps, no lag)
//! - audio: small bounded queue, drops blocks when the socket can't keep up
//!
//! A pump thread bridges the engine's sync endpoints (crossbeam events,
//! triple-buffered spectrum, rtrb audio) into those async lanes and caches
//! the latest state/caps/memories so a new session can be greeted instantly.

mod session;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::Router;
use axum::routing::get;
use tokio::sync::{mpsc, watch};
use tracing::{info, warn};

use sdroxide_proto::{AudioCodec, ServerMsg};
use sdroxide_types::{Command, DeviceCaps, MemoryChannel, RadioEvent, RadioState, SpectrumFrame};

#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    #[error("I/O: {0}")]
    Io(#[from] std::io::Error),
}

pub struct ServerParams {
    pub cmd_tx: crossbeam_channel::Sender<Command>,
    pub event_rx: crossbeam_channel::Receiver<RadioEvent>,
    pub spectrum_out: triple_buffer::Output<SpectrumFrame>,
    /// Interleaved stereo 48 kHz demod audio from the engine.
    pub audio_rx: rtrb::Consumer<f32>,
    /// Mono 48 kHz mic samples into the engine.
    pub mic_tx: rtrb::Producer<f32>,
    pub bind: String,
    pub port: u16,
    /// Directory with the built web client; `None` uses embedded assets
    /// (feature `embed-web`) or a plain info page.
    pub web_root: Option<PathBuf>,
}

pub(crate) struct SessionTx {
    pub reliable: mpsc::Sender<ServerMsg>,
    pub audio: mpsc::Sender<ServerMsg>,
    pub rx_codec: AudioCodec,
}

#[derive(Default, Clone)]
pub(crate) struct Latest {
    pub caps: DeviceCaps,
    pub state: RadioState,
    pub memories: Vec<MemoryChannel>,
}

pub(crate) struct Shared {
    pub cmd_tx: crossbeam_channel::Sender<Command>,
    pub latest: Mutex<Latest>,
    pub session: Mutex<Option<SessionTx>>,
    pub busy: AtomicBool,
    pub mic_tx: Mutex<rtrb::Producer<f32>>,
    pub spectrum_rx: watch::Receiver<Option<SpectrumFrame>>,
}

/// Build a tokio runtime and serve until the process exits.
pub fn run_blocking(params: ServerParams) -> Result<(), ServerError> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(serve(params))
}

pub async fn serve(params: ServerParams) -> Result<(), ServerError> {
    let (spectrum_watch, spectrum_rx) = watch::channel(None);
    let shared = Arc::new(Shared {
        cmd_tx: params.cmd_tx,
        latest: Mutex::new(Latest::default()),
        session: Mutex::new(None),
        busy: AtomicBool::new(false),
        mic_tx: Mutex::new(params.mic_tx),
        spectrum_rx,
    });

    {
        let shared = shared.clone();
        std::thread::Builder::new()
            .name("sdroxide-pump".into())
            .spawn(move || {
                pump(shared, params.event_rx, params.spectrum_out, params.audio_rx, spectrum_watch)
            })
            .expect("spawn pump thread");
    }

    let mut app = Router::new()
        .route("/ws", get(session::ws_route))
        .with_state(shared);
    app = add_static_routes(app, params.web_root);

    let addr: SocketAddr = format!("{}:{}", params.bind, params.port)
        .parse()
        .unwrap_or_else(|_| SocketAddr::from(([0, 0, 0, 0], params.port)));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("server listening on http://{addr}/");
    axum::serve(listener, app).await?;
    Ok(())
}

/// Bridge thread: engine endpoints → session lanes + latest-state cache.
fn pump(
    shared: Arc<Shared>,
    event_rx: crossbeam_channel::Receiver<RadioEvent>,
    mut spectrum_out: triple_buffer::Output<SpectrumFrame>,
    mut audio_rx: rtrb::Consumer<f32>,
    spectrum_watch: watch::Sender<Option<SpectrumFrame>>,
) {
    let mut mono = Vec::<f32>::new();
    let mut opus_enc: Option<opus::Encoder> = None;
    let mut audio_seq = 0u32;

    loop {
        // Events: block briefly so the loop runs ~200 Hz even when idle.
        match event_rx.recv_timeout(Duration::from_millis(5)) {
            Ok(ev) => {
                handle_event(&shared, ev);
                while let Ok(ev) = event_rx.try_recv() {
                    handle_event(&shared, ev);
                }
            }
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {}
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                warn!("engine gone; pump stopping");
                return;
            }
        }

        // Spectrum: latest wins.
        if spectrum_out.update() {
            let f = spectrum_out.output_buffer();
            if !f.bins.is_empty() {
                let _ = spectrum_watch.send_replace(Some(f.clone()));
            }
        }

        // Audio: drain stereo ring → mono → 20 ms frames → encode → lane.
        let pairs = audio_rx.slots() / 2;
        for _ in 0..pairs {
            let l = audio_rx.pop().unwrap_or(0.0);
            let r = audio_rx.pop().unwrap_or(0.0);
            mono.push(0.5 * (l + r));
        }

        let session_codec = shared.session.lock().unwrap().as_ref().map(|s| s.rx_codec);
        match session_codec {
            None => mono.clear(),
            Some(codec) => {
                while mono.len() >= 960 {
                    let payload = match codec {
                        AudioCodec::Opus48kMono => {
                            let enc = opus_enc.get_or_insert_with(|| {
                                opus::Encoder::new(
                                    48_000,
                                    opus::Channels::Mono,
                                    opus::Application::Audio,
                                )
                                .expect("opus encoder")
                            });
                            match enc.encode_vec_float(&mono[..960], 1400) {
                                Ok(v) => v,
                                Err(e) => {
                                    warn!("opus encode: {e}");
                                    mono.drain(..960);
                                    continue;
                                }
                            }
                        }
                        AudioCodec::Pcm16_48k => {
                            let mut v = Vec::with_capacity(1920);
                            for &s in &mono[..960] {
                                let i = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
                                v.extend_from_slice(&i.to_le_bytes());
                            }
                            v
                        }
                    };
                    audio_seq = audio_seq.wrapping_add(1);
                    if let Some(s) = shared.session.lock().unwrap().as_ref() {
                        // Bounded lane: dropping is the backpressure policy.
                        let _ = s.audio.try_send(ServerMsg::RxAudio { seq: audio_seq, payload });
                    }
                    mono.drain(..960);
                }
            }
        }
        // Bound the accumulator against pathological stalls.
        if mono.len() > 48_000 {
            let cut = mono.len() - 4_800;
            mono.drain(..cut);
        }
    }
}

fn handle_event(shared: &Shared, ev: RadioEvent) {
    let msg = {
        let mut latest = shared.latest.lock().unwrap();
        match ev {
            RadioEvent::Capabilities(c) => {
                latest.caps = c;
                None // delivered via HelloAck
            }
            RadioEvent::State(s) => {
                latest.state = s.clone();
                Some(ServerMsg::State(s))
            }
            RadioEvent::Memories(m) => {
                latest.memories = m.clone();
                Some(ServerMsg::Memories(m))
            }
            RadioEvent::Meters(m) => Some(ServerMsg::Meters(m)),
            RadioEvent::Spectrum(_) => None, // spectrum travels via the watch lane
            RadioEvent::ConnectionLost(e) => Some(ServerMsg::Error(e)),
            // A local radio-audio-device notice is meaningless to a remote
            // client (its audio lives on the server host), so don't forward it.
            RadioEvent::Notice(_) => None,
            RadioEvent::Ft8Decodes(d) => Some(ServerMsg::Ft8Decodes(d)),
            RadioEvent::Ft8Status(s) => Some(ServerMsg::Ft8Status(s)),
            RadioEvent::Ft8QsoLogged(r) => Some(ServerMsg::Ft8QsoLogged(r)),
            RadioEvent::SkimmerSpots(s) => Some(ServerMsg::SkimmerSpots(s)),
            RadioEvent::SstvLine { image_id, y, rgb } => {
                Some(ServerMsg::SstvLine { image_id, y, rgb })
            }
            RadioEvent::SstvImage { image_id, mode, w, h, png } => {
                Some(ServerMsg::SstvImage { image_id, mode, w, h, png })
            }
            RadioEvent::SstvStatus(s) => Some(ServerMsg::SstvStatus(s)),
            RadioEvent::DigiImage { png } => Some(ServerMsg::DigiImage { png }),
        }
    };
    if let Some(msg) = msg {
        if let Some(s) = shared.session.lock().unwrap().as_ref() {
            if s.reliable.try_send(msg).is_err() {
                warn!("reliable lane full; dropping message");
            }
        }
    }
}

fn add_static_routes(app: Router, web_root: Option<PathBuf>) -> Router {
    use axum::http::HeaderValue;
    use tower_http::set_header::SetResponseHeaderLayer;

    let app = match web_root {
        Some(dir) => {
            info!("serving web client from {}", dir.display());
            app.fallback_service(tower_http::services::ServeDir::new(dir))
        }
        None => add_embedded_or_placeholder(app),
    };
    // Cross-origin isolation: lets the client use SharedArrayBuffer if the
    // audio path is ever upgraded to it.
    app.layer(SetResponseHeaderLayer::if_not_present(
        axum::http::header::HeaderName::from_static("cross-origin-opener-policy"),
        HeaderValue::from_static("same-origin"),
    ))
    .layer(SetResponseHeaderLayer::if_not_present(
        axum::http::header::HeaderName::from_static("cross-origin-embedder-policy"),
        HeaderValue::from_static("require-corp"),
    ))
}

#[cfg(feature = "embed-web")]
fn add_embedded_or_placeholder(app: Router) -> Router {
    use axum::http::{StatusCode, Uri, header};
    use axum::response::IntoResponse;

    #[derive(rust_embed::RustEmbed)]
    #[folder = "$CARGO_MANIFEST_DIR/../sdroxide-web/dist"]
    struct WebAssets;

    async fn serve_embedded(uri: Uri) -> axum::response::Response {
        let path = uri.path().trim_start_matches('/');
        let path = if path.is_empty() { "index.html" } else { path };
        match WebAssets::get(path) {
            Some(f) => {
                let mime = mime_guess::from_path(path).first_or_octet_stream();
                ([(header::CONTENT_TYPE, mime.as_ref().to_string())], f.data.into_owned())
                    .into_response()
            }
            None => (StatusCode::NOT_FOUND, "not found").into_response(),
        }
    }
    app.fallback(serve_embedded)
}

#[cfg(not(feature = "embed-web"))]
fn add_embedded_or_placeholder(app: Router) -> Router {
    app.fallback(|| async {
        "sdroxide server is running. Build the web client with `trunk build` \
         in crates/sdroxide-web and pass --web-root, or rebuild the server \
         with the embed-web feature."
    })
}
