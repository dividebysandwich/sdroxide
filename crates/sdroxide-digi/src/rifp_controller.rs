//! `RifpController` — the session layer of RIFP (draft-dulaunoy-rifp-00).
//!
//! Between the modem below it (`sdroxide_dsp::rifp`) and the panel above it,
//! this owns everything the draft calls reassembly: it collects DATA chunks by
//! Session ID, holds them until a MANIFEST explains them, verifies the complete
//! object's size, CRC-32 and SHA-256 before believing any of it, and decodes
//! the result into a picture. Transmit is the mirror image — encode, chunk,
//! manifest, repeat, end.
//!
//! The draft is blunt about the threat model, and so is this: "a receiver
//! should assume that duplicate, reordered, conflicting, and truncated frames
//! are normal hostile inputs rather than exceptional conditions" (§12).

use std::time::SystemTime;

use sdroxide_dsp::rifp::{
    self, FLAG_RETRANSMISSION, FRAME_CANCEL, FRAME_DATA, FRAME_END, FRAME_MANIFEST, RifpFrame,
    RifpRx, RifpTx, TLV_CONTENT_HINT, TLV_RADIO_PROFILE, TLV_SENDER_ID, Tlv,
};
use sdroxide_types::{
    DigiConfig, DigiStatus, Mode, QsoStep, RIFP_MAP_MAX_CHUNKS, RifpEncoding, RifpMeta,
    RifpProfile, RifpSession, RifpStatus, TranscriptLine,
};

use crate::DigiEngine;
use crate::controller::DigiAction;
use crate::rifp_object::{self, MAX_OBJECT_BYTES, Manifest};

/// Transmit sample rate: the engine hands `fill_tx_block` 10 ms blocks at
/// 48 kHz, which is exactly ten samples per 4800-baud symbol.
const OUT_RATE: f64 = 48_000.0;

/// Most incomplete incoming transfers held at once (§8: limit the number of
/// concurrent sessions).
const MAX_SESSIONS: usize = 8;

/// Most octets held across all incomplete transfers.
const MAX_BUFFERED: usize = 16 * 1024 * 1024;

/// One incoming transfer being reassembled.
struct RxSession {
    id: u64,
    /// Identity for the progressive-paint events, so the panel can tell one
    /// picture's rows from another's.
    image_id: u32,
    manifest: Option<Manifest>,
    /// Total Count as last seen in a header — known from the first frame, even
    /// before the manifest arrives.
    total: u32,
    chunks: Vec<Option<Vec<u8>>>,
    have: u32,
    /// Chunks that arrived without the RETRANSMISSION flag, i.e. were not
    /// needed twice. A plain measure of how the path is doing.
    first_pass: u32,
    sender: Option<String>,
    hint: Option<String>,
    last_seen: SystemTime,
    /// Raster rows already sent to the panel.
    rows_sent: Vec<bool>,
    /// Set when two valid frames disagreed about the same content: §6.1 and
    /// §6.2 both say to stop combining rather than to guess.
    quarantined: bool,
    completed: bool,
}

impl RxSession {
    fn buffered(&self) -> usize {
        self.chunks.iter().flatten().map(|c| c.len()).sum()
    }

    /// The chunk map the panel draws, one bit per sequence number.
    fn map(&self) -> Vec<u8> {
        let n = self.chunks.len().min(RIFP_MAP_MAX_CHUNKS);
        let mut map = vec![0u8; n.div_ceil(8)];
        for (i, chunk) in self.chunks.iter().take(n).enumerate() {
            if chunk.is_some() {
                map[i / 8] |= 1 << (i % 8);
            }
        }
        map
    }
}

pub struct RifpController {
    cfg: DigiConfig,

    // RX
    rx: RifpRx,
    rx_scratch: Vec<RifpFrame>,
    sessions: Vec<RxSession>,
    /// Sessions already finished with, and when. A sender repeats its data
    /// frames — and its END — after the last chunk we needed, and every one of
    /// those would otherwise open a fresh half-empty session for a picture
    /// already on screen.
    done: Vec<(u64, SystemTime)>,
    rx_frames: u32,
    rx_objects: u32,
    next_image_id: u32,

    // TX
    tx: Option<RifpTx>,
    keyed: bool,
    tx_encoding: Option<RifpEncoding>,
    tx_bytes: u32,

    last_error: Option<String>,
    queued: Vec<DigiAction>,
    status_dirty: bool,
    last_status: Option<SystemTime>,
    now: SystemTime,
}

impl RifpController {
    pub fn new(cfg: DigiConfig, tap_rate: f64) -> Self {
        RifpController {
            rx: RifpRx::new(tap_rate),
            cfg,
            rx_scratch: Vec::new(),
            sessions: Vec::new(),
            done: Vec::new(),
            rx_frames: 0,
            rx_objects: 0,
            next_image_id: 0,
            tx: None,
            keyed: false,
            tx_encoding: None,
            tx_bytes: 0,
            last_error: None,
            queued: Vec::new(),
            status_dirty: true,
            last_status: None,
            now: SystemTime::now(),
        }
    }

    fn profile(&self) -> RifpProfile {
        self.cfg.rifp_profile
    }

    fn fail(&mut self, message: String) {
        tracing::debug!(target: "rifp", "{message}");
        self.last_error = Some(message);
        self.status_dirty = true;
    }

