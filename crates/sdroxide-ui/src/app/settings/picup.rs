//! Received pictures → FTP, on the Uploads tab.
//!
//! The station does the uploading, so this only edits the settings and asks it
//! to try them. Like the rest of the tab it has an APPLY button rather than
//! saving as you type: a half-typed host or password would otherwise be tried
//! with the next picture.

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
        net_heading(ui, "Received pictures → FTP");
        note(
            ui,
            "Uploads every received SSTV picture (and, if ticked, every weather chart) as it is \
             saved, laid out the way QSSTV does it so web pages written for QSSTV work unchanged.",
            crate::theme::gray(140),
        );
        crate::chrome::checkbox(ui, &mut edit.enabled, "Upload received pictures");

        ui.add_enabled_ui(edit.enabled, |ui| {
            net_row(ui, "Server", &mut edit.host, 200.0);
            ui.horizontal(|ui| {
                ui.add_sized([96.0, 22.0], egui::Label::new("Port"));
                ui.add(egui::DragValue::new(&mut edit.port).range(1..=65535));
            });
            ui.horizontal(|ui| {
                ui.add_sized([96.0, 22.0], egui::Label::new("Security"));
                let before = edit.security;
                ComboBox::from_id_salt("picup_security")
                    .selected_text(edit.security.label())
                    .show_styled(ui, |ui| {
                        for s in FtpSecurity::ALL {
                            ui.selectable_value(&mut edit.security, s, s.label());
                        }
                    });
                // Follow the change of convention, but only from the port
                // the old choice would have had — never over one typed in.
                if edit.security != before && edit.port == before.default_port() {
                    edit.port = edit.security.default_port();
                }
            });
            net_row(ui, "User", &mut edit.user, 160.0);
            net_secret(ui, "Password", &mut edit.password, 160.0);

            if edit.security.is_tls() {
                ui.horizontal(|ui| {
                    ui.add_sized([96.0, 22.0], egui::Label::new("Certificate"));
                    ComboBox::from_id_salt("picup_verify")
                        .selected_text(edit.verify.label())
                        .show_styled(ui, |ui| {
                            for v in TlsVerify::ALL {
                                ui.selectable_value(&mut edit.verify, v, v.label());
                            }
                        });
                });
                match edit.verify {
                    TlsVerify::System => {
                        note(
                            ui,
                            "Trusted when this computer's operating system trusts it, as a \
                             browser here would. A private CA can be added below (PEM).",
                            crate::theme::gray(140),
                        );
                        ui.horizontal(|ui| {
                            ui.add_sized([96.0, 22.0], egui::Label::new("Extra CA"));
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
                            "Only the certificate with this fingerprint is accepted — the way \
                             to trust a self-signed server. TEST shows the server's.",
                            crate::theme::gray(140),
                        );
                    }
                    TlsVerify::Off => note(
                        ui,
                        "⚠ Any certificate is accepted. The link is encrypted, but anyone in \
                         the path can pose as the server and read the password.",
                        RED,
                    ),
                }
            } else {
                note(
                    ui,
                    "⚠ Plain FTP sends the password and the pictures unencrypted.",
                    crate::theme::YELLOW(),
                );
            }

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.add_sized([96.0, 22.0], egui::Label::new(""));
                crate::chrome::checkbox(ui, &mut edit.upload_sstv, "SSTV");
            });
            ui.add_enabled_ui(edit.upload_sstv, |ui| {
                net_row(ui, "SSTV dir", &mut edit.sstv_dir, 200.0);
            });
            ui.horizontal(|ui| {
                ui.add_sized([96.0, 22.0], egui::Label::new(""));
                crate::chrome::checkbox(ui, &mut edit.upload_wefax, "Weather fax");
            });
            ui.add_enabled_ui(edit.upload_wefax, |ui| {
                net_row(ui, "WEFAX dir", &mut edit.wefax_dir, 200.0);
            });
            note(ui, "Empty is the login directory.", crate::theme::gray(140));

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.add_sized([96.0, 22.0], egui::Label::new("File names"));
                ComboBox::from_id_salt("picup_naming")
                    .selected_text(edit.naming.label())
                    .show_styled(ui, |ui| {
                        for n in UploadNaming::ALL {
                            ui.selectable_value(&mut edit.naming, n, n.label());
                        }
                    });
                if edit.naming == UploadNaming::Sequence {
                    ui.label("keep");
                    ui.add(egui::DragValue::new(&mut edit.keep_images).range(1..=999));
                }
            });
            ui.horizontal(|ui| {
                ui.add_sized([96.0, 22.0], egui::Label::new("Format"));
                ComboBox::from_id_salt("picup_format")
                    .selected_text(edit.format.label())
                    .show_styled(ui, |ui| {
                        for f in UploadFormat::ALL {
                            ui.selectable_value(&mut edit.format, f, f.label());
                        }
                    });
                if edit.naming == UploadNaming::Sequence {
                    crate::chrome::checkbox(ui, &mut edit.add_extension, "add extension")
                        .on_hover_text("image1.png rather than QSSTV's bare image1");
                }
            });
            ui.horizontal_wrapped(|ui| {
                ui.add_sized([96.0, 22.0], egui::Label::new(""));
                crate::chrome::checkbox(ui, &mut edit.stamp, "Caption bar").on_hover_text(
                    "Time, frequency, mode and the sender's FSK ID across the top, as QSSTV \
                     draws it. Only the uploaded copy is marked.",
                );
                crate::chrome::checkbox(ui, &mut edit.downscale, "Fit 960 × 768");
                crate::chrome::checkbox(ui, &mut edit.heartbeat, "checknew.jpg").on_hover_text(
                    "QSSTV's heartbeat: a tiny picture whose height is the minute, uploaded \
                     beside each one, and made two pixels wide when the program closes.",
                );
                crate::chrome::checkbox(ui, &mut edit.passive_nat_workaround, "NAT workaround")
                    .on_hover_text(
                        "Ignore a private address in the server's passive-mode reply and use \
                         the server's own — for a server behind NAT that does not know its \
                         public address.",
                    );
            });
        });

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if crate::chrome::chip_accent(
                ui,
                false,
                RichText::new(" APPLY ").strong(),
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
                egui::Button::new(RichText::new("TEST").size(11.0)),
            );
            if test
                .on_hover_text(
                    "Connect with the settings above — applied or not — log in, and write and \
                     delete a small file in each upload directory.",
                )
                .clicked()
            {
                cmds.push(Command::TestPictureUpload(Box::new(edit.clone())));
            }
            if waiting {
                ui.label(RichText::new("checking…").size(11.0).weak());
            }
        });
        if let Some(st) = self.picup_test.as_ref().filter(|_| self.picup_test_pending.is_none()) {
            result_line(ui, st);
            if let Some(fp) = st.fingerprint.as_deref() {
                let pinned = edit.verify == TlsVerify::Pinned
                    && sdroxide_types::normalize_fingerprint(&edit.pinned_sha256) == fp;
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(format!(
                            "Server certificate SHA-256: {}",
                            format_fingerprint(fp)
                        ))
                        .size(10.5)
                        .monospace(),
                    );
                    if !pinned
                        && ui
                            .small_button("Trust this certificate")
                            .on_hover_text(
                                "Pin it: switch to \"Pinned fingerprint\" with this one. Check \
                                 it against the server first — this is what the server said, \
                                 not proof of who it is. Then APPLY.",
                            )
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
                ui.label(RichText::new("Last upload").size(11.0).weak());
                result_line(ui, st);
            });
        }
    }
}
