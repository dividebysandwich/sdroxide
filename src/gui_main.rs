//! Native GUI mode: engine thread + audio output + eframe window.

use anyhow::Result;
use sdroxide_config::Settings;
use sdroxide_radio::{AudioParams, EngineConfig, IqSource, MicParams, start_engine};
use sdroxide_types::{DeviceCaps, Mode};
use tracing::warn;

use crate::local_controller::LocalController;

pub fn run(
    source: Box<dyn IqSource>,
    caps: DeviceCaps,
    settings: &Settings,
    initial_mode: Option<Mode>,
) -> Result<()> {
    // The cpal streams must outlive the GUI; keep the handles on this stack frame.
    let (_audio_out, audio_params) = match sdroxide_audio::start_output(48_000) {
        Ok((out, producer)) => {
            let rate = out.sample_rate;
            (Some(out), Some(AudioParams { producer, out_rate: rate }))
        }
        Err(e) => {
            warn!("no audio output ({e}); running silent");
            (None, None)
        }
    };
    let (_mic_in, mic_params) = if caps.is_transmit_capable() {
        match sdroxide_audio::start_input(48_000) {
            Ok((input, consumer)) => {
                let rate = input.sample_rate;
                (Some(input), Some(MicParams { consumer, rate }))
            }
            Err(e) => {
                warn!("no microphone ({e}); TX carries silence");
                (None, None)
            }
        }
    } else {
        (None, None)
    };

    let cfg = EngineConfig {
        audio: audio_params,
        mic: mic_params,
        cal_offset_db: settings.cal_offset_db as f32,
        initial_mode,
        tx_ham_only: settings.tx_ham_only,
    };
    let mut handles = start_engine(source, caps, cfg);
    let engine_thread = handles.thread.take();
    let ctrl = LocalController::new(handles);

    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([800.0, 500.0])
            .with_title("sdroxide"),
        ..Default::default()
    };
    let result = eframe::run_native(
        "sdroxide",
        options,
        Box::new(move |cc| Ok(Box::new(sdroxide_ui::SdroxideApp::new(cc, Box::new(ctrl))))),
    )
    .map_err(|e| anyhow::anyhow!("eframe: {e}"));

    if let Some(out) = &_audio_out {
        tracing::info!(underruns = out.underruns(), "audio session ended");
    }
    // The controller (and its command sender) died with the app above; the
    // engine notices and exits. Wait for it so the SoapySDR device closes
    // before process teardown.
    if let Some(t) = engine_thread {
        let _ = t.join();
    }
    result
}

/// Native remote client: same app, `RemoteController` over WebSocket, audio
/// through local cpal devices.
pub fn run_remote(url: &str) -> Result<()> {
    struct CpalBridge {
        _out: Option<sdroxide_audio::AudioOutput>,
        out: Option<sdroxide_radio::rtrb::Producer<f32>>,
        _mic: Option<sdroxide_audio::AudioInput>,
        mic: Option<sdroxide_radio::rtrb::Consumer<f32>>,
        mic_resampler: Option<sdroxide_dsp::MonoResampler>,
        raw: Vec<f32>,
    }

    impl sdroxide_ui::AudioBridge for CpalBridge {
        fn caps(&self) -> sdroxide_proto::AudioCaps {
            sdroxide_proto::AudioCaps { opus_decode: false, opus_encode: false }
        }
        fn play(&mut self, pcm: &[f32]) {
            if let Some(out) = self.out.as_mut() {
                for &s in pcm {
                    if out.push(s).is_err() || out.push(s).is_err() {
                        break; // ring full
                    }
                }
            }
        }
        fn pull_mic(&mut self, out_vec: &mut Vec<f32>) {
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
    }

    let (out_stream, out_producer) = match sdroxide_audio::start_output(48_000) {
        Ok((o, p)) => (Some(o), Some(p)),
        Err(e) => {
            warn!("no audio output ({e}); running silent");
            (None, None)
        }
    };
    let (mic_stream, mic_consumer, mic_rate) = match sdroxide_audio::start_input(48_000) {
        Ok((i, c)) => {
            let rate = i.sample_rate;
            (Some(i), Some(c), rate)
        }
        Err(e) => {
            warn!("no microphone ({e}); TX carries silence");
            (None, None, 48_000.0)
        }
    };
    let bridge = CpalBridge {
        _out: out_stream,
        out: out_producer,
        _mic: mic_stream,
        mic: mic_consumer,
        mic_resampler: sdroxide_dsp::MonoResampler::new(mic_rate, 48_000.0),
        raw: Vec::new(),
    };

    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([800.0, 500.0])
            .with_title(format!("sdroxide — remote {url}")),
        ..Default::default()
    };
    // Connect inside the creator so the socket can wake the UI (repaint) the
    // moment a message arrives, instead of waiting for the next poll.
    let url = url.to_string();
    eframe::run_native(
        "sdroxide-remote",
        options,
        Box::new(move |cc| {
            let ctx = cc.egui_ctx.clone();
            // A deadline hint, not an immediate repaint: audio packets arrive
            // far faster than the display needs to redraw, and egui takes the
            // soonest of all requested deadlines anyway — this escapes the
            // idle poll quickly without outpacing the app's frame scheduler.
            let ctrl = sdroxide_ui::RemoteController::connect(&url, Some(Box::new(bridge)), move || {
                ctx.request_repaint_after(std::time::Duration::from_millis(33))
            })
            .map_err(|e| format!("connect {url}: {e}"))?;
            Ok(Box::new(sdroxide_ui::SdroxideApp::new(cc, Box::new(ctrl))))
        }),
    )
    .map_err(|e| anyhow::anyhow!("eframe: {e}"))
}