    fn rifp_status(&self) -> RifpStatus {
        let (frame, frames) = self.tx.as_ref().map(|t| t.frame_position()).unwrap_or((0, 0));
        let mut sessions: Vec<RifpSession> = self
            .sessions
            .iter()
            .map(|s| RifpSession {
                session: format!("{:016x}", s.id),
                sender: s.sender.clone(),
                have_manifest: s.manifest.is_some(),
                have: s.have,
                total: s.total,
                map: s.map(),
                idle_s: self
                    .now
                    .duration_since(s.last_seen)
                    .map(|d| d.as_secs() as u32)
                    .unwrap_or(0),
            })
            .collect();
        sessions.sort_by_key(|s| s.idle_s);
        RifpStatus {
            profile: self.profile(),
            tx_active: self.keyed,
            tx_progress: self.tx.as_ref().map(|t| t.progress()).unwrap_or(0.0),
            tx_frame: frame,
            tx_frames: frames,
            tx_remaining_s: self.tx.as_ref().map(|t| t.remaining_s().ceil() as u32).unwrap_or(0),
            tx_encoding: self.tx_encoding,
            tx_bytes: self.tx_bytes,
            signal: self.rx.level(),
            rx_frames: self.rx_frames,
            rx_bad_frames: self.rx.bad_frames(),
            rx_objects: self.rx_objects,
            sessions,
            last_error: self.last_error.clone(),
        }
    }

    /// A minimal FT8-style status; RIFP state travels via
    /// [`DigiAction::RifpStatus`].
    fn digi_status(&self) -> DigiStatus {
        DigiStatus {
            mode: Mode::Rifp,
            step: QsoStep::Idle,
            dx_call: None,
            dx_grid: None,
            tx_next: self.keyed,
            tx_pending_msg: None,
            // RIFP is centred on the dial, not offset into a sideband.
            audio_hz: 0.0,
            tx_even: false,
            transmitting: self.keyed,
            tx_watchdog: false,
            transcript: Vec::<TranscriptLine>::new(),
            config: self.cfg.clone(),
            text_rx: String::new(),
            tx_sent: 0,
            fsq_heard: Vec::new(),
            fsq_messages: Vec::new(),
            rade: None,
            js8: None,
            fox_queue: Vec::new(),
            call_queue: Vec::new(),
            clock_offset_s: None,
            cw: None,
            wspr: None,
        }
    }

    // ───────────────────────────── receive ─────────────────────────────

    fn on_frame(&mut self, frame: RifpFrame) {
        self.rx_frames = self.rx_frames.wrapping_add(1);
        self.status_dirty = true;

        if self.done.iter().any(|(id, _)| *id == frame.session_id) {
            // Already received and verified; the rest of the sender's repeats
            // are nothing to us.
            return;
        }
        if frame.frame_type == FRAME_CANCEL {
            // The sender says it has given up; drop what we were holding (§6.4).
            if let Some(i) = self.sessions.iter().position(|s| s.id == frame.session_id) {
                self.sessions.remove(i);
                self.fail(format!("session {:016x} cancelled by the sender", frame.session_id));
            }
            self.finish(frame.session_id);
            return;
        }
        if !matches!(frame.frame_type, FRAME_MANIFEST | FRAME_DATA | FRAME_END) {
            // §6.5: a validated frame of an unknown type is simply ignored, and
            // it does not invalidate the rest of the session.
            return;
        }

        let index = match self.sessions.iter().position(|s| s.id == frame.session_id) {
            Some(i) => i,
            None => {
                if frame.frame_type == FRAME_END {
                    // Nothing to complete; an END for a session we never saw is
                    // just a late arrival.
                    return;
                }
                self.open_session(&frame)
            }
        };

        {
            let session = &mut self.sessions[index];
            session.last_seen = self.now;
            if session.quarantined || session.completed {
                return;
            }
            if let Some(sender) = frame.text_tlv(TLV_SENDER_ID) {
                session.sender = Some(clip(sender, 32));
            }
            if let Some(hint) = frame.text_tlv(TLV_CONTENT_HINT) {
                session.hint = Some(clip(hint, 64));
            }
        }

        match frame.frame_type {
            FRAME_MANIFEST => self.on_manifest(index, &frame),
            FRAME_DATA => self.on_data(index, &frame),
            // §6.3: END repeats the manifest's integrity values and is a
            // completion hint, not a requirement. Everything it asserts is
            // checked at completion anyway, so it only prompts a re-check.
            _ => {}
        }
        self.try_complete(index);
        // A quarantined session is finished with too: §6.1 and §6.2 say not to
        // combine its frames, and re-admitting it on the next repeat would be
        // exactly that.
        let closed: Vec<u64> =
            self.sessions.iter().filter(|s| s.completed || s.quarantined).map(|s| s.id).collect();
        for id in closed {
            self.finish(id);
        }
    }

    /// Retire a session: drop its buffers and remember the ID so the sender's
    /// later repeats cannot resurrect it.
    fn finish(&mut self, id: u64) {
        self.sessions.retain(|s| s.id != id);
        self.done.retain(|(other, _)| *other != id);
        self.done.push((id, self.now));
        // Bounded, oldest first — the list is a courtesy, not a ledger.
        if self.done.len() > 32 {
            self.done.remove(0);
        }
    }

