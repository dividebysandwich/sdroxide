//! Native GUI mode: one engine thread + audio output per radio, one eframe
//! window with a tab per radio.

use anyhow::Result;
use eframe::egui;
use sdroxide_config::Settings;
use sdroxide_radio::{
    AudioParams, EngineConfig, IqSource, RadeWatch, StoreSync, TrSwitch, TxGate, start_engine,
};
use std::sync::Arc;
use tracing::warn;

use crate::RadioBoot;
use crate::local_controller::LocalController;

/// Engine + audio + controller for one radio. Every radio gets its own cpal
/// output stream — the operating system mixes them — and its own microphone
/// capture when it can transmit; the [`TxGate`] is what keeps two of them from
/// being on the air at once.
fn build_controller(
    boot: RadioBoot,
    settings: &Settings,
    tx_ham_only: bool,
    primary: bool,
    gate: &Arc<TxGate>,
    sync: &Arc<StoreSync>,
    rade: &Arc<RadeWatch>,
    tr: &Arc<TrSwitch>,
) -> LocalController {
    let (audio_out, audio_params) = match sdroxide_audio::start_output(
        settings.audio_output.as_deref(),
        48_000,
        settings.audio_routing,
    ) {
        Ok((out, producer)) => {
            let rate = out.sample_rate;
            (Some(out), Some(AudioParams { producer, out_rate: rate }))
        }
        Err(e) => {
            warn!("no audio output ({e}); running silent");
            (None, None)
        }
    };
    // Opening the microphone is a blocking call into the sound stack, and that
    // stack can sit on it for half a minute at a time — the system's default
    // source being a monitor of a card this radio has already claimed is enough
    // to do it. So it happens on a thread of its own and the controller hands it
    // to the engine when it lands; the radio does not wait to come up.
    let pending_mic = boot
        .caps
        .is_transmit_capable()
        .then(|| sdroxide_audio::start_input_background(settings.audio_input.clone(), 48_000));

    let cfg = EngineConfig {
        audio: audio_params,
        mic: None,
        cal_offset_db: settings.cal_offset_db as f32,
        initial_mode: boot.initial_mode,
        initial_antenna: boot.initial_antenna,
        tx_ham_only,
        cb_tx_allowed: settings.cb_tx_allowed,
        swr_guard: settings.swr_guard,
        swr_limit: settings.swr_limit,
        reopen: boot.reopen,
        // This engine is one of the operator's radios: remember where they
        // leave it, in its own scope.
        remember_session: true,
        store: boot.store.clone(),
        instance: boot.id,
        primary,
        tx_gate: Some(gate.clone()),
        store_sync: Some(sync.clone()),
        rade_watch: Some(rade.clone()),
        tr_switch: Some(tr.clone()),
        record_iq: boot.record_iq.clone(),
    };
    let handles = start_engine(boot.source, boot.caps, cfg);
    LocalController::new(
        handles,
        audio_out,
        pending_mic,
        settings.audio_output.clone(),
        settings.audio_input.clone(),
        settings.audio_routing,
        boot.store,
    )
}

