//! The Icom RC-28, whichever way this client reaches it.
//!
//! Native: `sdroxide_rc28`'s worker thread, over the platform's HID stack.
//! Browser: WebHID, through `rc28_bridge.js`, feeding the same report parser.
//! Either way the input runtime sees one [`Rc28Link`] producing
//! [`Rc28Event`]s, and the knob works against a radio on the far side of
//! `--connect` exactly as it does against one in this process.

use sdroxide_rc28::{Rc28Event, Rc28Status};

/// The link to the device, and what has been told to it.
pub(crate) struct Rc28Link {
    #[cfg(not(target_arch = "wasm32"))]
    handle: sdroxide_rc28::Rc28Handle,
    #[cfg(target_arch = "wasm32")]
    web: web::WebRc28,
    enabled: bool,
    /// LEDs last requested, so an unchanged state is not re-sent every frame.
    leds: Option<u8>,
}

impl Rc28Link {
    pub(crate) fn new(enabled: bool, ctx: &eframe::egui::Context) -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let ctx = ctx.clone();
            let handle = sdroxide_rc28::spawn(enabled, move || crate::repaint::animate(&ctx));
            Rc28Link { handle, enabled, leds: None }
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = ctx;
            let web = web::WebRc28::default();
            web.set_enabled(enabled);
            Rc28Link { web, enabled, leds: None }
        }
    }

    /// Follow the settings' on/off. Cheap to call every frame.
    pub(crate) fn set_enabled(&mut self, on: bool) {
        if on == self.enabled {
            return;
        }
        self.enabled = on;
        self.leds = None;
        #[cfg(not(target_arch = "wasm32"))]
        self.handle.set_enabled(on);
        #[cfg(target_arch = "wasm32")]
        self.web.set_enabled(on);
    }

    pub(crate) fn poll(&mut self) -> Vec<Rc28Event> {
        #[cfg(not(target_arch = "wasm32"))]
        let events = self.handle.poll();
        #[cfg(target_arch = "wasm32")]
        let events = self.web.poll();
        if events.iter().any(|e| matches!(e, Rc28Event::Connected(_))) {
            // A fresh device shows nothing until told.
            self.leds = None;
        }
        events
    }

    /// Throw away what has queued up, for a radio tab that is not focused.
    ///
    /// Native only: there every tab has a worker of its own, while the
    /// browser's bridge is one queue for the whole page — draining it here
    /// would take the focused tab's knob turns away from it.
    ///
    /// Either way the tab stops driving the LEDs, which are the focused tab's
    /// to show, and forgets what it last sent them — so that on being focused
    /// again it puts its own lights back rather than trusting a cache another
    /// tab has since made stale.
    pub(crate) fn discard(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = self.handle.poll();
            if self.leds.is_some() {
                self.handle.release_leds();
            }
        }
        self.leds = None;
    }

    /// Light exactly these of TRANSMIT, F-1 and F-2.
    pub(crate) fn set_leds(&mut self, lit: u8) {
        if self.leds == Some(lit) {
            return;
        }
        self.leds = Some(lit);
        #[cfg(not(target_arch = "wasm32"))]
        self.handle.set_leds(lit);
        #[cfg(target_arch = "wasm32")]
        self.web.set_leds(lit);
    }

    pub(crate) fn status(&self) -> Rc28Status {
        #[cfg(not(target_arch = "wasm32"))]
        return self.handle.status();
        #[cfg(target_arch = "wasm32")]
        return self.web.status();
    }

    /// Whether this client can reach an RC-28 at all, and if not, why not.
    pub(crate) fn unsupported_reason() -> Option<&'static str> {
        #[cfg(not(target_arch = "wasm32"))]
        return None;
        #[cfg(target_arch = "wasm32")]
        return web::unsupported_reason();
    }

    /// Browser only: open the browser's device chooser. Must be called from a
    /// click — WebHID refuses otherwise.
    pub(crate) fn request_device(&self) {
        #[cfg(target_arch = "wasm32")]
        self.web.request();
    }
}

#[cfg(target_arch = "wasm32")]
mod web {
    use sdroxide_rc28::proto::{self, Decoder, Report};
    use sdroxide_rc28::{Rc28Event, Rc28Status};
    use wasm_bindgen::JsCast;
    use wasm_bindgen::prelude::*;