    fn open_session(&mut self, frame: &RifpFrame) -> usize {
        // Make room before making a session: oldest first, but never evict one
        // that is still being fed.
        while self.sessions.len() >= MAX_SESSIONS
            || self.sessions.iter().map(|s| s.buffered()).sum::<usize>() > MAX_BUFFERED
        {
            let Some(oldest) =
                self.sessions.iter().enumerate().min_by_key(|(_, s)| s.last_seen).map(|(i, _)| i)
            else {
                break;
            };
            let dropped = self.sessions.remove(oldest);
            self.fail(format!("dropped incomplete session {:016x}", dropped.id));
        }
        self.next_image_id = self.next_image_id.wrapping_add(1);
        // Total Count is not trusted yet: it sizes an allocation, so it is
        // capped here and checked against the manifest later.
        let total = frame.total.min(MAX_OBJECT_BYTES);
        self.sessions.push(RxSession {
            id: frame.session_id,
            image_id: self.next_image_id,
            manifest: None,
            total: frame.total,
            chunks: vec![None; total as usize],
            have: 0,
            first_pass: 0,
            sender: None,
            hint: None,
            last_seen: self.now,
            rows_sent: Vec::new(),
            quarantined: false,
            completed: false,
        });
        self.sessions.len() - 1
    }

    fn on_manifest(&mut self, index: usize, frame: &RifpFrame) {
        let manifest = match Manifest::parse(&frame.payload, frame.session_id, frame.total) {
            Ok(m) => m,
            Err(e) => {
                self.fail(format!("session {:016x}: {e}", frame.session_id));
                return;
            }
        };
        let session = &mut self.sessions[index];
        if let Some(existing) = &session.manifest {
            // §6.1: identical manifests are duplicates; conflicting ones mean
            // the chunks must not be combined.
            if existing.payload_sha256 != manifest.payload_sha256
                || existing.encoded_size != manifest.encoded_size
                || existing.chunk_count != manifest.chunk_count
            {
                session.quarantined = true;
                let id = session.id;
                self.fail(format!("session {id:016x}: conflicting manifests, abandoned"));
            }
            return;
        }
        if session.chunks.len() != manifest.chunk_count as usize {
            session.chunks.resize(manifest.chunk_count as usize, None);
            session.have = session.chunks.iter().flatten().count() as u32;
        }
        session.total = manifest.chunk_count;
        session.rows_sent = vec![false; manifest.height as usize];
        session.manifest = Some(manifest);
        self.status_dirty = true;
        // Chunks may have arrived before the manifest that explains them (§8),
        // so anything already held becomes paintable the moment it does.
        self.emit_rows(index, 0, u32::MAX);
    }

    fn on_data(&mut self, index: usize, frame: &RifpFrame) {
        let session = &mut self.sessions[index];
        // §6.2: ignore a DATA frame whose Sequence Number is at or past Total.
        if frame.sequence >= session.total || frame.sequence as usize >= session.chunks.len() {
            return;
        }
        let seq = frame.sequence as usize;
        match &session.chunks[seq] {
            Some(existing) => {
                if existing != &frame.payload {
                    // Two valid frames, same sequence, different contents:
                    // one of them is not from the station we think.
                    session.quarantined = true;
                    let id = session.id;
                    self.fail(format!(
                        "session {id:016x}: chunk {seq} arrived twice with different \
                         contents, abandoned"
                    ));
                }
                return;
            }
            None => {
                if frame.flags & FLAG_RETRANSMISSION == 0 {
                    session.first_pass += 1;
                }
                session.chunks[seq] = Some(frame.payload.clone());
                session.have += 1;
            }
        }
        self.status_dirty = true;
        self.emit_rows(index, seq as u32, seq as u32);
    }

    /// Paint whatever rows the chunks in `first..=last` just completed.
    ///
    /// Only the unencoded raster can be shown before the object is whole: a
    /// PNG, a JPEG or a facsimile page has no meaning until its last octet
    /// arrives, and RLE8 and ZLIB stop making sense at the first gap.
    fn emit_rows(&mut self, index: usize, first: u32, last: u32) {
        let session = &self.sessions[index];
        let Some(manifest) = &session.manifest else { return };
        if manifest.encoding() != Some(RifpEncoding::Raw) {
            return;
        }
        let (w, h, bpp) = (manifest.width, manifest.height, manifest.bits_per_pixel as u8);
        let chunk_size = manifest.chunk_size as u64;
        let row_bits = w as u64 * bpp as u64;
        if row_bits == 0 || chunk_size == 0 {
            return;
        }
        // Rows the touched chunks can possibly have completed.
        let byte_lo = first as u64 * chunk_size;
        let byte_hi = (last as u64 + 1) * chunk_size;
        let row_lo = (byte_lo * 8 / row_bits).saturating_sub(1);
        let row_hi = ((byte_hi * 8).div_ceil(row_bits) + 1).min(h as u64);

        let mut runs: Vec<(u32, Vec<u8>)> = Vec::new();
        for row in row_lo..row_hi {
            if session.rows_sent.get(row as usize) == Some(&true) {
                continue;
            }
            let lo = (row * row_bits / 8) as usize;
            let hi = ((row + 1) * row_bits).div_ceil(8) as usize;
            let (c_lo, c_hi) = (lo / chunk_size as usize, (hi - 1) / chunk_size as usize);
            if (c_lo..=c_hi).any(|c| session.chunks.get(c).is_none_or(|x| x.is_none())) {
                continue;
            }
            // Slice the row's octets out of the chunks that hold them.
            let mut raster = Vec::with_capacity(hi - lo);
            for c in c_lo..=c_hi {
                let chunk = session.chunks[c].as_ref().expect("checked above");
                let base = c * chunk_size as usize;
                let from = lo.saturating_sub(base);
                let to = (hi - base).min(chunk.len());
                if from < to {
                    raster.extend_from_slice(&chunk[from..to]);
                }
            }
            // The row's samples start part-way into the first octet whenever
            // the depth does not divide the width.
            let skip = (row * row_bits % 8) as usize / bpp.max(1) as usize;
            let Ok(pixels) = rifp_object::unpack_raster(
                &raster,
                (w as usize + skip).min(u16::MAX as usize) as u16,
                1,
                bpp,
            ) else {
                continue;
            };
            if pixels.len() < skip + w as usize {
                continue;
            }
            let gray = pixels[skip..skip + w as usize].to_vec();
            match runs.last_mut() {
                Some((y, rows)) if *y as u64 + (rows.len() / w as usize) as u64 == row => {
                    rows.extend_from_slice(&gray)
                }
                _ => runs.push((row as u32, gray)),
            }
        }
        if runs.is_empty() {
            return;
        }
        let session = &mut self.sessions[index];
        let image_id = session.image_id;
        for (y, rows) in &runs {
            for r in 0..(rows.len() / w as usize) {
                if let Some(sent) = session.rows_sent.get_mut(*y as usize + r) {
                    *sent = true;
                }
            }
        }
        for (y, rows) in runs {
            self.queued.push(DigiAction::RifpRows {
                image_id,
                y: y as u16,
                w: w as u16,
                h: h as u16,
                rows,
            });
        }
    }