pub fn run_multi(
    radios: Vec<RadioBoot>,
    settings: &Settings,
    // Whether the engines refuse to key outside the amateur bands. Resolved in
    // `main` from `config.toml` and the `--oob-tx` flag.
    tx_ham_only: bool,
    // A sanitized command line (radio-0 overrides stripped) for radios created
    // at runtime from the tab strip.
    factory_cli: crate::Cli,
    // Another sdroxide found on this machine at start-up, worded for the
    // banner — see `crate::other_instance`.
    other_instance: Option<String>,
) -> Result<()> {
    let gate = Arc::new(TxGate::new());
    let sync = Arc::new(StoreSync::new());
    let rade = Arc::new(RadeWatch::new());
    // The external T/R switch is one box in the antenna line, whichever radio
    // is keying — so it is shared like the interlock above rather than opened
    // once per radio.
    let tr = Arc::new(TrSwitch::new());

    let mut tabs = Vec::new();
    for (i, boot) in radios.into_iter().enumerate() {
        let id = boot.id;
        let name = boot.name.clone();
        let enabled = boot.enabled;
        let ctrl = build_controller(boot, settings, tx_ham_only, i == 0, &gate, &sync, &rade, &tr);
        tabs.push(sdroxide_ui::RadioTab { id, name, enabled, ctrl: Box::new(ctrl) });
    }

    // The "+" tab: create the scope on disk, then bring the radio up on the
    // stand-in source — a new radio has no interface yet (`Backend::None`), so
    // the open fails by design and the tab starts at Settings → Radio.
    let factory_gate = gate.clone();
    let factory_sync = sync.clone();
    let factory_rade = rade.clone();
    let factory_tr = tr.clone();
    let factory: sdroxide_ui::RadioFactory = Box::new(move || {
        let slot = sdroxide_config::create_radio("").map_err(|e| e.to_string())?;
        let settings = Settings::load();
        let store = sdroxide_config::Store::radio(slot.id);
        let mut c = factory_cli.clone();
        let initial_mode = c.apply_session(store.load_session());
        let (source, caps) =
            match crate::open_converted_source(&store.load_radio_config(), &c, &settings) {
                Ok(pair) => pair,
                Err(e) => (
                    Box::new(crate::null_source::NullSource::new(c.center_hz(), format!("{e}")))
                        as Box<dyn IqSource>,
                    crate::synthetic_caps("No radio"),
                ),
            };
        let boot = RadioBoot {
            id: slot.id,
            name: slot.name.clone(),
            // Freshly created, so switched on: it has no interface yet, which
            // is a different thing from having one and being told not to open
            // it.
            enabled: true,
            source,
            caps,
            initial_mode,
            initial_antenna: (None, None),
            reopen: Some(crate::reopen_factory_for(&c, store.clone(), slot.id)),
            store,
            // A radio added at runtime is never the capture target: `--record-iq`
            // names one file, and two bands interleaved into it are neither.
            record_iq: None,
        };
        let ctrl = build_controller(
            boot,
            &settings,
            tx_ham_only,
            false,
            &factory_gate,
            &factory_sync,
            &factory_rade,
            &factory_tr,
        );
        Ok(sdroxide_ui::RadioTab {
            id: slot.id,
            name: slot.name,
            enabled: true,
            ctrl: Box::new(ctrl),
        })
    });

    // Ahead of `wgpu_options`, which reads the note this leaves behind.
    sdroxide_ui::install_renderer_panic_note();
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        // Limits taken from the adapter rather than eframe's WebGPU baseline,
        // so a GPU that grants less than the baseline still opens a window.
        wgpu_options: sdroxide_ui::wgpu_options(),
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([800.0, 500.0])
            // Matches StartupWMClass in packaging/linux/sdroxide.desktop, so
            // the window groups under the menu entry in taskbars and docks.
            .with_app_id("sdroxide")
            .with_icon(sdroxide_ui::app_icon())
            // Program, version, then what it is connected to — so a screenshot
            // says which build it came from (#664), and says *Brown* rather
            // than the name of the program upstream calls it.
            .with_title(sdroxide_ui::window_title("")),
        // On a first start there is no saved geometry, and where the window
        // manager puts a window then is its own business — Windows cascades
        // them, which on a screen barely wider than the window pushes the
        // right-hand edge, and the Setup gear on it, off the display. Centred,
        // the window is on the screen from the first frame;
        // `MultiApp::fit_window` then deals with one that is bigger than the
        // screen altogether (issue #234). Dropped once a position has been
        // remembered — eframe applies this *after* restoring one, so
        // `sdroxide_ui::event_loop::run` is what makes "first start" mean it
        // (issue #256).
        centered: true,
        ..Default::default()
    };
    // Engine threads are joined by each controller's `shutdown()` — on tab
    // close, and for the rest in `SdroxideApp::on_exit` — so every device
    // closes before process teardown can race the C libraries' exit handlers.
    sdroxide_ui::event_loop::run(
        "sdroxide",
        options,
        Box::new(move |cc| {
            let mut multi =
                sdroxide_ui::MultiApp::new(cc, tabs, Some(factory), Some(remote_factory()));
            if let Some(w) = other_instance {
                multi.warn_other_instance(w);
            }
            Ok(Box::new(multi))
        }),
    )
    .map_err(|e| anyhow::anyhow!("eframe: {e}"))
}

/// This machine's sound devices, hung off a connection to somebody else's
/// station: their audio comes out of these speakers, and this microphone is
/// what goes on their air.
struct CpalBridge {
    _out: Option<sdroxide_audio::AudioOutput>,
    out: Option<sdroxide_radio::rtrb::Producer<f32>>,
    _mic: Option<sdroxide_audio::AudioInput>,
    mic: Option<sdroxide_radio::rtrb::Consumer<f32>>,
    /// The microphone open, while it is still running — same reasoning as the
    /// local radios': it blocks for as long as the sound stack wants, and this
    /// is the UI thread.
    pending_mic: Option<sdroxide_audio::PendingInput>,
    mic_resampler: Option<sdroxide_dsp::MonoResampler>,
    raw: Vec<f32>,
    /// Selected device names; `None` = system default.
    out_name: Option<String>,
    in_name: Option<String>,
    /// Which speaker channel(s) the received audio reaches (#654).
    routing: sdroxide_types::AudioRouting,
}

