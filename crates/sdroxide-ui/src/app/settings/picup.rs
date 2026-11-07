//! Received pictures → FTP, on the Uploads tab.
//!
//! The station does the uploading, so this only edits the settings and asks it
//! to try them. Like the rest of the tab it has an APPLY button rather than
//! saving as you type: a half-typed host or password would otherwise be tried
//! with the next picture.

use crate::i18n::{Tr, tr};
use eframe::egui::{self, Color32, ComboBox, RichText};
use sdroxide_types::{
    Command, FtpSecurity, PictureUploadConfig, PictureUploadStatus, TlsVerify, UploadFormat,
    UploadNaming, format_fingerprint,
};

use super::net::{net_heading, net_row, net_secret};
use crate::chrome::StyledCombo;

const RED: Color32 = Color32::from_rgb(255, 120, 120);

fn note(ui: &mut egui::Ui, text: &str, colour: Color32) {
    ui.label(RichText::new(text).size(10.5).color(colour));
}

fn result_line(ui: &mut egui::Ui, st: &PictureUploadStatus) {
    let (mark, colour) = if st.ok { ("✔", crate::theme::GREEN()) } else { ("✖", RED) };
    let secs = st.unix.rem_euclid(86_400);
    let when = format!("{:02}:{:02} UTC", secs / 3600, secs % 3600 / 60);
    ui.label(RichText::new(format!("{mark} {when}  {}", st.message)).size(11.0).color(colour));
}

