//! Browser client: the same `SdroxideApp` over a WebSocket
//! `RemoteController`, with audio through a JS AudioWorklet bridge.

/// The directory this page was served from, as a path ending in `/`.
///
/// Every address the client builds hangs off this rather than off the root,
/// because the page need not be at the root: behind a reverse proxy it is
/// commonly under a prefix — `https://host/sdroxide/` on a Tailscale
/// certificate, where subdomains are not on offer (issue #241) — and a socket
/// opened at `wss://host/ws` would then be asking the proxy for something it
/// has never heard of.
///
/// A path ending in `/` is already a directory. Anything else has its last
/// segment dropped, which is a file name: `/sdroxide/index.html` was served
/// from `/sdroxide/`. A path with no `/` at all — which a browser does not
/// produce — reads as the root.
///
/// Not gated on wasm like the rest of this file, so the rule can be checked on
/// the machine it is built on rather than only in a browser.
#[cfg_attr(not(test), allow(dead_code))]
fn page_base(pathname: &str) -> String {
    match pathname.rfind('/') {
        Some(i) => pathname[..=i].to_string(),
        None => "/".to_string(),
    }
}

#[cfg(target_arch = "wasm32")]
mod web {
    use eframe::wasm_bindgen::{self, JsCast, prelude::*};
    use sdroxide_proto::AudioCaps;
    use sdroxide_ui::{AudioBridge, MultiApp, RadioTab, RemoteController, SolarApp};

    // Implemented in assets/audio_bridge.js (loaded by index.html).
    #[wasm_bindgen(js_namespace = ["window", "sdroxideAudio"])]
    extern "C" {
        #[wasm_bindgen(js_name = pushPcm)]
        fn push_pcm(pcm: &[f32]);
        #[wasm_bindgen(js_name = pullMic)]
        fn pull_mic() -> Vec<f32>;
        // `catch`, because this one is newer than the rest of the bridge. A
        // browser holding a cached `audio_bridge.js` from before it existed
        // would otherwise throw a TypeError straight through wasm on the first
        // frame, killing the whole UI over an optional feature.
        #[wasm_bindgen(js_name = setMicActive, catch)]
        fn set_mic_active(active: bool) -> Result<(), JsValue>;
    }