impl CpalBridge {
    /// Open the devices this machine's settings name. Every connection gets a
    /// bridge of its own — two servers on screen are two streams the operating
    /// system mixes, exactly as two local radios are.
    fn open() -> Self {
        let settings = Settings::load();
        let mut bridge = CpalBridge {
            _out: None,
            out: None,
            _mic: None,
            mic: None,
            pending_mic: None,
            mic_resampler: None,
            raw: Vec::new(),
            out_name: settings.audio_output.clone(),
            in_name: settings.audio_input.clone(),
            routing: settings.audio_routing,
        };
        bridge.open_output();
        bridge.open_input();
        bridge
    }

    fn open_output(&mut self) {
        self._out = None;
        self.out = None;
        match sdroxide_audio::start_output(self.out_name.as_deref(), 48_000, self.routing) {
            Ok((o, p)) => {
                self._out = Some(o);
                self.out = Some(p);
            }
            Err(e) => warn!("no audio output ({e}); running silent"),
        }
    }

    /// Start the open; [`CpalBridge::poll_pending_mic`] finishes it. Assigning
    /// `pending_mic` drops any open still in flight, so an earlier pick cannot
    /// land on top of a later one.
    fn open_input(&mut self) {
        self._mic = None;
        self.mic = None;
        self.pending_mic =
            Some(sdroxide_audio::start_input_background(self.in_name.clone(), 48_000));
    }

    fn poll_pending_mic(&mut self) {
        let Some(result) = self.pending_mic.as_mut().and_then(|p| p.take()) else { return };
        self.pending_mic = None;
        match result {
            Ok((i, c)) => {
                self.mic_resampler = sdroxide_dsp::MonoResampler::new(i.sample_rate, 48_000.0);
                self._mic = Some(i);
                self.mic = Some(c);
            }
            Err(e) => warn!("no microphone ({e}); TX carries silence"),
        }
    }
}

impl sdroxide_ui::AudioBridge for CpalBridge {
    fn caps(&self) -> sdroxide_proto::AudioCaps {
        sdroxide_proto::AudioCaps { opus_decode: false, opus_encode: false }
    }
    fn play(&mut self, pcm: &[f32]) {
        if let Some(out) = self.out.as_mut() {
            sdroxide_audio::queue_mono(out, pcm, sdroxide_audio::PLAYOUT_BACKLOG);
        }
    }
    fn pull_mic(&mut self, out_vec: &mut Vec<f32>) {
        self.poll_pending_mic();
        let Some(mic) = self.mic.as_mut() else { return };
        self.raw.clear();
        while let Ok(s) = mic.pop() {
            self.raw.push(s);
        }
        match &mut self.mic_resampler {
            Some(r) => r.push(&self.raw, out_vec),
            None => out_vec.extend_from_slice(&self.raw),
        }
    }
    fn devices(&self) -> Option<sdroxide_types::AudioDevices> {
        Some(sdroxide_types::AudioDevices {
            outputs: sdroxide_audio::output_device_names(),
            inputs: sdroxide_audio::input_device_names(),
            selected_output: self.out_name.clone(),
            selected_input: self.in_name.clone(),
            routing: self.routing,
        })
    }
    fn set_device(&mut self, output: bool, name: Option<String>) {
        if output {
            self.out_name = name;
            self.open_output();
        } else {
            self.in_name = name;
            self.open_input();
        }
        // Same persisted selection as the local app (same machine).
        let mut s = sdroxide_config::Settings::load();
        s.audio_output = self.out_name.clone();
        s.audio_input = self.in_name.clone();
        s.audio_routing = self.routing;
        if let Err(e) = s.save() {
            warn!("saving audio device selection: {e}");
        }
    }

    fn set_routing(&mut self, routing: sdroxide_types::AudioRouting) {
        if routing == self.routing {
            return;
        }
        self.routing = routing;
        self.open_output();
        let mut s = sdroxide_config::Settings::load();
        s.audio_routing = self.routing;
        if let Err(e) = s.save() {
            warn!("saving audio routing: {e}");
        }
    }
}

