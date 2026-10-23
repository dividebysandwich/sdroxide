//! Headless-ish screenshot capture for the main window.
//!
//! Wraps a [`crate::multi::MultiApp`] and, after a configurable number of
//! frames (or after executing a list of actions from `--screenshot-actions`),
//! asks egui for a screenshot of the next frame, writes it as a PNG, and closes
//! the window. Used by the `--screenshot` / `--screenshot-actions` CLI flags to
//! produce a preview image without a human at the keyboard.

use crate::multi::MultiApp;
use eframe::egui;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Result channel shared between the wrapper and the caller.
pub type ScreenshotResult = Arc<Mutex<Option<Result<(), String>>>>;

/// A parsed screenshot action.
#[derive(Debug, Clone)]
pub enum ScreenshotAction {
    Open(String),
    Click(String),
    Hover(String),
    Tab(String),
    Wait(u32),
    Capture,
}

/// Parse an actions file into a list of actions.
pub fn parse_actions(content: &str) -> Result<Vec<ScreenshotAction>, String> {
    let mut actions = Vec::new();
    for (lineno, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.splitn(2, char::is_whitespace).collect();
        let cmd = parts[0].to_lowercase();
        let arg = parts.get(1).map(|s| s.trim()).unwrap_or("");
        match cmd.as_str() {
            "open" => {
                if arg.is_empty() {
                    return Err(format!("line {}: 'open' requires an argument", lineno + 1));
                }
                actions.push(ScreenshotAction::Open(arg.to_string()));
            }
            "click" => {
                if arg.is_empty() {
                    return Err(format!("line {}: 'click' requires an argument", lineno + 1));
                }
                actions.push(ScreenshotAction::Click(arg.to_string()));
            }
            "hover" => {
                if arg.is_empty() {
                    return Err(format!("line {}: 'hover' requires an argument", lineno + 1));
                }
                actions.push(ScreenshotAction::Hover(arg.to_string()));
            }
            "tab" => {
                if arg.is_empty() {
                    return Err(format!("line {}: 'tab' requires an argument", lineno + 1));
                }
                actions.push(ScreenshotAction::Tab(arg.to_string()));
            }
            "wait" => {
                let ms: u32 = arg.parse().map_err(|_| {
                    format!("line {}: 'wait' requires a number of milliseconds", lineno + 1)
                })?;
                actions.push(ScreenshotAction::Wait(ms));
            }
            "capture" => {
                actions.push(ScreenshotAction::Capture);
            }
            _ => {
                return Err(format!(
                    "line {}: unknown action '{}'. Valid: open, click, hover, tab, wait, capture",
                    lineno + 1,
                    cmd
                ));
            }
        }
    }
    if actions.is_empty() {
        return Err("actions file is empty".to_string());
    }
    Ok(actions)
}

pub struct ScreenshotApp {
    inner: MultiApp,
    frames: usize,
    target_frames: usize,
    width: u32,
    height: u32,
    output_path: PathBuf,
    requested: bool,
    done: bool,
    result: ScreenshotResult,
    resized: bool,
    native: bool,
    // Action queue
    actions: Vec<ScreenshotAction>,
    action_index: usize,
    wait_frames: u32,
    actions_done: bool,
}

impl ScreenshotApp {
    pub fn new(
        inner: MultiApp,
        target_frames: usize,
        width: u32,
        height: u32,
        output_path: PathBuf,
        result: ScreenshotResult,
        native: bool,
        actions: Vec<ScreenshotAction>,
    ) -> Self {
        let ae = actions.is_empty();
        Self {
            inner,
            frames: 0,
            target_frames,
            width,
            height,
            output_path,
            requested: false,
            done: false,
            result,
            resized: false,
            native,
            actions,
            action_index: 0,
            wait_frames: 0,
            actions_done: ae,
        }
    }