impl crate::app::SdroxideApp {
    /// The section itself. `edit` is the dialog's copy; `apply` asks for it to
    /// be sent.
    pub(in crate::app) fn picup_section(
        &self,
        ui: &mut egui::Ui,
        cmds: &mut Vec<Command>,
        edit: &mut PictureUploadConfig,
        apply: &mut bool,
    ) {
        net_heading(ui, &tr!("settings-picup-received-pictures-ftp"));
        note(
            ui,
            &tr!("settings-picup-uploads-every-received-sstv-picture"),
            crate::theme::gray(140),
        );
        crate::chrome::checkbox(
            ui,
            &mut edit.enabled,
            tr!("settings-picup-upload-received-pictures"),
        );

        ui.add_enabled_ui(edit.enabled, |ui| {
            net_row(ui, &tr!("settings-picup-server"), &mut edit.host, 200.0);
            ui.horizontal(|ui| {
                ui.add_sized([96.0, 22.0], egui::Label::new(tr!("settings-picup-port")));
                ui.add(egui::DragValue::new(&mut edit.port).range(1..=65535));
            });
            ui.horizontal(|ui| {
                ui.add_sized([96.0, 22.0], egui::Label::new(tr!("settings-picup-security")));
                let before = edit.security;
                ComboBox::from_id_salt("picup_security")
                    .selected_text(edit.security.tr())
                    .show_styled(ui, |ui| {
                        for s in FtpSecurity::ALL {
                            ui.selectable_value(&mut edit.security, s, s.tr());
                        }
                    });
                // Follow the change of convention, but only from the port
                // the old choice would have had — never over one typed in.
                if edit.security != before && edit.port == before.default_port() {
                    edit.port = edit.security.default_port();
                }
            });
            net_row(ui, &tr!("settings-picup-user"), &mut edit.user, 160.0);
            net_secret(ui, &tr!("settings-picup-password"), &mut edit.password, 160.0);

            if edit.security.is_tls() {
                ui.horizontal(|ui| {
                    ui.add_sized([96.0, 22.0], egui::Label::new(tr!("settings-picup-certificate")));
                    ComboBox::from_id_salt("picup_verify")
                        .selected_text(edit.verify.tr())
                        .show_styled(ui, |ui| {
                            for v in TlsVerify::ALL {
                                ui.selectable_value(&mut edit.verify, v, v.tr());
                            }
                        });
                });
                match edit.verify {
                    TlsVerify::System => {
                        note(
                            ui,
                            &tr!("settings-picup-trusted-when-computer-s-operating"),
                            crate::theme::gray(140),
                        );
                        ui.horizontal(|ui| {
                            ui.add_sized(
                                [96.0, 22.0],
                                egui::Label::new(tr!("settings-picup-extra-ca")),
                            );
                            crate::chrome::field(
                                ui,
                                egui::TextEdit::multiline(&mut edit.extra_ca_pem)
                                    .desired_rows(2)
                                    .desired_width(320.0)
                                    .font(egui::TextStyle::Monospace)
                                    .hint_text("-----BEGIN CERTIFICATE-----"),
                            );
                        });
                    }
                    TlsVerify::Pinned => {
                        net_row(ui, "SHA-256", &mut edit.pinned_sha256, 320.0);
                        note(
                            ui,
                            &tr!("settings-picup-only-certificate-fingerprint-accepted-way"),
                            crate::theme::gray(140),
                        );
                    }
                    TlsVerify::Off => note(
                        ui,
                        &tr!("settings-picup-any-certificate-accepted-link-encrypted"),
                        RED,
                    ),
                }
            } else {
                note(
                    ui,
                    &tr!("settings-picup-plain-ftp-sends-password-pictures"),
                    crate::theme::YELLOW(),
                );
            }

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.add_sized([96.0, 22.0], egui::Label::new(""));
                crate::chrome::checkbox(ui, &mut edit.upload_sstv, tr!("settings-picup-sstv"));
            });
            ui.add_enabled_ui(edit.upload_sstv, |ui| {
                net_row(ui, &tr!("settings-picup-sstv-dir"), &mut edit.sstv_dir, 200.0);
            });
            ui.horizontal(|ui| {
                ui.add_sized([96.0, 22.0], egui::Label::new(""));
                crate::chrome::checkbox(
                    ui,
                    &mut edit.upload_wefax,
                    tr!("settings-picup-weather-fax"),
                );
            });
            ui.add_enabled_ui(edit.upload_wefax, |ui| {
                net_row(ui, &tr!("settings-picup-wefax-dir"), &mut edit.wefax_dir, 200.0);
            });
            note(ui, &tr!("settings-picup-empty-login-directory"), crate::theme::gray(140));

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.add_sized([96.0, 22.0], egui::Label::new(tr!("settings-picup-file-names")));
                ComboBox::from_id_salt("picup_naming").selected_text(edit.naming.tr()).show_styled(
                    ui,
                    |ui| {
                        for n in UploadNaming::ALL {
                            ui.selectable_value(&mut edit.naming, n, n.tr());
                        }
                    },
                );
                if edit.naming == UploadNaming::Sequence {
                    ui.label("keep");
                    ui.add(egui::DragValue::new(&mut edit.keep_images).range(1..=999));
                }
            });
            ui.horizontal(|ui| {
                ui.add_sized([96.0, 22.0], egui::Label::new(tr!("settings-picup-format")));
                ComboBox::from_id_salt("picup_format")
                    .selected_text(edit.format.label())
                    .show_styled(ui, |ui| {
                        for f in UploadFormat::ALL {
                            ui.selectable_value(&mut edit.format, f, f.label());
                        }
                    });
                if edit.naming == UploadNaming::Sequence {
                    crate::chrome::checkbox(
                        ui,
                        &mut edit.add_extension,
                        tr!("settings-picup-add-extension"),
                    )
                    .on_hover_text(tr!("settings-picup-image1-png-rather-than-qsstv"));
                }
            });
            ui.horizontal_wrapped(|ui| {
                ui.add_sized([96.0, 22.0], egui::Label::new(""));
                crate::chrome::checkbox(ui, &mut edit.stamp, tr!("settings-picup-caption-bar"))
                    .on_hover_text(tr!("settings-picup-time-frequency-mode-sender-s"));
                crate::chrome::checkbox(ui, &mut edit.downscale, tr!("settings-picup-fit-960-768"));
                crate::chrome::checkbox(ui, &mut edit.heartbeat, "checknew.jpg")
                    .on_hover_text(tr!("settings-picup-qsstv-s-heartbeat-tiny-picture"));
                crate::chrome::checkbox(
                    ui,
                    &mut edit.passive_nat_workaround,
                    tr!("settings-picup-nat-workaround"),
                )
                .on_hover_text(tr!("settings-picup-ignore-private-address-server-s"));
            });
        });

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if crate::chrome::chip_accent(
                ui,
                false,
                RichText::new(tr!("settings-picup-apply")).strong(),
                crate::theme::GREEN(),
                crate::theme::INK_ON_CYAN(),
            )
            .clicked()
            {
                *apply = true;
            }
            let waiting = self.picup_test_pending.is_some();
            let test = ui.add_enabled(
                !waiting && edit.is_usable(),
                egui::Button::new(RichText::new(tr!("settings-picup-test")).size(11.0)),
            );
            if test
                .on_hover_text(tr!("settings-picup-connect-settings-above-applied-not"))
                .clicked()
            {
                cmds.push(Command::TestPictureUpload(Box::new(edit.clone())));
            }
            if waiting {
                ui.label(RichText::new(tr!("settings-picup-checking")).size(11.0).weak());
            }
        });
        if let Some(st) = self.picup_test.as_ref().filter(|_| self.picup_test_pending.is_none()) {
            result_line(ui, st);
            if let Some(fp) = st.fingerprint.as_deref() {
                let pinned = edit.verify == TlsVerify::Pinned
                    && sdroxide_types::normalize_fingerprint(&edit.pinned_sha256) == fp;
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(tr!(
                            "settings-picup-server-fingerprint",
                            fingerprint = format_fingerprint(fp)
                        ))
                        .size(10.5)
                        .monospace(),
                    );
                    if !pinned
                        && ui
                            .small_button(tr!("settings-picup-trust-certificate"))
                            .on_hover_text(tr!("settings-picup-pin-switch-pinned-fingerprint-one"))
                            .clicked()
                    {
                        edit.verify = TlsVerify::Pinned;
                        edit.pinned_sha256 = format_fingerprint(fp);
                    }
                });
            }
        }
        if let Some(st) = self.picup_last.as_ref() {
            ui.horizontal(|ui| {
                ui.label(RichText::new(tr!("settings-picup-last-upload")).size(11.0).weak());
                result_line(ui, st);
            });
        }
    }
}