/// Render N frames of the UI to a PNG file, then exit.
///
/// The window is placed off-screen (see the viewport builder below) so nothing
/// flashes on the operator's display, but it is still *rendered* — eframe does
/// not paint a hidden window, so `with_visible(true)` is required.
pub fn run_screenshot(
    radios: Vec<RadioBoot>,
    settings: &Settings,
    tx_ham_only: bool,
    factory_cli: crate::Cli,
    output_path: std::path::PathBuf,
    frames: usize,
    width: u32,
    height: u32,
    native: bool,
    actions: Vec<sdroxide_ui::screenshot::ScreenshotAction>,
) -> Result<()> {
    use std::sync::{Arc, Mutex};

    let gate = Arc::new(TxGate::new());
    let sync = Arc::new(StoreSync::new());
    let rade = Arc::new(RadeWatch::new());
    let tr = Arc::new(TrSwitch::new());

    let mut tabs = Vec::new();
    for (i, boot) in radios.into_iter().enumerate() {
        let id = boot.id;
        let name = boot.name.clone();
        let enabled = boot.enabled;
        let ctrl = build_controller(boot, settings, tx_ham_only, i == 0, &gate, &sync, &rade, &tr);
        tabs.push(sdroxide_ui::RadioTab { id, name, enabled, ctrl: Box::new(ctrl) });
    }

    let factory_gate = gate.clone();
    let factory_sync = sync.clone();
    let factory_rade = rade.clone();
    let factory_tr = tr.clone();
    let factory: sdroxide_ui::RadioFactory = Box::new(move || {
        let slot = sdroxide_config::create_radio("").map_err(|e| e.to_string())?;
        let settings = Settings::load();
        let store = sdroxide_config::Store::radio(slot.id);
        let mut c = factory_cli.clone();
        let initial_mode = c.apply_session(store.load_session());
        let (source, caps) =
            match crate::open_converted_source(&store.load_radio_config(), &c, &settings) {
                Ok(pair) => pair,
                Err(e) => (
                    Box::new(crate::null_source::NullSource::new(c.center_hz(), format!("{e}")))
                        as Box<dyn IqSource>,
                    crate::synthetic_caps("No radio"),
                ),
            };
        let boot = RadioBoot {
            id: slot.id,
            name: slot.name.clone(),
            enabled: true,
            source,
            caps,
            initial_mode,
            initial_antenna: (None, None),
            reopen: Some(crate::reopen_factory_for(&c, store.clone(), slot.id)),
            store,
            record_iq: None,
        };
        let ctrl = build_controller(
            boot,
            &settings,
            tx_ham_only,
            false,
            &factory_gate,
            &factory_sync,
            &factory_rade,
            &factory_tr,
        );
        Ok(sdroxide_ui::RadioTab {
            id: slot.id,
            name: slot.name,
            enabled: true,
            ctrl: Box::new(ctrl),
        })
    });

    sdroxide_ui::install_renderer_panic_note();

    // Make the process DPI-unaware so `with_inner_size` sets physical pixels
    // directly. Otherwise Windows scales the logical size by the monitor's DPI
    // factor (1.75 on this machine) and the window renders at 2240x1400
    // instead of 1280x800.
    #[cfg(target_os = "windows")]
    {
        #[link(name = "user32")]
        unsafe extern "system" {
            fn SetProcessDpiAwarenessContext(value: isize) -> i32;
        }
        // DPI_AWARENESS_CONTEXT_UNAWARE = -4
        unsafe { SetProcessDpiAwarenessContext(-4); }
    }

    let result: sdroxide_ui::ScreenshotResult = Arc::new(Mutex::new(None));

    let frames = frames.max(1);
    let out_path = output_path.clone();
    let result_clone = result.clone();

    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        wgpu_options: sdroxide_ui::wgpu_options(),
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([width as f32, height as f32])
            .with_position(egui::pos2(100.0, 100.0))
            .with_maximized(false)
            .with_fullscreen(false)
            .with_resizable(false)
            .with_visible(true)
            .with_app_id("sdroxide")
            .with_icon(sdroxide_ui::app_icon())
            .with_title(sdroxide_ui::window_title("")),
        centered: false,
        ..Default::default()
    };

    let run_result = sdroxide_ui::event_loop::run(
        "sdroxide",
        options,
        Box::new(move |cc| {
            let mut multi = sdroxide_ui::MultiApp::new(cc, tabs, Some(factory), Some(remote_factory()));
            Ok(Box::new(sdroxide_ui::ScreenshotApp::new(
                multi,
                frames,
                width,
                height,
                out_path,
                result_clone,
                native,
                actions,
            )))
        }),
    );

    // Report the screenshot outcome.
    match result.lock().unwrap().take() {
        Some(Ok(())) => {
            println!("Screenshot saved to {}", output_path.display());
            Ok(())
        }
        Some(Err(e)) => Err(anyhow::anyhow!("screenshot failed: {e}")),
        None => {
            // The window closed without producing a screenshot.
            if let Err(e) = run_result {
                Err(anyhow::anyhow!("eframe: {e}"))
            } else {
                Err(anyhow::anyhow!(
                    "screenshot was not captured (window closed before the capture frame)"
                ))
            }
        }
    }
}

