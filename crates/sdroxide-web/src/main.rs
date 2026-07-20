//! Browser client: the same `SdroxideApp` over a WebSocket
//! `RemoteController`, with audio through a JS AudioWorklet bridge.

#[cfg(target_arch = "wasm32")]
mod web {
    use eframe::wasm_bindgen::{self, JsCast, prelude::*};
    use sdroxide_proto::AudioCaps;
    use sdroxide_ui::{AudioBridge, RemoteController, SdroxideApp};

    // Implemented in assets/audio_bridge.js (loaded by index.html).
    #[wasm_bindgen(js_namespace = ["window", "sdroxideAudio"])]
    extern "C" {
        #[wasm_bindgen(js_name = pushPcm)]
        fn push_pcm(pcm: &[f32]);
        #[wasm_bindgen(js_name = pullMic)]
        fn pull_mic() -> Vec<f32>;
    }

    struct WebAudioBridge;

    impl AudioBridge for WebAudioBridge {
        fn caps(&self) -> AudioCaps {
            // PCM16 both ways for now; a WebCodecs Opus path can upgrade
            // this without protocol changes.
            AudioCaps { opus_decode: false, opus_encode: false }
        }
        fn play(&mut self, pcm: &[f32]) {
            push_pcm(pcm);
        }
        fn pull_mic(&mut self, out: &mut Vec<f32>) {
            out.extend(pull_mic());
        }
    }

    pub fn run() {
        console_error_panic_hook::set_once();

        wasm_bindgen_futures::spawn_local(async {
            let window = web_sys::window().expect("window");
            let document = window.document().expect("document");
            let canvas = document
                .get_element_by_id("sdroxide_canvas")
                .expect("canvas element")
                .dyn_into::<web_sys::HtmlCanvasElement>()
                .expect("canvas type");

            let location = window.location();
            let ws_proto = if location.protocol().as_deref() == Ok("https:") {
                "wss"
            } else {
                "ws"
            };
            let host = location.host().unwrap_or_else(|_| "localhost:4950".into());
            let url = format!("{ws_proto}://{host}/ws");

            let ctrl = RemoteController::connect(&url, Some(Box::new(WebAudioBridge)))
                .expect("websocket connect");

            eframe::WebRunner::new()
                .start(
                    canvas,
                    eframe::WebOptions::default(),
                    Box::new(move |cc| Ok(Box::new(SdroxideApp::new(cc, Box::new(ctrl))))),
                )
                .await
                .expect("eframe start");
        });
    }
}

fn main() {
    #[cfg(target_arch = "wasm32")]
    web::run();
}