    /// Complete the session if every chunk is in and the whole object checks
    /// out (§8). Nothing is believed until the digest agrees.
    fn try_complete(&mut self, index: usize) {
        let session = &self.sessions[index];
        if session.quarantined || session.completed {
            return;
        }
        let Some(manifest) = &session.manifest else { return };
        if session.have != manifest.chunk_count || session.chunks.iter().any(|c| c.is_none()) {
            return;
        }
        let object: Vec<u8> =
            session.chunks.iter().flatten().flat_map(|c| c.iter().copied()).collect();
        let id = session.id;

        if object.len() as u64 != manifest.encoded_size {
            let (got, want) = (object.len(), manifest.encoded_size);
            self.sessions[index].quarantined = true;
            self.fail(format!("session {id:016x}: reassembled {got} octets, manifest says {want}"));
            return;
        }
        if rifp_object::object_crc32(&object) != manifest.payload_crc32 {
            self.sessions[index].quarantined = true;
            self.fail(format!("session {id:016x}: object CRC-32 mismatch"));
            return;
        }
        if rifp_object::sha256_hex(&object) != manifest.payload_sha256.to_ascii_lowercase() {
            self.sessions[index].quarantined = true;
            self.fail(format!("session {id:016x}: object SHA-256 mismatch"));
            return;
        }

        let Some(encoding) = manifest.encoding() else {
            let (mt, ce) = (manifest.media_type.clone(), manifest.content_encoding.clone());
            self.sessions[index].quarantined = true;
            self.fail(format!("session {id:016x}: unsupported object type {mt} / {ce}"));
            return;
        };
        let (w, h, bpp) =
            (manifest.width as u16, manifest.height as u16, manifest.bits_per_pixel as u8);
        let gray = match rifp_object::decode_object(&object, encoding, w, h, bpp) {
            Ok(gray) => gray,
            Err(e) => {
                self.sessions[index].quarantined = true;
                self.fail(format!("session {id:016x}: {e}"));
                return;
            }
        };

        let meta = RifpMeta {
            session: format!("{id:016x}"),
            filename: manifest.safe_filename(),
            sender: session.sender.clone(),
            hint: session.hint.clone(),
            media_type: manifest.media_type.clone(),
            content_encoding: manifest.content_encoding.clone(),
            width: w,
            height: h,
            bits_per_pixel: bpp,
            encoded_size: manifest.encoded_size as u32,
            chunk_count: manifest.chunk_count,
            chunks_first_pass: session.first_pass,
        };
        let image_id = session.image_id;
        let rgb: Vec<u8> = gray.iter().flat_map(|&g| [g, g, g]).collect();
        self.queued.push(DigiAction::RifpImage { image_id, meta, w, h, rgb });
        self.sessions[index].completed = true;
        self.rx_objects = self.rx_objects.wrapping_add(1);
        self.last_error = None;
        self.status_dirty = true;
        tracing::info!(target: "rifp", session = format!("{id:016x}"), w, h, "picture received");
    }

    // ───────────────────────────── transmit ────────────────────────────

    /// Encode a composed picture and queue the whole transmission: manifests,
    /// data with repeats, and the end frame.
    fn build_transmission(&mut self, rgb: &[u8], w: u16, h: u16) -> Result<RifpTx, String> {
        let cfg = &self.cfg;
        let encoded = rifp_object::encode_object(
            rgb,
            w,
            h,
            cfg.rifp_encoding,
            cfg.rifp_bits_per_pixel,
            cfg.rifp_dither,
        )?;
        let (media_type, content_encoding) =
            encoded.encoding.manifest_pair().ok_or("no content encoding chosen")?;
        let chunk_size = cfg.rifp_chunk_size.clamp(16, 4096) as usize;
        let chunks: Vec<&[u8]> = encoded.bytes.chunks(chunk_size).collect();
        if chunks.is_empty() {
            return Err("the encoded picture is empty".into());
        }
        let session_id = rifp_object::new_session_id();
        let created = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| rifp_object::rfc3339_utc(d.as_secs() as i64))
            .ok();
        let profile = cfg.rifp_profile;
        let manifest = Manifest {
            protocol: "rifp".into(),
            protocol_version: format!("{}.{}", rifp::VERSION_MAJOR, rifp::VERSION_MINOR),
            session_id: format!("{session_id:016x}"),
            filename: format!("{}.{}", picture_stem(&cfg.my_call), file_suffix(encoded.encoding)),
            media_type: media_type.into(),
            content_encoding: content_encoding.into(),
            width: w as u32,
            height: h as u32,
            bits_per_pixel: encoded.bits_per_pixel as u32,
            encoded_size: encoded.bytes.len() as u64,
            payload_crc32: rifp_object::object_crc32(&encoded.bytes),
            payload_sha256: rifp_object::sha256_hex(&encoded.bytes),
            chunk_size: chunk_size as u32,
            chunk_count: chunks.len() as u32,
            raw_size: Some(encoded.raw_size as u64),
            created,
            radio_profile: Some(profile.name().into()),
            extensions: None,
        };
        let manifest_json = manifest.to_json();