/// Dial an sdroxide server and wrap the session as a radio tab.
///
/// Used both for `--connect`, where it is the whole session, and for Settings →
/// Remote, where it is one more tab beside the radios already open. `id` keys
/// that tab's saved layout; `ctx` is what the socket wakes when a message
/// arrives.
fn connect_remote(
    url: &str,
    id: u32,
    ctx: &eframe::egui::Context,
) -> Result<sdroxide_ui::RadioTab, String> {
    let ctx = ctx.clone();
    // A deadline hint, not an immediate repaint: audio packets arrive far
    // faster than the display needs to redraw, and egui takes the soonest of
    // all requested deadlines anyway — this escapes the idle poll quickly
    // without outpacing the app's frame scheduler.
    let ctrl = sdroxide_ui::RemoteController::connect(
        url,
        Some(Box::new(CpalBridge::open())),
        move || sdroxide_ui::repaint::after_ms(&ctx, 33),
    )?;
    // Named after the address, minus the parts every server shares — the shell
    // overrides this with whatever the operator typed when the connection came
    // from the General tab.
    // Somebody else's radio: this station's switch does not reach it (see
    // [`sdroxide_ui::RadioTab::enabled`]).
    Ok(sdroxide_ui::RadioTab { id, name: tab_name(url), enabled: true, ctrl: Box::new(ctrl) })
}

/// What to call a connection in the tab strip: the address without the scheme
/// or the endpoint every server shares, and — when the address names one of a
/// station's further radios — which radio it is, since that is the only thing
/// telling two tabs on the same station apart.
fn tab_name(url: &str) -> String {
    let rest = url.split_once("://").map_or(url, |(_, rest)| rest).trim_end_matches('/');
    let Some((host, path)) = rest.split_once("/ws") else { return rest.to_string() };
    match path.trim_start_matches('/').parse::<u32>() {
        // Radio 0 is what a bare address reaches anyway; spelling it out in
        // the strip would only take room from the host.
        Ok(0) | Err(_) => host.to_string(),
        Ok(id) => format!("{host} radio {}", id + 1),
    }
}

/// The General tab's dialler, handed to the shell so CONNECT has something to
/// call. Lives here because the connection needs this machine's sound devices.
fn remote_factory() -> sdroxide_ui::RemoteFactory {
    Box::new(connect_remote)
}

/// Native remote client: same app, `RemoteController` over WebSocket, audio
/// through local cpal devices.
pub fn run_remote(url: &str) -> Result<()> {
    // Ahead of `wgpu_options`, which reads the note this leaves behind.
    sdroxide_ui::install_renderer_panic_note();
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        wgpu_options: sdroxide_ui::wgpu_options(),
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([800.0, 500.0])
            .with_app_id("sdroxide")
            .with_icon(sdroxide_ui::app_icon())
            .with_title(sdroxide_ui::window_title(&format!("remote {url}"))),
        // Centred on a first start, for the reason the local window is — see
        // `run` above (issues #234 and #256).
        centered: true,
        ..Default::default()
    };
    // Connect inside the creator so the socket can wake the UI (repaint) the
    // moment a message arrives, instead of waiting for the next poll.
    let url = url.to_string();
    sdroxide_ui::event_loop::run(
        "sdroxide-remote",
        options,
        Box::new(move |cc| {
            // Radio 0, so a client that has always been started this way keeps
            // the layout it saved before there was a shell around it.
            let tab =
                connect_remote(&url, 0, &cc.egui_ctx).map_err(|e| format!("connect {url}: {e}"))?;
            // No radio factory: this machine's own hardware, if it has any, is
            // not what this session is for. A *further* server still is —
            // Settings → General works here exactly as it does in the shack.
            Ok(Box::new(sdroxide_ui::MultiApp::new(cc, vec![tab], None, Some(remote_factory()))))
        }),
    )
    .map_err(|e| anyhow::anyhow!("eframe: {e}"))
}
