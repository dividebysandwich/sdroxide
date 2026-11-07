//! The MAIL window: Winlink radio email.
//!
//! Modelled on the logbook — an in-root `egui::Window` with a salted id, so a
//! split view can have one per radio — rather than on the 3D map's separate
//! viewport. A mailbox is a searchable record store, which is what the logbook
//! already is.
//!
//! The window holds no mailbox of its own. It asks the engine for a page of a
//! folder and for one message at a time, exactly as the picture gallery does,
//! because on a remote client the mail lives on the machine with the radio and
//! may have attachments in it.

use crate::i18n::{Tr, tr};
use eframe::egui;

use crate::app::panels::widgets::row_cell;
use sdroxide_types::{
    Command, MailDraft, MailEntry, MailFolder, MailListing, MailMessage, WinlinkStatus,
};

/// How many rows to ask for at a time.
const PAGE: u32 = 100;

/// Everything the window remembers between frames.
#[derive(Default)]
pub struct MailUi {
    pub open: bool,
    folder: MailFolder,
    listing: Option<MailListing>,
    /// The message being read, if any.
    selected: Option<Box<MailMessage>>,
    /// Set while a fetch is outstanding, so the pane can say so rather than
    /// showing the previous message as though it were the one clicked.
    loading: Option<String>,
    status: WinlinkStatus,
    /// The compose pane, when open.
    draft: Option<MailDraft>,
    compose_to: String,
    compose_cc: String,
    /// The session-transcript window, which is its own window rather than a
    /// mode of this one: reading the log while working through messages is the
    /// point of having it.
    pub log_open: bool,
    /// Where the draggable divider sits, as a fraction of the body height.
    /// Clamped on use so neither pane can be dragged away entirely.
    split: f32,
    /// Set when the folder needs re-listing — on open, after a session, after
    /// a delete. Cleared by the request that goes out.
    dirty: bool,
    /// Last frame's `open`, so the listing can be asked for on the rising edge.
    /// Without this the window opens showing "loading…" for ever: nothing else
    /// sets `dirty`, so the request is never made.
    was_open: bool,
}

impl MailUi {
    /// Starting divider position, and the range it may be dragged through.
    const SPLIT_DEFAULT: f32 = 0.45;
    const SPLIT_MIN: f32 = 0.12;
    const SPLIT_MAX: f32 = 0.85;

    pub fn on_status(&mut self, status: WinlinkStatus) {
        // A session that has just finished may have filed new mail, so the
        // open folder is stale.
        if self.status.busy && !status.busy {
            self.dirty = true;
        }
        self.status = status;
    }

    pub fn on_listing(&mut self, listing: MailListing) {
        if listing.folder == self.folder {
            self.listing = Some(listing);
        }
    }

    pub fn on_message(&mut self, msg: Box<MailMessage>) {
        if self.loading.as_deref() == Some(msg.mid.as_str()) {
            self.loading = None;
        }
        self.selected = Some(msg);
    }

    pub fn on_saved(&mut self, _mid: String) {
        self.draft = None;
        self.compose_to.clear();
        self.compose_cc.clear();
        self.dirty = true;
    }

    pub fn on_deleted(&mut self, folder: MailFolder, mid: &str) {
        if self.selected.as_ref().is_some_and(|m| m.mid == mid) {
            self.selected = None;
        }
        if folder == self.folder {
            self.dirty = true;
        }
    }

    fn select_folder(&mut self, folder: MailFolder) {
        if self.folder != folder {
            self.folder = folder;
            self.listing = None;
            self.selected = None;
            self.loading = None;
            self.dirty = true;
        }
    }
}