        // Every frame carries the profile it was sent under, plus whatever the
        // operator chose to identify with.
        let mut tlvs = vec![Tlv::text(TLV_RADIO_PROFILE, profile.name())];
        let call = cfg.my_call.trim().to_uppercase();
        if cfg.rifp_send_sender_id && !call.is_empty() {
            tlvs.push(Tlv::text(TLV_SENDER_ID, &clip(&call, 24)));
        }
        let hint = cfg.rifp_content_hint.trim();
        if !hint.is_empty() {
            tlvs.push(Tlv::text(TLV_CONTENT_HINT, &clip(hint, 48)));
        }
        let header_len =
            rifp::BASE_HEADER_LEN + tlvs.iter().map(|t| 4 + t.value.len()).sum::<usize>();
        if header_len > rifp::MAX_HEADER_LEN {
            return Err("header extensions do not fit in 255 octets".into());
        }

        let total = chunks.len() as u32;
        let make = |frame_type: u8, sequence: u32, payload: Vec<u8>, repeat: bool| {
            let mut f = RifpFrame::new(frame_type, session_id, sequence, total, payload);
            f.flags = if repeat { FLAG_RETRANSMISSION } else { 0 };
            f.tlvs = tlvs.clone();
            f.air_frame()
        };

        let manifest_repeats = cfg.rifp_manifest_repeats.max(1);
        let data_repeats = cfg.rifp_data_repeats.max(1);
        let mut frames = Vec::new();
        for r in 0..manifest_repeats {
            frames.push(make(FRAME_MANIFEST, 0, manifest_json.clone(), r > 0));
        }
        for (seq, chunk) in chunks.iter().enumerate() {
            let seq = seq as u32;
            // Repeat the manifest through a long transfer so a receiver that
            // tuned in late can still make sense of what it has (§6.1).
            if cfg.rifp_manifest_every > 0
                && seq > 0
                && seq.is_multiple_of(cfg.rifp_manifest_every as u32)
            {
                frames.push(make(FRAME_MANIFEST, 0, manifest_json.clone(), true));
            }
            for r in 0..data_repeats {
                frames.push(make(FRAME_DATA, seq, chunk.to_vec(), r > 0));
            }
        }
        let mut end = Vec::with_capacity(rifp::END_PAYLOAD_LEN);
        end.extend_from_slice(&(encoded.bytes.len() as u64).to_be_bytes());
        end.extend_from_slice(&manifest.payload_crc32.to_be_bytes());
        end.extend_from_slice(&rifp_object::sha256_bytes(&encoded.bytes));
        frames.push(make(FRAME_END, total, end, false));

        self.tx_encoding = Some(encoded.encoding);
        self.tx_bytes = encoded.bytes.len() as u32;
        tracing::info!(
            target: "rifp",
            session = format!("{session_id:016x}"),
            encoding = encoded.encoding.label(),
            bytes = encoded.bytes.len(),
            frames = frames.len(),
            "transmitting"
        );
        Ok(RifpTx::new(frames, OUT_RATE))
    }
}

/// Trim a received or configured string to something a panel can display,
/// dropping control characters on the way (§5: impose a display-length limit).
fn clip(text: &str, max: usize) -> String {
    text.chars().filter(|c| !c.is_control()).take(max).collect()
}

/// Filename stem for an outgoing picture: the operator's callsign where there
/// is one, reduced to characters a receiver will keep.
fn picture_stem(call: &str) -> String {
    let cleaned: String = call
        .trim()
        .to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
        .take(16)
        .collect();
    if cleaned.is_empty() { "sdroxide".into() } else { cleaned }
}

fn file_suffix(encoding: RifpEncoding) -> &'static str {
    match encoding {
        RifpEncoding::Png => "png",
        RifpEncoding::Jpeg => "jpg",
        RifpEncoding::Group3 | RifpEncoding::Group4 => "tif",
        _ => "raster",
    }
}

impl DigiEngine for RifpController {
    fn mode(&self) -> Mode {
        Mode::Rifp
    }

    fn on_rx_audio(&mut self, tap: &[f32]) {
        let mut frames = std::mem::take(&mut self.rx_scratch);
        frames.clear();
        self.rx.process(tap, &mut frames);
        for frame in frames.drain(..) {
            // Several symbol-timing phases decode the same transmission, so
            // the same frame arrives more than once. Above this line that is
            // indistinguishable from a sender's own repeat, and both are
            // handled the same way: identical content is a duplicate.
            self.on_frame(frame);
        }
        self.rx_scratch = frames; // hand the buffer back for reuse
    }

