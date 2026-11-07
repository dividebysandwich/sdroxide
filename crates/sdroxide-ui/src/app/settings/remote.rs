//! The Remote tab: connect this screen to an sdroxide server somewhere else.
//!
//! The same thing `--connect` does on the command line, without the command
//! line — and, because a station reached this way arrives as a radio tab of its
//! own, without giving up the radios already open here. The engine stays where
//! the antenna is; what crosses the network is the spectrum, the audio and the
//! commands.
//!
//! Native only. A browser client is already attached to the server that served
//! it and has nowhere to put a second connection, so there is no such tab
//! there — see [`crate::app::persist::load_remote_server`].
//!
//! What is edited here is the *screen's* setting, not the station's: it is
//! written to this machine's `config.toml` as it is typed, the way the control
//! bindings are, because there is nothing to apply and a pressed CONNECT is a
//! poor moment to discover the address was never saved.

use crate::i18n::tr;
use eframe::egui::{self, RichText};
use sdroxide_types::RemoteServer;

/// Draw the tab. `connect` is set when the operator presses CONNECT; the dial
/// itself happens in the shell, after the frame — it is the only thing that can
/// add the tab the connection lands in.
///
/// `can_connect` is whether this session can hold another radio at all (false
/// where there is no shell around the app to put one in), and `status` is what
/// the last attempt from this screen did, if there was one.
pub(in crate::app) fn settings_remote_tab(
    ui: &mut egui::Ui,
    cfg: &mut RemoteServer,
    connect: &mut bool,
    can_connect: bool,
    status: Option<&Result<String, String>>,
) {
    ui.label(
        RichText::new(tr!("settings-remote-connect-server"))
            .size(14.0)
            .strong()
            .color(crate::theme::CYAN()),
    );
    ui.add_space(4.0);
    ui.label(RichText::new(tr!("settings-remote-drive-sdroxide-running-somewhere-else")).weak());
    ui.add_space(8.0);

    // A pasted URL carries its own scheme and its own port (or that scheme's
    // default), so the two boxes below have nothing left to say and grey out
    // rather than sitting there looking as though they still apply.
    let typed_url = cfg.host.contains("://");
    let mut entered = false;
    egui::Grid::new("remote-grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label(RichText::new(tr!("settings-remote-address")).strong());
        let addr = crate::chrome::field(
            ui,
            egui::TextEdit::singleline(&mut cfg.host)
                .desired_width(240.0)
                .hint_text(tr!("settings-remote-host-name-ip")),
        );
        let addr = addr.on_hover_text(tr!("settings-remote-machine-running-sdroxide-server-host"));
        // Enter in the address box is the same as pressing CONNECT: an address
        // box is a thing one types into and presses return on.
        entered = addr.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        ui.end_row();

        ui.label(RichText::new(tr!("settings-remote-port")).strong());
        ui.add_enabled_ui(!typed_url, |ui| {
            ui.add(egui::DragValue::new(&mut cfg.port).range(1..=65535))
                .on_hover_text(tr!("settings-remote-port-server-listens-server-port"));
        });
        ui.end_row();

        // The scheme, as a switch rather than as something to be spelled out in
        // the address box. A browser client reads it off the page it was served
        // from; this one has no page, so the operator has to say — and until
        // they could, a station behind an HTTPS proxy on 443 was reachable only
        // by typing the whole URL out, which then took the port box with it
        // (issue #360).
        ui.label(RichText::new(tr!("settings-remote-secure")).strong());
        ui.add_enabled_ui(!typed_url, |ui| {
            ui.checkbox(&mut cfg.tls, tr!("settings-remote-wss-tls"))
                .on_hover_text(tr!("settings-remote-tick-where-server-sits-behind"));
        });
        ui.end_row();
    });

    ui.add_space(8.0);
    let ready = !cfg.host.trim().is_empty() && can_connect;
    ui.horizontal(|ui| {
        let button = ui.add_enabled_ui(ready, |ui| {
            crate::chrome::chip_accent(
                ui,
                false,
                RichText::new(tr!("settings-remote-connect")).strong(),
                crate::theme::GREEN(),
                crate::theme::INK_ON_CYAN(),
            )
        });
        if ready
            && (button
                .inner
                .on_hover_text(tr!("settings-remote-open-station-radio-tab-own"))
                .clicked()
                || entered)
        {
            *connect = true;
        }
        if !cfg.host.trim().is_empty() {
            ui.add(egui::Label::new(RichText::new(cfg.url()).weak().monospace()).wrap());
        }
    });

    if !can_connect {
        ui.add_space(4.0);
        ui.label(
            RichText::new(tr!("settings-remote-client-cannot-hold-second-connection"))
                .color(crate::theme::ALERT()),
        );
    }

    match status {
        Some(Ok(msg)) => {
            ui.add_space(6.0);
            ui.label(RichText::new(msg).color(crate::theme::GREEN()));
        }
        Some(Err(e)) => {
            ui.add_space(6.0);
            ui.add(egui::Label::new(RichText::new(e).color(crate::theme::ALERT())).wrap());
        }
        None => {}
    }

    ui.add_space(10.0);
    ui.separator();
    ui.add_space(6.0);
    ui.label(RichText::new(tr!("settings-remote-station-appears-tab-beside-radios")).weak());

    #[cfg(all(feature = "remote", not(target_arch = "wasm32")))]
    trusted_certs(ui);
}

/// The `wss://` certificates trusted on this machine, each with a way back to
/// ordinary checking — for a server that has since been given a proper
/// certificate, or one whose certificate was renewed and now needs trusting
/// afresh.
#[cfg(all(feature = "remote", not(target_arch = "wasm32")))]
fn trusted_certs(ui: &mut egui::Ui) {
    let certs = crate::remote_tls::all_trusted();
    if certs.is_empty() {
        return;
    }
    ui.add_space(10.0);
    ui.label(
        RichText::new(tr!("settings-remote-trusted-certificates"))
            .size(14.0)
            .strong()
            .color(crate::theme::CYAN()),
    );
    ui.add_space(4.0);
    for (authority, sha256) in certs {
        ui.horizontal(|ui| {
            ui.label(RichText::new(&authority).strong());
            if ui
                .small_button(tr!("settings-remote-forget"))
                .on_hover_text(tr!("settings-remote-forget-hint"))
                .clicked()
                && let Err(e) = crate::remote_tls::forget(&authority)
            {
                tracing::warn!("forgetting the certificate for {authority}: {e}");
            }
        });
        ui.add(
            egui::Label::new(
                RichText::new(sdroxide_types::fingerprint_display(&sha256)).monospace().size(10.0),
            )
            .wrap(),
        );
    }
}