    #[derive(Default)]
    struct WebAudioBridge {
        /// Last value handed to JS. `pump_mic` calls every frame; the bridge
        /// wants the edge, not the level.
        mic_active: bool,
        /// Set once the JS side has been found to be missing `setMicActive`,
        /// so a stale bridge costs one warning rather than one per frame.
        mic_active_unsupported: bool,
    }

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
        fn set_mic_active(&mut self, active: bool) {
            if active == self.mic_active || self.mic_active_unsupported {
                return;
            }
            self.mic_active = active;
            if let Err(e) = set_mic_active(active) {
                self.mic_active_unsupported = true;
                web_sys::console::warn_2(
                    &"sdroxide audio: setMicActive unavailable (stale audio_bridge.js?); \
                      microphone transmit will not work until the page is reloaded"
                        .into(),
                    &e,
                );
            }
        }
    }

    /// Which wgpu backend to ask the browser for.
    ///
    /// The default is wgpu's own: WebGPU where the browser has it, WebGL2
    /// otherwise. `?gfx=webgl` pins it to WebGL2 and `?gfx=webgpu` to WebGPU.
    ///
    /// The escape hatch exists because the solar view is by far this app's
    /// heaviest graphics consumer — depth, MSAA, a few dozen draws a frame —
    /// and browser WebGPU implementations are not uniformly ready for that.
    /// Firefox on Linux in particular has been seen to abort the *whole browser
    /// process* with this page open, which is a fault no amount of care on this
    /// side can prevent. WebGL2 draws the same scene; it loses MSAA and some
    /// depth precision, and that is the whole difference.
    fn web_options(search: &str) -> eframe::WebOptions {
        use sdroxide_ui::egui_wgpu::{self, wgpu};

        // Adapter-reported limits either way — see `sdroxide_ui::wgpu_options`.
        let mut options = sdroxide_ui::wgpu_options();
        let backends = if search.contains("gfx=webgl") {
            wgpu::Backends::GL
        } else if search.contains("gfx=webgpu") {
            wgpu::Backends::BROWSER_WEBGPU
        } else {
            return eframe::WebOptions { wgpu_options: options, ..Default::default() };
        };

        if let egui_wgpu::WgpuSetup::CreateNew(setup) = &mut options.wgpu_setup {
            setup.instance_descriptor.backends = backends;
        }
        eframe::WebOptions { wgpu_options: options, ..Default::default() }
    }

    /// The radio id in `?radio=N`, if the query string names one. Anything
    /// that is not a number is ignored rather than guessed at: the fallback is
    /// the station's first radio, which is always there.
    fn radio_query(search: &str) -> Option<u32> {
        search
            .trim_start_matches('?')
            .split('&')
            .find_map(|p| p.strip_prefix("radio="))
            .and_then(|v| v.parse::<u32>().ok())
    }

    /// One radio of the station, as a tab: its own socket and its own audio
    /// bridge, exactly as a native client gives each radio its own sound
    /// stream.
    fn radio_tab(url: &str, id: u32, ctx: &eframe::egui::Context) -> Result<RadioTab, String> {
        let ctx = ctx.clone();
        // Deadline hint, not an immediate repaint — see the native remote
        // client for rationale.
        let ctrl =
            RemoteController::connect(url, Some(Box::new(WebAudioBridge::default())), move || {
                sdroxide_ui::repaint::after_ms(&ctx, 33)
            })?;
        // Deliberately unnamed: nobody typed an address here — the page came
        // from this station — so the shell fills the name in from what the
        // station calls the radio, as it does for the rest of the roster.
        Ok(RadioTab { id, name: String::new(), enabled: true, ctrl: Box::new(ctrl) })
    }

    /// The dialler the shell uses for the station's further radios.
    fn radio_tab_factory()
    -> impl FnMut(&str, u32, &eframe::egui::Context) -> Result<RadioTab, String> {
        |url: &str, id: u32, ctx: &eframe::egui::Context| radio_tab(url, id, ctx)
    }

    /// Ask the browser to keep this origin's storage rather than treat it as
    /// evictable.
    ///
    /// **This is what "the phone slept and everything came back default" is.**
    /// The client keeps its whole screen in the browser — theme, layout, chip
    /// choices, the reception log — and a browser may clear "best-effort"
    /// storage whenever it wants the room. The eviction is silent: the next
    /// load simply finds nothing, so the operator sees the defaults and
    /// reasonably reads that as the program having forgotten them. It has been
    /// reported twice on the phone, as the bindings and then as the theme.
    ///
    /// `persist()` asks for the bucket that is *not* cleared. It can be
    /// refused — Chrome grants it on engagement, Safari not at all unless the
    /// page is installed — which is why nothing depends on the answer, and why
    /// a profile can also hold the screen on the server. The promise is
    /// deliberately dropped: the request is the whole of what this does.
    fn keep_storage() {
        let Some(window) = web_sys::window() else { return };
        let _ = window.navigator().storage().persist();
    }

    pub fn run() {
        console_error_panic_hook::set_once();
        keep_storage();

        wasm_bindgen_futures::spawn_local(async {
            let window = web_sys::window().expect("window");
            let document = window.document().expect("document");
            let canvas = document
                .get_element_by_id("sdroxide_canvas")
                .expect("canvas element")
                .dyn_into::<web_sys::HtmlCanvasElement>()
                .expect("canvas type");

            // One wasm binary serves both pages, picked by the query string.
            // A second Trunk target would duplicate egui, eframe and wgpu — by
            // far the largest part of the bundle — in a second download, to
            // separate two views that share nearly all of their code.
            //
            // `?view=solar` also needs no server route of its own: it resolves
            // to the same index.html through the existing static fallback,
            // which is what lets the ☀ 3D chip open a plain relative URL.
            let location = window.location();
            let search = location.search().unwrap_or_default();
            let solar = search.contains("view=solar");
            // `?trace=1` numbers every settings row's y in the browser, and
            // opens the window it is for — see `sdroxide_ui::chrome::trace_on`.
            if search.contains("trace=1") {
                sdroxide_ui::chrome::set_trace(true);
            }
            // `?tab=radio` opens that settings page, so a reported page can be
            // measured without clicking through to it.
            if let Some(tab) =
                search.trim_start_matches('?').split('&').find_map(|p| p.strip_prefix("tab="))
            {
                sdroxide_ui::set_open_settings_tab(tab);
            }
            let options = web_options(&search);

            let ws_proto =
                if location.protocol().as_deref() == Ok("https:") { "wss" } else { "ws" };
            let host = location.host().unwrap_or_else(|_| "localhost:4950".into());
            // Where this page came from, not the root — see `page_base`.
            let base = crate::page_base(&location.pathname().unwrap_or_else(|_| "/".into()));

            let runner = eframe::WebRunner::new();
            let result = if solar {
                // The map is a viewer: no audio bridge, so this tab never asks
                // for the microphone, and its own endpoint, so it does not take
                // the control slot the main tab holds.
                document.set_title(&format!("{} — solar system", sdroxide_version::FLAVOR));
                let url = format!("{ws_proto}://{host}{base}solar-ws");
                runner
                    .start(
                        canvas,
                        options,
                        Box::new(move |cc| {
                            Ok(Box::new(
                                SolarApp::new(cc, &url)
                                    .map_err(|e| format!("solar websocket connect: {e}"))?,
                            ))
                        }),
                    )
                    .await
            } else {
                // `?radio=N` opens one of a station's radios; without it this
                // page opens the first. Either way the station says what else
                // it has and the shell brings the rest up as tabs, so a
                // browser holds a whole station exactly as the desktop client
                // does. (The list is also at `radios` beside this page, for
                // anyone looking from outside the app.)
                let first = radio_query(&search);
                let url = match first {
                    Some(id) => format!("{ws_proto}://{host}{base}ws/{id}"),
                    None => format!("{ws_proto}://{host}{base}ws"),
                };
                runner
                    .start(
                        canvas,
                        options,
                        // Connect inside the creator so the socket can wake the UI
                        // (repaint) the moment a message arrives.
                        Box::new(move |cc| {
                            let tab = radio_tab(&url, first.unwrap_or(0), &cc.egui_ctx)
                                .map_err(|e| format!("websocket connect: {e}"))?;
                            // No radio factory: a browser has no hardware of
                            // its own to open. The remote one is what the
                            // station's further radios arrive through.
                            Ok(Box::new(MultiApp::new(
                                cc,
                                vec![tab],
                                None,
                                Some(Box::new(radio_tab_factory())),
                            )))
                        }),
                    )
                    .await
            };
            result.expect("eframe start");
        });
    }
}

fn main() {
    #[cfg(target_arch = "wasm32")]
    web::run();
}

#[cfg(test)]
mod tests {
    use super::page_base;

    /// Every address this client builds hangs off the directory the page came
    /// from, so the whole client works under a reverse proxy's path prefix
    /// (issue #241).
    #[test]
    fn the_base_is_the_directory_the_page_came_from() {
        assert_eq!(page_base("/"), "/");
        assert_eq!(page_base("/index.html"), "/");
        assert_eq!(page_base("/sdroxide/"), "/sdroxide/");
        assert_eq!(page_base("/sdroxide/index.html"), "/sdroxide/");
        assert_eq!(page_base("/shack/sdr/index.html"), "/shack/sdr/");
        // A prefix with no trailing slash is a *file* as far as the browser is
        // concerned, and it resolves relative URLs beside it — so this has to
        // agree with what the page's own `<script src>` tags will do.
        assert_eq!(page_base("/sdroxide"), "/");
        // Not a shape a browser produces, but the answer is still an address.
        assert_eq!(page_base(""), "/");
    }
}