impl crate::app::SdroxideApp {
    pub(in crate::app) fn mail_window(&mut self, ctx: &egui::Context, cmds: &mut Vec<Command>) {
        // Opening the window is itself a reason to (re)list: the mailbox may
        // have changed since it was last closed, and on the very first open
        // there is nothing to show at all.
        if self.mail.open && !self.mail.was_open {
            self.mail.dirty = true;
        }
        self.mail.was_open = self.mail.open;

        if !self.mail.open {
            return;
        }

        // Ask for the folder we are showing, once per change rather than per
        // frame — a listing request walks the mailbox on the engine host.
        if self.mail.dirty {
            self.mail.dirty = false;
            cmds.push(Command::MailList { folder: self.mail.folder, offset: 0, count: PAGE });
        }

        // An `egui::Window` shrinks to its content, so `default_height` alone
        // leaves an empty inbox drawn as a couple of lines. `min_height` keeps
        // a floor, and `set_min_height` inside makes the content actually ask
        // for the room — without that the window collapses again the moment a
        // folder is empty.
        let min_h = crate::layout::window_h(ctx, 520.0);
        let mut open = self.mail.open;
        let tool_title = tr!("winlink-mail");
        let mut tool_st = self.take_tool_state("MAIL");
        let tool_child = crate::app::undocked::ChildWindow::own(self);
        crate::app::undocked::tool_window(
            ctx,
            crate::app::undocked::ToolWindow::new("MAIL", &tool_title, [860.0, 640.0])
                .min_size([420.0, 520.0]),
            &mut tool_st,
            tool_child,
            &mut open,
            |ui| {
                ui.set_min_height(min_h);
                self.mail_body(ui, cmds);
            },
        );
        self.put_tool_state("MAIL", tool_st);
        self.mail.open = open;
    }