    fn poll(&mut self, now: SystemTime, _dial_hz: f64) -> Vec<DigiAction> {
        self.now = now;
        let mut actions = std::mem::take(&mut self.queued);
        if self.tx.is_some() && !self.keyed {
            self.keyed = true;
            self.status_dirty = true;
            actions.push(DigiAction::KeyTx);
        }

        // Expire sessions nothing has fed for a while (§8).
        let timeout = self.cfg.rifp_session_timeout_s.max(30) as u64;
        let stale: Vec<u64> = self
            .sessions
            .iter()
            .filter(|s| now.duration_since(s.last_seen).map(|d| d.as_secs()).unwrap_or(0) > timeout)
            .map(|s| s.id)
            .collect();
        for id in stale {
            self.sessions.retain(|s| s.id != id);
            self.fail(format!("session {id:016x} expired incomplete"));
        }
        // Forget finished sessions on the same clock, so a station reusing an
        // ID much later is not ignored forever.
        self.done
            .retain(|(_, at)| now.duration_since(*at).map(|d| d.as_secs()).unwrap_or(0) <= timeout);

        let periodic = match self.last_status {
            Some(t) => now.duration_since(t).map(|d| d.as_millis() >= 200).unwrap_or(true),
            None => true,
        };
        if self.status_dirty || periodic {
            self.status_dirty = false;
            self.last_status = Some(now);
            actions.push(DigiAction::RifpStatus(self.rifp_status()));
        }
        actions
    }

    fn tx_burst_active(&self) -> bool {
        self.keyed
    }

    fn fill_tx_block(&mut self, out: &mut [f32]) -> bool {
        match &mut self.tx {
            Some(tx) => {
                self.status_dirty = true;
                tx.next_block(out)
            }
            None => {
                out.fill(0.0);
                true
            }
        }
    }

    fn on_burst_done(&mut self) {
        self.tx = None;
        self.keyed = false;
        self.status_dirty = true;
    }

    fn abort(&mut self) {
        self.abort_tx();
    }

    fn abort_tx(&mut self) {
        self.tx = None;
        self.keyed = false;
        self.status_dirty = true;
    }

    fn set_config(&mut self, cfg: DigiConfig) {
        self.cfg = cfg;
        self.status_dirty = true;
    }

    fn set_audio_hz(&mut self, _hz: f32) {}

    fn audio_hz(&self) -> f32 {
        0.0
    }

    fn status(&self) -> DigiStatus {
        self.digi_status()
    }

    fn set_rifp_image(&mut self, rgb: Vec<u8>, w: u16, h: u16) {
        match self.build_transmission(&rgb, w, h) {
            Ok(tx) => {
                self.last_error = None;
                self.tx = Some(tx);
            }
            Err(e) => self.fail(format!("transmit: {e}")),
        }
        self.status_dirty = true;
    }