    /// Execute the next pending action. Returns Ok(()) on success, Err on failure.
    fn execute_next_action(&mut self, ctx: &egui::Context) -> Result<(), String> {
        if self.actions_done || self.action_index >= self.actions.len() {
            self.actions_done = true;
            return Ok(());
        }

        if self.wait_frames > 0 {
            self.wait_frames -= 1;
            return Ok(());
        }

        let action = self.actions[self.action_index].clone();
        match action {
            ScreenshotAction::Open(name) => {
                let name_lower = name.to_lowercase();
                if name_lower == "settings" || name_lower.starts_with("settings") {
                    let tab = name.split_whitespace().nth(1);
                    self.inner.open_settings(tab);
                } else if name_lower == "schedule" {
                    self.inner.open_dialog("schedule");
                } else {
                    return Err(format!(
                        "'open {}': unknown dialog (try 'settings' or 'schedule')",
                        name
                    ));
                }
                self.action_index += 1;
                self.wait_frames = 2;
            }
            ScreenshotAction::Click(name) => {
                let parts: Vec<&str> = name.split_whitespace().collect();
                if parts.len() == 2 {
                    if let (Ok(x), Ok(y)) = (parts[0].parse::<f32>(), parts[1].parse::<f32>()) {
                        let pos = egui::pos2(x, y);
                        ctx.input_mut(|i| {
                            i.events.push(egui::Event::PointerButton {
                                pos,
                                button: egui::PointerButton::Primary,
                                pressed: true,
                                modifiers: egui::Modifiers::default(),
                            });
                        });
                        self.wait_frames = 1;
                        self.action_index += 1;
                        return Ok(());
                    }
                }
                let name_lower = name.to_lowercase();
                if name_lower == "settings" || name_lower.starts_with("settings") {
                    let tab = name.split_whitespace().nth(1);
                    self.inner.open_settings(tab);
                    self.action_index += 1;
                    self.wait_frames = 2;
                    return Ok(());
                }
                if name_lower == "schedule" {
                    self.inner.open_dialog("schedule");
                    self.action_index += 1;
                    self.wait_frames = 2;
                    return Ok(());
                }
                if name_lower == "all_chip" {
                    self.inner.toggle_all_chip();
                    self.action_index += 1;
                    self.wait_frames = 2;
                    return Ok(());
                }
                if name_lower == "fav_chip" {
                    self.inner.toggle_fav_chip();
                    self.action_index += 1;
                    self.wait_frames = 2;
                    return Ok(());
                }
                return Err(format!(
                    "'click {}': unknown element (use coordinates: click X Y, or 'settings')",
                    name
                ));
            }
            ScreenshotAction::Hover(name) => {
                let parts: Vec<&str> = name.split_whitespace().collect();
                if parts.len() == 2 {
                    if let (Ok(x), Ok(y)) = (parts[0].parse::<f32>(), parts[1].parse::<f32>()) {
                        let pos = egui::pos2(x, y);
                        ctx.input_mut(|i| {
                            i.events.push(egui::Event::PointerMoved(pos));
                        });
                        self.action_index += 1;
                        return Ok(());
                    }
                }
                return Err(format!(
                    "'hover {}': unknown element (use coordinates: hover X Y)",
                    name
                ));
            }
            ScreenshotAction::Tab(name) => {
                // If settings is open, switch the settings tab; otherwise switch radio tab.
                self.inner.set_settings_tab(&name);
                self.inner.switch_tab(&name);
                self.action_index += 1;
                self.wait_frames = 2;
            }
            ScreenshotAction::Wait(ms) => {
                let frames = ((ms as f32 / 16.67).ceil() as u32).max(1);
                self.wait_frames = frames;
                self.action_index += 1;
            }
            ScreenshotAction::Capture => {
                self.action_index += 1;
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
                self.requested = true;
                self.actions_done = true;
            }
        }
        Ok(())
    }
}

impl eframe::App for ScreenshotApp {
    fn raw_input_hook(&mut self, ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        eframe::App::raw_input_hook(&mut self.inner, ctx, raw_input);
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        if !self.resized {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                self.width as f32,
                self.height as f32,
            )));
            self.resized = true;
        }

        eframe::App::ui(&mut self.inner, ui, frame);

        if !self.actions_done && !self.done {
            match self.execute_next_action(ui.ctx()) {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("screenshot action error: {}", e);
                    *self.result.lock().unwrap() = Some(Err(e));
                    self.done = true;
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }

        if self.requested && !self.done {
            let events = ui.ctx().input(|i| i.events.clone());
            for ev in events {
                if let egui::Event::Screenshot { image, .. } = ev {
                    let img = image.as_ref();
                    let rendered_w = img.size[0] as u32;
                    let rendered_h = img.size[1] as u32;
                    let mut rgba = image::RgbaImage::new(rendered_w, rendered_h);
                    for (i, px) in img.pixels.iter().enumerate() {
                        let x = (i % img.size[0]) as u32;
                        let y = (i / img.size[0]) as u32;
                        rgba.put_pixel(x, y, image::Rgba([px.r(), px.g(), px.b(), px.a()]));
                    }
                    let (rgba, was_resized) = if self.native {
                        eprintln!(
                            "screenshot: native mode, writing at rendered resolution {}x{}",
                            rendered_w, rendered_h
                        );
                        (rgba, false)
                    } else if rendered_w != self.width || rendered_h != self.height {
                        let scale_x = rendered_w as f64 / self.width as f64;
                        let scale_y = rendered_h as f64 / self.height as f64;
                        eprintln!(
                            "screenshot: rendered {}x{} != requested {}x{} (scale {:.3}x{:.3}), resizing",
                            rendered_w, rendered_h, self.width, self.height, scale_x, scale_y
                        );
                        let resized = image::imageops::resize(
                            &rgba,
                            self.width,
                            self.height,
                            image::imageops::FilterType::Lanczos3,
                        );
                        (resized, true)
                    } else {
                        (rgba, false)
                    };
                    let _ = was_resized;
                    let outcome = match rgba.save(&self.output_path) {
                        Ok(()) => Ok(()),
                        Err(e) => Err(format!("saving PNG: {e}")),
                    };
                    *self.result.lock().unwrap() = Some(outcome);
                    self.done = true;
                }
            }
        }

        if self.actions.is_empty() && !self.requested && self.frames >= self.target_frames {
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            self.requested = true;
        }

        self.frames += 1;
        if self.done {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::App::save(&mut self.inner, storage);
    }

    fn on_exit(&mut self) {
        eframe::App::on_exit(&mut self.inner);
    }

    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        eframe::App::clear_color(&self.inner, visuals)
    }
}