    // Implemented in sdroxide-web's assets/rc28_bridge.js. Every call is
    // `catch`: a page served without the script (or with a stale copy of it)
    // must cost the RC-28, not the whole client.
    #[wasm_bindgen(js_namespace = ["window", "sdroxideRc28"])]
    extern "C" {
        #[wasm_bindgen(catch)]
        fn supported() -> Result<bool, JsValue>;
        #[wasm_bindgen(js_name = setEnabled, catch)]
        fn set_enabled_js(on: bool) -> Result<(), JsValue>;
        #[wasm_bindgen(catch)]
        fn request() -> Result<(), JsValue>;
        #[wasm_bindgen(catch)]
        fn drain() -> Result<js_sys::Array, JsValue>;
        #[wasm_bindgen(catch)]
        fn write(bytes: &[u8]) -> Result<(), JsValue>;
        #[wasm_bindgen(js_name = connectedName, catch)]
        fn connected_name() -> Result<String, JsValue>;
        #[wasm_bindgen(js_name = lastError, catch)]
        fn last_error() -> Result<String, JsValue>;
    }

    fn bridge_present() -> bool {
        web_sys::window()
            .and_then(|w| js_sys::Reflect::get(&w, &JsValue::from_str("sdroxideRc28")).ok())
            .is_some_and(|v| !v.is_undefined())
    }

    pub(super) fn unsupported_reason() -> Option<&'static str> {
        if !bridge_present() {
            return Some("This page was served without the RC-28 bridge — reload it.");
        }
        if !supported().unwrap_or(false) {
            return Some(
                "This browser cannot reach USB devices. The RC-28 works in Chrome, Edge or \
                 Opera, over https (or http://localhost).",
            );
        }
        None
    }

    #[derive(Default)]
    pub(super) struct WebRc28 {
        decoder: Decoder,
        /// The version string, from the reply this tab happened to drain.
        firmware: String,
    }

    impl WebRc28 {
        fn usable() -> bool {
            bridge_present() && supported().unwrap_or(false)
        }

        pub(super) fn set_enabled(&self, on: bool) {
            if Self::usable() {
                let _ = set_enabled_js(on);
            }
        }

        pub(super) fn request(&self) {
            if Self::usable() {
                let _ = request();
            }
        }

        pub(super) fn set_leds(&self, lit: u8) {
            if Self::usable() {
                let _ = write(&proto::led_report(lit | proto::LED_LINK));
            }
        }

        /// Asked of the bridge rather than kept here: the bridge is one per
        /// page, and the tab that drained "connected" may not be this one.
        pub(super) fn status(&self) -> Rc28Status {
            if !Self::usable() {
                return Rc28Status::default();
            }
            let name = connected_name().unwrap_or_default();
            let error = last_error().ok().filter(|e| !e.is_empty());
            Rc28Status {
                connected: !name.is_empty(),
                firmware: if name.is_empty() { String::new() } else { self.firmware.clone() },
                name,
                error,
            }
        }

        pub(super) fn poll(&mut self) -> Vec<Rc28Event> {
            let mut out = Vec::new();
            if !Self::usable() {
                return out;
            }
            let Ok(items) = drain() else { return out };
            for item in items.iter() {
                if let Some(text) = item.as_string() {
                    self.note(&text, &mut out);
                } else if let Ok(bytes) = item.dyn_into::<js_sys::Uint8Array>() {
                    match Report::parse(&bytes.to_vec()) {
                        Some(Report::Firmware(v)) => self.firmware = v,
                        Some(r) => {
                            out.extend(self.decoder.feed(&r).into_iter().map(Rc28Event::Input))
                        }
                        None => {}
                    }
                }
            }
            out
        }

        /// One of the bridge's string messages. Errors are left to
        /// [`Self::status`], which asks the bridge for the last one.
        fn note(&mut self, text: &str, out: &mut Vec<Rc28Event>) {
            if let Some(name) = text.strip_prefix("connected:") {
                self.decoder.reset();
                self.firmware.clear();
                // Ask for the version, and light LINK.
                let _ = write(&proto::firmware_request());
                let _ = write(&proto::led_report(proto::LED_LINK));
                out.push(Rc28Event::Connected(name.to_string()));
            } else if text == "disconnected" {
                self.decoder.reset();
                self.firmware.clear();
                out.push(Rc28Event::Disconnected);
            }
        }
    }
}