    fn rifp_drop_session(&mut self, session: &str) {
        if session.is_empty() {
            self.sessions.clear();
        } else {
            self.sessions.retain(|s| format!("{:016x}", s.id) != session);
        }
        self.last_error = None;
        self.status_dirty = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdroxide_types::RifpSize;

    fn config(encoding: RifpEncoding, bpp: u8) -> DigiConfig {
        DigiConfig {
            my_call: "OE1TEST".into(),
            rifp_encoding: encoding,
            rifp_bits_per_pixel: bpp,
            rifp_size: RifpSize::Tiny,
            rifp_chunk_size: 192,
            // One pass each: the test wants the shortest transmission that is
            // still a complete one.
            rifp_data_repeats: 1,
            rifp_manifest_repeats: 1,
            rifp_manifest_every: 8,
            rifp_content_hint: "test card".into(),
            rifp_dither: false,
            ..DigiConfig::default()
        }
    }

    /// A picture, out of one controller's transmitter and into another's
    /// receiver over the modem — the whole protocol end to end.
    fn round_trip(encoding: RifpEncoding, bpp: u8) -> (Vec<u8>, Vec<DigiAction>) {
        let (w, h) = (48u16, 32u16);
        let rgb: Vec<u8> = (0..w as usize * h as usize)
            .flat_map(|i| {
                let v = ((i * 5) % 256) as u8;
                [v, v, v]
            })
            .collect();

        let mut tx = RifpController::new(config(encoding, bpp), OUT_RATE);
        tx.set_rifp_image(rgb.clone(), w, h);
        assert!(tx.tx.is_some(), "transmission was not built: {:?}", tx.last_error);

        let mut rx = RifpController::new(config(encoding, bpp), OUT_RATE);
        let mut block = [0.0f32; 480];
        let mut actions = Vec::new();
        let mut now = SystemTime::UNIX_EPOCH;
        while !tx.tx.as_ref().is_none_or(|t| t.done()) {
            tx.fill_tx_block(&mut block);
            rx.on_rx_audio(&block);
            now += std::time::Duration::from_millis(10);
            actions.extend(rx.poll(now, 0.0));
        }
        rx.on_rx_audio(&[0.0; 4800]);
        actions.extend(rx.poll(now, 0.0));
        (rgb, actions)
    }

    #[test]
    fn transmits_and_receives_a_picture() {
        let (_, actions) = round_trip(RifpEncoding::Zlib, 4);
        let image = actions.iter().find_map(|a| match a {
            DigiAction::RifpImage { meta, w, h, rgb, .. } => Some((meta, *w, *h, rgb)),
            _ => None,
        });
        let (meta, w, h, rgb) = image.expect("no picture completed");
        assert_eq!((w, h), (48, 32));
        assert_eq!(rgb.len(), 48 * 32 * 3);
        assert_eq!(meta.sender.as_deref(), Some("OE1TEST"));
        assert_eq!(meta.hint.as_deref(), Some("test card"));
        assert_eq!(meta.media_type, "image/rifp-raster");
        assert_eq!(meta.content_encoding, "zlib");
        assert_eq!(meta.bits_per_pixel, 4);
        // Nothing was needed twice on a perfect channel.
        assert_eq!(meta.chunks_first_pass, meta.chunk_count);
    }

    #[test]
    fn raw_raster_paints_before_it_finishes() {
        let (_, actions) = round_trip(RifpEncoding::Raw, 8);
        let rows: Vec<&DigiAction> =
            actions.iter().filter(|a| matches!(a, DigiAction::RifpRows { .. })).collect();
        assert!(!rows.is_empty(), "raw raster should paint progressively");
        // Every row of the picture, and no more.
        let painted: usize = rows
            .iter()
            .map(|a| match a {
                DigiAction::RifpRows { w, rows, .. } => rows.len() / *w as usize,
                _ => 0,
            })
            .sum();
        assert_eq!(painted, 32);
        assert!(actions.iter().any(|a| matches!(a, DigiAction::RifpImage { .. })));
    }

    /// The picture that comes out is the picture that went in — through the
    /// quantiser, so compare against what the sender itself encoded.
    #[test]
    fn received_pixels_match_what_was_sent() {
        let (rgb, actions) = round_trip(RifpEncoding::Png, 8);
        let sent = rifp_object::encode_object(&rgb, 48, 32, RifpEncoding::Png, 8, false).unwrap();
        let got = actions
            .iter()
            .find_map(|a| match a {
                DigiAction::RifpImage { rgb, .. } => Some(rgb.clone()),
                _ => None,
            })
            .expect("no picture completed");
        let got_gray: Vec<u8> = got.chunks_exact(3).map(|p| p[0]).collect();
        assert_eq!(got_gray, sent.gray);
    }

    /// The whole radio path: encode, frame, modulate to IQ, demodulate, and
    /// reassemble — the same modulator and demodulator the engine builds.
    ///
    /// This exists because the layers above it cannot see what the receive
    /// filtering does to a burst. A sharp passband rings across symbol
    /// transitions and a quick DC block drags the slicing level onto a run of
    /// identical bits; both cost whole frames on a channel with no noise in it
    /// at all, and both are invisible to a test that hands the modem's own
    /// waveform straight back to itself.
    ///
    /// The picture is chosen so its *payload* is the awkward case, which at
    /// 8 bpp with the raw raster is the pixel values themselves: long runs of
    /// 0x00 broken by shorter runs of 0xFF. A run in one direction pushes any
    /// tracking level onto itself, and the run back overshoots — which is how
    /// bits start landing on the wrong side of the slicer without a single dB
    /// of noise being involved.
    #[test]
    fn survives_the_real_modulator_and_demodulator() {
        let (w, h) = (96u16, 48u16);
        let rgb: Vec<u8> = (0..w as usize * h as usize)
            .flat_map(|i| {
                let v = if i % 360 < 300 { 0u8 } else { 255 };
                [v, v, v]
            })
            .collect();
        // One pass per frame: a repeat would paper over exactly what is under
        // test.
        let mut cfg = config(RifpEncoding::Raw, 8);
        cfg.rifp_data_repeats = 1;
        cfg.rifp_manifest_repeats = 1;

        let mut tx = RifpController::new(cfg.clone(), OUT_RATE);
        tx.set_rifp_image(rgb.clone(), w, h);
        let mut modulator =
            sdroxide_dsp::make_modulator(Mode::Rifp, OUT_RATE, Mode::Rifp.default_filter())
                .expect("RIFP modulator");
        let mut demod = sdroxide_dsp::make_demod(Mode::Rifp, OUT_RATE).expect("RIFP demodulator");
        let mut rx = RifpController::new(cfg, OUT_RATE);

        let mut symbols = [0.0f32; 480];
        let mut iq = Vec::new();
        let mut audio = Vec::new();
        let mut actions = Vec::new();
        let mut now = SystemTime::UNIX_EPOCH;
        // A little silence first, so the demodulator starts from rest.
        for _ in 0..10 {
            iq.clear();
            modulator.process(&[0.0; 480], &mut iq);
            audio.clear();
            demod.process(&iq, &mut audio);
            rx.on_rx_audio(&audio);
        }
        loop {
            let done = tx.fill_tx_block(&mut symbols);
            iq.clear();
            modulator.process(&symbols, &mut iq);
            audio.clear();
            demod.process(&iq, &mut audio);
            rx.on_rx_audio(&audio);
            now += std::time::Duration::from_millis(10);
            actions.extend(rx.poll(now, 0.0));
            if done {
                break;
            }
        }
        for _ in 0..10 {
            iq.clear();
            modulator.process(&[0.0; 480], &mut iq);
            audio.clear();
            demod.process(&iq, &mut audio);
            rx.on_rx_audio(&audio);
        }
        actions.extend(rx.poll(now, 0.0));

        let picture = actions.iter().find_map(|a| match a {
            DigiAction::RifpImage { meta, rgb, .. } => Some((meta, rgb)),
            _ => None,
        });
        let (meta, out) = picture.unwrap_or_else(|| {
            panic!(
                "no picture over the radio path: {} sessions, last error {:?}",
                rx.sessions.len(),
                rx.last_error
            )
        });
        assert_eq!((meta.width, meta.height), (w, h));
        assert_eq!(meta.chunks_first_pass, meta.chunk_count, "a frame had to be repeated");
        assert_eq!(rx.rx.bad_frames(), 0, "the receive chain corrupted frames");
        // Every pixel, exactly: the raw raster is lossless, so anything else
        // means the modem read a bit wrong and the CRC happened to pass.
        let got: Vec<u8> = out.chunks_exact(3).map(|p| p[0]).collect();
        let want: Vec<u8> = rgb.chunks_exact(3).map(|p| p[0]).collect();
        assert_eq!(got, want, "the picture came back changed");
    }

    /// A frame that fails its CRC must never reach the session layer.
    #[test]
    fn corrupted_frames_are_counted_not_believed() {
        let mut rx = RifpController::new(config(RifpEncoding::Raw, 8), OUT_RATE);
        let mut frame = RifpFrame::new(FRAME_DATA, 0x1234, 0, 4, vec![7; 32]);
        frame.tlvs.push(Tlv::text(TLV_SENDER_ID, "NOISE"));
        let mut air = frame.air_frame();
        let n = air.len();
        air[n - 20] ^= 0x08; // one bit, inside the payload

        let mut tx = RifpTx::new(vec![air], OUT_RATE);
        let mut block = [0.0f32; 480];
        while !tx.done() {
            tx.next_block(&mut block);
            rx.on_rx_audio(&block);
        }
        assert!(rx.sessions.is_empty(), "a bad-CRC frame opened a session");
        assert_eq!(rx.rx_frames, 0);
        assert!(rx.rx.bad_frames() > 0, "the corrupted frame was not counted");
    }

    /// Two different payloads for one sequence number: §6.2 says abandon the
    /// session rather than pick one — and a later repeat must not restart it.
    #[test]
    fn conflicting_chunks_abandon_the_session() {
        let mut rx = RifpController::new(config(RifpEncoding::Raw, 8), OUT_RATE);
        rx.on_frame(RifpFrame::new(FRAME_DATA, 9, 0, 2, vec![1; 16]));
        assert_eq!(rx.sessions.len(), 1);
        rx.on_frame(RifpFrame::new(FRAME_DATA, 9, 0, 2, vec![2; 16]));
        assert!(rx.sessions.is_empty());
        assert!(rx.last_error.as_deref().unwrap_or("").contains("different"));
        rx.on_frame(RifpFrame::new(FRAME_DATA, 9, 1, 2, vec![3; 16]));
        assert!(rx.sessions.is_empty(), "an abandoned session was restarted");
    }

    /// A sender keeps repeating after the last chunk we needed. Those repeats
    /// must not open a second, half-empty session for a picture already shown.
    #[test]
    fn repeats_after_completion_do_not_reopen_the_session() {
        let (w, h) = (48u16, 32u16);
        let rgb = vec![128u8; w as usize * h as usize * 3];
        let mut tx = RifpController::new(config(RifpEncoding::Raw, 8), OUT_RATE);
        tx.set_rifp_image(rgb, w, h);
        let mut rx = RifpController::new(config(RifpEncoding::Raw, 8), OUT_RATE);
        let mut block = [0.0f32; 480];
        let mut actions = Vec::new();
        let mut now = SystemTime::UNIX_EPOCH;
        while !tx.tx.as_ref().is_none_or(|t| t.done()) {
            tx.fill_tx_block(&mut block);
            rx.on_rx_audio(&block);
            now += std::time::Duration::from_millis(10);
            actions.extend(rx.poll(now, 0.0));
        }
        rx.on_rx_audio(&[0.0; 4800]);
        actions.extend(rx.poll(now, 0.0));

        assert_eq!(
            actions.iter().filter(|a| matches!(a, DigiAction::RifpImage { .. })).count(),
            1,
            "the picture was delivered more than once"
        );
        assert!(rx.sessions.is_empty(), "the sender's repeats left a phantom session behind");
    }

    /// A CANCEL drops what we were holding for that session (§6.4).
    #[test]
    fn cancel_drops_the_session() {
        let mut rx = RifpController::new(config(RifpEncoding::Raw, 8), OUT_RATE);
        rx.on_frame(RifpFrame::new(FRAME_DATA, 42, 0, 2, vec![1; 16]));
        assert_eq!(rx.sessions.len(), 1);
        rx.on_frame(RifpFrame::new(FRAME_CANCEL, 42, 0, 0, b"done".to_vec()));
        assert!(rx.sessions.is_empty());
    }

    /// Chunks that arrive before the manifest are kept, not dropped (§8).
    #[test]
    fn data_before_manifest_is_kept() {
        let mut rx = RifpController::new(config(RifpEncoding::Raw, 8), OUT_RATE);
        rx.on_frame(RifpFrame::new(FRAME_DATA, 5, 1, 3, vec![9; 8]));
        rx.on_frame(RifpFrame::new(FRAME_DATA, 5, 2, 3, vec![9; 8]));
        assert_eq!(rx.sessions[0].have, 2);
        assert!(rx.sessions[0].manifest.is_none());
    }

    /// Concurrent sessions are bounded; the least recently fed goes first.
    #[test]
    fn session_count_is_bounded() {
        let mut rx = RifpController::new(config(RifpEncoding::Raw, 8), OUT_RATE);
        for id in 0..(MAX_SESSIONS as u64 + 4) {
            rx.now += std::time::Duration::from_secs(1);
            rx.on_frame(RifpFrame::new(FRAME_DATA, id, 0, 2, vec![1; 16]));
        }
        assert_eq!(rx.sessions.len(), MAX_SESSIONS);
        // The four oldest were evicted, not the newest.
        assert!(rx.sessions.iter().all(|s| s.id >= 4));
    }
}