    fn mail_body(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        self.mail_header(ui, cmds);
        ui.separator();

        // Whatever is left under the header. In an auto-sizing window this can
        // come back as zero or infinite, so it is clamped rather than trusted.
        let avail = ui.available_height();
        let body_h = if avail.is_finite() && avail > 1.0 { avail } else { 420.0 };

        if self.mail.draft.is_some() {
            self.mail_compose(ui, cmds);
            return;
        }

        // List above, message below, with a draggable divider between them.
        // One column rather than two, so the window stays usable at the width
        // a browser client on a laptop actually gets.
        let split = self.mail.split;
        let split = if split <= 0.0 { MailUi::SPLIT_DEFAULT } else { split };
        let list_h = (body_h * split).clamp(80.0, (body_h - 80.0).max(80.0));

        egui::ScrollArea::vertical()
            .id_salt("mail-list")
            .min_scrolled_height(list_h)
            .max_height(list_h)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                self.mail_list(ui, cmds);
            });

        self.mail_divider(ui, body_h);

        egui::ScrollArea::vertical().id_salt("mail-body").auto_shrink([false, false]).show(
            ui,
            |ui| {
                self.mail_message(ui, cmds);
            },
        );
    }

    /// The draggable divider between the list and the message.
    ///
    /// A plain `ui.separator()` is a hairline nobody can grab, so this
    /// allocates a band tall enough to hit, shows a resize cursor over it, and
    /// converts the drag into a fraction of the body height. Fractions rather
    /// than pixels so the split survives the window being resized.
    fn mail_divider(&mut self, ui: &mut egui::Ui, body_h: f32) {
        const BAND: f32 = 7.0;
        let (rect, resp) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), BAND), egui::Sense::drag());

        if resp.hovered() || resp.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
        }
        if resp.dragged() && body_h > 1.0 {
            let split =
                if self.mail.split <= 0.0 { MailUi::SPLIT_DEFAULT } else { self.mail.split };
            self.mail.split =
                (split + resp.drag_delta().y / body_h).clamp(MailUi::SPLIT_MIN, MailUi::SPLIT_MAX);
        }
        if resp.double_clicked() {
            self.mail.split = MailUi::SPLIT_DEFAULT;
        }

        // A line with a grip in the middle, brighter while it is being used, so
        // it reads as something to grab rather than as decoration.
        let lit = resp.hovered() || resp.dragged();
        let colour = if lit { crate::theme::CYAN() } else { crate::theme::LINE() };
        let y = rect.center().y;
        ui.painter().hline(rect.x_range(), y, egui::Stroke::new(1.0, colour));
        let grip = 26.0;
        ui.painter().hline(
            (rect.center().x - grip)..=(rect.center().x + grip),
            y,
            egui::Stroke::new(3.0, colour),
        );
    }

    fn mail_header(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        // The folders are tabs — each opens a different listing below — so they
        // are drawn as tabs, and the session buttons that follow stay buttons.
        crate::chrome::tab_bar(ui, |ui, bar| {
            for folder in MailFolder::ALL {
                let count = self.mail.status.counts[folder as usize];
                let label = if count > 0 {
                    format!("{} {count}", folder.tr())
                } else {
                    folder.tr().to_string()
                };
                if bar.tab(ui, self.mail.folder == folder, label).clicked() {
                    self.mail.select_folder(folder);
                }
            }

            bar.end_tabs(ui);

            let busy = self.mail.status.busy;
            // One button, two jobs. A session that cannot be stopped is a
            // session the operator watches for two minutes wondering whether
            // anything is happening — and a greyed-out CONNECT says nothing
            // about how to get out of it.
            if busy {
                if ui
                    .button(tr!("winlink-abort"))
                    .on_hover_text(tr!("winlink-stop-session-progress"))
                    .clicked()
                {
                    cmds.push(Command::WinlinkAbort);
                }
            } else if ui.button(tr!("winlink-connect")).clicked() {
                cmds.push(Command::WinlinkConnect);
            }
            if ui.add_enabled(!busy, egui::Button::new(tr!("winlink-compose"))).clicked() {
                self.mail.draft = Some(MailDraft::default());
            }
            if ui.selectable_label(self.mail.log_open, "LOG").clicked() {
                self.mail.log_open = !self.mail.log_open;
            }
        });

        // One status line: what it is doing, or what went wrong last time.
        if self.mail.status.busy {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(if self.mail.status.activity.is_empty() {
                    tr!("winlink-connecting")
                } else {
                    self.mail.status.activity.clone()
                });
            });
        } else if let Some(err) = self.mail.status.last_error.clone() {
            // Errors persist until the next session rather than flashing past:
            // a forwarding failure is the thing the operator most needs to
            // read, and it is often the only clue to why no mail arrived.
            ui.colored_label(crate::theme::ALERT(), tr!("winlink-last-session-failed", err = err));
        } else if self.mail.status.last_session.is_some() {
            ui.label(tr!(
                "winlink-last-session-received-sent",
                last_received = self.mail.status.last_received,
                last_sent = self.mail.status.last_sent
            ));
        } else {
            ui.label(tr!("winlink-not-connected-yet"));
        }
    }

    fn mail_list(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        let Some(listing) = self.mail.listing.clone() else {
            ui.label(tr!("winlink-loading"));
            return;
        };
        if listing.entries.is_empty() {
            ui.label(tr!("winlink-empty", to_lowercase = self.mail.folder.tr().to_lowercase()));
            return;
        }

        // Column widths, in the decode list's manner: fixed for the fields that
        // line up down the page, with the subject taking whatever is left so
        // the table always fills the window's width.
        let full = ui.available_width();
        let date_w = 108.0;
        let who_w = (full * 0.22).clamp(90.0, 200.0);
        let att_w = 26.0;
        let btn_w = 108.0;
        let gaps = ui.spacing().item_spacing.x * 4.0;
        let subject_w = (full - date_w - who_w - att_w - btn_w - gaps - 18.0).max(80.0);

        self.mail_list_head(ui, date_w, who_w, att_w, subject_w);

        for (i, entry) in listing.entries.iter().enumerate() {
            self.mail_row(ui, i, entry, date_w, who_w, att_w, subject_w, cmds);
        }

        if listing.total as usize > listing.entries.len() {
            ui.label(
                egui::RichText::new(tr!(
                    "winlink-showing",
                    entries = listing.entries.len(),
                    total = listing.total
                ))
                .size(11.0)
                .color(crate::theme::TEXT()),
            );
        }
    }

    /// The column headings, dim and small so they frame the rows without
    /// competing with them.
    fn mail_list_head(
        &self,
        ui: &mut egui::Ui,
        date_w: f32,
        who_w: f32,
        att_w: f32,
        subject_w: f32,
    ) {
        let inbox = self.mail.folder == MailFolder::Inbox;
        let head = |text: &str| {
            egui::Label::new(
                egui::RichText::new(text).size(10.0).strong().color(crate::theme::CYAN_DIM()),
            )
            .selectable(false)
        };
        ui.horizontal(|ui| {
            // Matches the rows' frame margin, or the headings sit left of the
            // columns they name.
            ui.add_space(11.0);
            row_cell(ui, date_w, 16.0, false, head(&tr!("winlink-date")));
            row_cell(
                ui,
                who_w,
                16.0,
                false,
                head(&if inbox { tr!("winlink-from") } else { tr!("winlink-to-heading") }),
            );
            row_cell(ui, att_w, 16.0, false, head(""));
            row_cell(ui, subject_w, 16.0, false, head(&tr!("winlink-subject")));
        });
    }

    /// One message row: a full-width band the whole of which selects the
    /// message, with the archive/delete buttons pinned right.
    #[allow(clippy::too_many_arguments)]
    fn mail_row(
        &mut self,
        ui: &mut egui::Ui,
        index: usize,
        entry: &MailEntry,
        date_w: f32,
        who_w: f32,
        att_w: f32,
        subject_w: f32,
        cmds: &mut Vec<Command>,
    ) {
        let selected = self.mail.selected.as_ref().is_some_and(|m| m.mid == entry.mid);
        let inbox = self.mail.folder == MailFolder::Inbox;
        // Unread is only meaningful in the inbox, and only until it is opened.
        let unread = inbox && entry.unread && !selected;
        let row_h = 22.0;

        // Where the buttons ended up, so the row-body click can exclude them.
        let mut buttons_left = None;

        let inner = egui::Frame::new()
            .fill(if selected { crate::theme::TOME_BG() } else { crate::theme::ROW_BG() })
            .inner_margin(egui::Margin { left: 10, right: 6, top: 3, bottom: 3 })
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let dim = crate::theme::TEXT();
                    let strong = crate::theme::TEXT_STRONG();

                    // Date: dim and monospace, so the column reads as a ruler
                    // down the side rather than as content.
                    row_cell(
                        ui,
                        date_w,
                        row_h,
                        false,
                        egui::Label::new(
                            egui::RichText::new(&entry.date).monospace().size(11.0).color(dim),
                        )
                        .selectable(false),
                    );

                    // Correspondent: the identity, so it carries the accent
                    // colour and the weight.
                    let who = if inbox { &entry.from } else { &entry.to };
                    let who = who.strip_prefix("SMTP:").unwrap_or(who);
                    row_cell(
                        ui,
                        who_w,
                        row_h,
                        false,
                        egui::Label::new(
                            egui::RichText::new(who)
                                .size(12.0)
                                .strong()
                                .color(crate::theme::CYAN()),
                        )
                        .truncate()
                        .selectable(false),
                    );

                    // Attachment marker, its own narrow column so subjects
                    // still line up whether or not there is one.
                    // The attachment count rather than a paperclip: this font
                    // has no glyph for one and draws a tofu box, and the number
                    // says more than the icon did anyway.
                    row_cell(
                        ui,
                        att_w,
                        row_h,
                        // Right-aligned so it tucks against the subject it
                        // belongs to rather than floating mid-row.
                        true,
                        egui::Label::new(if entry.attachments > 0 {
                            egui::RichText::new(entry.attachments.to_string())
                                .size(11.0)
                                .strong()
                                .color(crate::theme::YELLOW())
                        } else {
                            egui::RichText::new("")
                        })
                        .selectable(false),
                    );

                    let subject = if entry.subject.is_empty() {
                        egui::RichText::new(tr!("winlink-no-subject"))
                            .size(12.0)
                            .italics()
                            .color(dim)
                    } else {
                        // Unread stands out by weight and brightness; read mail
                        // recedes. Colour alone would not survive a colourblind
                        // operator or a dim screen.
                        let t = egui::RichText::new(&entry.subject).size(12.0);
                        if unread { t.strong().color(strong) } else { t.color(dim) }
                    };
                    row_cell(
                        ui,
                        subject_w,
                        row_h,
                        false,
                        egui::Label::new(subject).truncate().selectable(false),
                    );

                    // Pinned right. Allocated inside the row so they take their
                    // clicks before the row-body interaction below sees them.
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let left = ui.cursor().max.x;
                        if ui
                            .small_button("DEL")
                            .on_hover_text(tr!("winlink-delete-message"))
                            .clicked()
                        {
                            cmds.push(Command::MailDelete {
                                folder: self.mail.folder,
                                mid: entry.mid.clone(),
                            });
                        }
                        if self.mail.folder != MailFolder::Archive
                            && ui
                                .small_button(tr!("winlink-arch"))
                                .on_hover_text(tr!("winlink-move-archive"))
                                .clicked()
                        {
                            cmds.push(Command::MailMove {
                                from: self.mail.folder,
                                to: MailFolder::Archive,
                                mid: entry.mid.clone(),
                            });
                        }
                        buttons_left = Some(ui.min_rect().left());
                        let _ = left;
                    });
                });
            });

        let r = inner.response.rect;

        // Left accent bar: gold for unread, cyan for the open message, dim
        // otherwise — the same vocabulary the decode list uses.
        let (accent, aw) = if unread {
            (crate::theme::YELLOW(), 3.5)
        } else if selected {
            (crate::theme::CYAN(), 3.5)
        } else {
            (crate::theme::CYAN_DIM(), 2.0)
        };
        ui.painter().rect_filled(
            egui::Rect::from_min_max(r.left_top(), egui::pos2(r.left() + aw, r.bottom())),
            0.0,
            accent,
        );

        // The row body — everything left of the buttons — opens the message.
        // Excluding the buttons' rect is what keeps this interaction from
        // covering and stealing their clicks.
        let body_right = buttons_left.map(|x| x - 2.0).unwrap_or(r.right());
        let body = egui::Rect::from_min_max(r.left_top(), egui::pos2(body_right, r.bottom()));
        let resp = ui.interact(body, ui.id().with(("mail-row", index)), egui::Sense::click());
        if resp.hovered() && !selected {
            ui.painter().rect_filled(r, 0.0, crate::theme::ROW_HOVER().gamma_multiply(0.5));
        }
        if resp.clicked() && !selected {
            self.mail.loading = Some(entry.mid.clone());
            self.mail.selected = None;
            cmds.push(Command::MailGet { folder: self.mail.folder, mid: entry.mid.clone() });
        }
    }

    fn mail_message(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        if let Some(mid) = self.mail.loading.clone() {
            ui.label(tr!("winlink-fetching", mid = mid));
            return;
        }
        let Some(msg) = self.mail.selected.clone() else {
            ui.label(tr!("winlink-select-message"));
            return;
        };

        ui.heading(if msg.subject.is_empty() {
            tr!("winlink-no-subject")
        } else {
            msg.subject.clone()
        });
        ui.label(tr!("winlink-from-2", msg = msg.from, date = msg.date));
        ui.label(tr!("winlink-to-list", to = msg.to.join(", ")));
        if !msg.cc.is_empty() {
            ui.label(tr!("winlink-cc-list", cc = msg.cc.join(", ")));
        }

        if !msg.attachments.is_empty() {
            ui.separator();
            for att in &msg.attachments {
                // Attachments are shown, not saved: the browser client cannot
                // write to the operator's disk, and the native one would be
                // writing somewhere it has not asked about. Naming them is
                // enough to know the message arrived whole.
                ui.label(tr!("winlink-attachment-bytes", att = att.name, data = att.data.len()));
            }
        }

        ui.separator();
        ui.horizontal(|ui| {
            if ui.button(tr!("winlink-reply")).clicked() {
                let mut subject = msg.subject.clone();
                if !subject.to_ascii_uppercase().starts_with("RE:") {
                    subject = format!("RE: {subject}");
                }
                self.mail.compose_to = msg.from.clone();
                self.mail.compose_cc.clear();
                self.mail.draft = Some(MailDraft {
                    to: vec![msg.from.clone()],
                    cc: vec![],
                    subject,
                    // Quote the original, so a reply over a slow link still
                    // carries the context the recipient needs.
                    body: msg.body.lines().map(|l| format!("> {l}\r\n")).collect(),
                    attachments: vec![],
                });
            }
            if ui.button(tr!("winlink-delete")).clicked() {
                cmds.push(Command::MailDelete { folder: msg.folder, mid: msg.mid.clone() });
            }
        });

        ui.separator();
        crate::chrome::field(
            ui,
            egui::TextEdit::multiline(&mut msg.body.as_str())
                .desired_width(f32::INFINITY)
                .font(egui::TextStyle::Monospace),
        );
    }

    fn mail_compose(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        let Some(mut draft) = self.mail.draft.clone() else { return };

        egui::Grid::new("mail-compose").num_columns(2).show(ui, |ui| {
            ui.label(tr!("winlink-to"));
            crate::chrome::field(ui, egui::TextEdit::singleline(&mut self.mail.compose_to));
            ui.end_row();
            ui.label(tr!("winlink-cc"));
            crate::chrome::field(ui, egui::TextEdit::singleline(&mut self.mail.compose_cc));
            ui.end_row();
            ui.label(tr!("winlink-subject-2"));
            crate::chrome::field(ui, egui::TextEdit::singleline(&mut draft.subject));
            ui.end_row();
        });
        ui.label(tr!("winlink-callsign-smtp-someone-example-org"));

        ui.separator();
        crate::chrome::field(
            ui,
            egui::TextEdit::multiline(&mut draft.body)
                .desired_width(f32::INFINITY)
                .desired_rows(12)
                .font(egui::TextStyle::Monospace),
        );

        ui.separator();
        ui.horizontal(|ui| {
            let to = split_addresses(&self.mail.compose_to);
            if ui
                .add_enabled(!to.is_empty(), egui::Button::new(tr!("winlink-file-outbox")))
                .clicked()
            {
                draft.to = to;
                draft.cc = split_addresses(&self.mail.compose_cc);
                cmds.push(Command::MailCompose(Box::new(draft.clone())));
            }
            if ui.button(tr!("winlink-discard")).clicked() {
                self.mail.draft = None;
                self.mail.compose_to.clear();
                self.mail.compose_cc.clear();
                return;
            }
            ui.label(tr!("winlink-filed-messages-go-out-next"));
        });

        if self.mail.draft.is_some() {
            self.mail.draft = Some(draft);
        }
    }

    /// The session transcript, in a window of its own.
    ///
    /// Separate rather than a mode of the mail window on purpose: watching a
    /// forwarding session while still reading and composing messages is the
    /// whole reason to have it open. It follows the live status, so it updates
    /// as the session runs without anything having to poll it.
    pub(in crate::app) fn mail_log_window(&mut self, ctx: &egui::Context) {
        if !self.mail.log_open {
            return;
        }

        let mut open = self.mail.log_open;
        let tool_title = tr!("winlink-mail-log");
        let mut tool_st = self.take_tool_state("MAIL-LOG");
        let tool_child = crate::app::undocked::ChildWindow::own(self);
        crate::app::undocked::tool_window(
            ctx,
            crate::app::undocked::ToolWindow::new("MAIL-LOG", &tool_title, [560.0, 420.0])
                .min_size([320.0, 200.0]),
            &mut tool_st,
            tool_child,
            &mut open,
            |ui| {
                ui.set_min_height(crate::layout::window_h(ctx, 200.0));

                if self.mail.status.busy {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(tr!("winlink-session-running"));
                    });
                    // Keep the transcript moving while a session is live; there
                    // is no other repaint source when the radio is idle.
                    crate::repaint::after_ms(ui.ctx(), 250);
                    ui.separator();
                }

                if self.mail.status.log.is_empty() {
                    ui.label(tr!("winlink-no-session-yet"));
                    return;
                }

                egui::ScrollArea::vertical()
                    .id_salt("mail-log")
                    .auto_shrink([false, false])
                    // Follow the tail while a session is running, so the newest
                    // lines stay in view without the operator chasing them.
                    .stick_to_bottom(self.mail.status.busy)
                    .show(ui, |ui| {
                        for line in &self.mail.status.log {
                            // Direction colouring: what we said versus what the
                            // peer said, which is the first thing you need when
                            // reading a failed session.
                            let colour = if line.starts_with('>') {
                                crate::theme::CYAN()
                            } else if line.starts_with("< ***") {
                                crate::theme::ALERT()
                            } else {
                                crate::theme::TEXT()
                            };
                            ui.label(
                                egui::RichText::new(line).monospace().size(11.0).color(colour),
                            );
                        }
                    });
            },
        );
        self.put_tool_state("MAIL-LOG", tool_st);
        self.mail.log_open = open;
    }
}

/// Split a comma- or semicolon-separated address list, dropping blanks.
fn split_addresses(text: &str) -> Vec<String> {
    text.split([',', ';']).map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_address_lists() {
        assert_eq!(split_addresses("OE1XYZ"), ["OE1XYZ"]);
        assert_eq!(split_addresses(" OE1XYZ , OE3ABC ;"), ["OE1XYZ", "OE3ABC"]);
        assert_eq!(split_addresses("SMTP:a@b.org"), ["SMTP:a@b.org"]);
        assert!(split_addresses("  ,  ; ").is_empty());
    }

    #[test]
    fn changing_folder_clears_the_previous_view() {
        let mut ui = MailUi { folder: MailFolder::Inbox, ..Default::default() };
        ui.listing = Some(MailListing { folder: MailFolder::Inbox, ..Default::default() });
        ui.selected = Some(Box::new(MailMessage::default()));

        ui.select_folder(MailFolder::Sent);
        // Stale rows from the old folder must not linger while the new listing
        // is in flight.
        assert!(ui.listing.is_none());
        assert!(ui.selected.is_none());
        assert!(ui.dirty);
    }

    #[test]
    fn a_listing_for_another_folder_is_ignored() {
        // Two requests can be outstanding after fast tab clicks; the late one
        // must not repaint the folder the operator is now looking at.
        let mut ui = MailUi { folder: MailFolder::Sent, ..Default::default() };
        ui.on_listing(MailListing { folder: MailFolder::Inbox, total: 9, ..Default::default() });
        assert!(ui.listing.is_none());
        ui.on_listing(MailListing { folder: MailFolder::Sent, total: 3, ..Default::default() });
        assert_eq!(ui.listing.unwrap().total, 3);
    }

    #[test]
    fn a_finished_session_marks_the_folder_stale() {
        let mut ui = MailUi::default();
        ui.on_status(WinlinkStatus { busy: true, ..Default::default() });
        ui.dirty = false;
        ui.on_status(WinlinkStatus { busy: false, ..Default::default() });
        assert!(ui.dirty, "new mail may have arrived, so the listing must refresh");
    }

    #[test]
    fn opening_the_window_asks_for_a_listing() {
        // The rising edge is the only thing that triggers the first request;
        // without it the window shows "loading…" for ever.
        let mut ui = MailUi::default();
        assert!(!ui.dirty);
        ui.open = true;
        // Mirrors the edge check in `mail_window`.
        if ui.open && !ui.was_open {
            ui.dirty = true;
        }
        ui.was_open = ui.open;
        assert!(ui.dirty, "opening must request a listing");

        // And it does not re-request on every subsequent frame.
        ui.dirty = false;
        if ui.open && !ui.was_open {
            ui.dirty = true;
        }
        assert!(!ui.dirty, "an already-open window must not re-request each frame");
    }

    #[test]
    fn the_divider_starts_at_the_default_and_stays_in_range() {
        // `split` is zero on a fresh `MailUi`, which must read as "use the
        // default" rather than "collapse the list to nothing".
        let ui = MailUi::default();
        assert_eq!(ui.split, 0.0);
        let effective = if ui.split <= 0.0 { MailUi::SPLIT_DEFAULT } else { ui.split };
        assert!((MailUi::SPLIT_MIN..=MailUi::SPLIT_MAX).contains(&effective));
    }

    #[test]
    fn dragging_the_divider_cannot_collapse_either_pane() {
        // Mirrors the clamp in `mail_divider`: whatever the drag, both panes
        // keep a share of the body.
        let clamp = |v: f32| v.clamp(MailUi::SPLIT_MIN, MailUi::SPLIT_MAX);
        for drag in [-10.0f32, -1.0, -0.001, 0.0, 0.001, 1.0, 10.0] {
            let split = clamp(MailUi::SPLIT_DEFAULT + drag);
            assert!(split >= MailUi::SPLIT_MIN, "list collapsed at drag {drag}");
            assert!(split <= MailUi::SPLIT_MAX, "message pane collapsed at drag {drag}");
        }
    }

    #[test]
    fn the_log_is_a_separate_window_from_the_mailbox() {
        // The point of the change: the transcript can be open while the
        // operator carries on reading mail, so the two flags are independent.
        let mut ui = MailUi { open: true, ..Default::default() };
        ui.log_open = true;
        assert!(ui.open && ui.log_open);
        ui.open = false;
        assert!(ui.log_open, "closing the mailbox must not close the log");
    }

    #[test]
    fn deleting_the_open_message_closes_it() {
        let mut ui = MailUi {
            selected: Some(Box::new(MailMessage { mid: "ABC".into(), ..Default::default() })),
            ..Default::default()
        };
        ui.on_deleted(MailFolder::Inbox, "ABC");
        assert!(ui.selected.is_none());
    }
}
