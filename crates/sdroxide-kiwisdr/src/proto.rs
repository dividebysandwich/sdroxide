//! The KiwiSDR wire format: the frames the receiver sends and the `SET` lines
//! it is driven with.
//!
//! Pure, so all of it is testable without a receiver — which matters, because
//! the protocol is documented nowhere. What is here was read off a live
//! KiwiSDR 1 running v1.902 and then checked against the two packet structs in
//! the server's own `rx/rx_sound.h` and `rx/rx_waterfall.h`.
//!
//! # The one asymmetry worth knowing about
//!
//! The audio packet's tag is **three** bytes (`char id[3]` — `strncpy(…, "SND",
//! 3)`), and the waterfall packet's is **four** (`char id4[4]`, holding
//! `"W/F "`). Reading three in both places leaves every waterfall field a byte
//! out: `x_bin_server` comes back as 32 — the space — and a stray byte appears
//! in front of the bins. It looks like a plausible frame, which is what makes
//! it worth a paragraph.

/// Set while the receiver's ADC is clipping.
pub const SND_FLAG_ADC_OVFL: u8 = 0x02;
/// Two channels rather than one, which is how `mod=iq` delivers I and Q.
pub const SND_FLAG_STEREO: u8 = 0x08;
/// IMA-ADPCM rather than linear. Never set in `iq` mode — the receiver does not
/// compress a stereo stream — which is why no decoder is implemented here.
pub const SND_FLAG_COMPRESSED: u8 = 0x10;
/// Samples are little-endian. Only ever set on a camped connection, which this
/// client does not make.
pub const SND_FLAG_LITTLE_ENDIAN: u8 = 0x80;

/// Bins per waterfall frame — `WF_WIDTH` in `rx/rx_waterfall.h`.
pub const WF_WIDTH: usize = 1024;

/// The receiver's deepest zoom, and so the grid its `x_bin` is counted on.
///
/// A frame says where it starts as a bin index on the *maximum-zoom* grid,
/// which divides the whole band into `WF_WIDTH << WF_MAX_ZOOM` bins whatever
/// zoom the frame itself is at. Verified live against a KiwiSDR 1 on
/// 0-30 MHz: asked for zoom 4 centred on 9950 kHz it answered `x_bin=5040155`,
/// which on this grid is 9012.5 kHz, and a 1875 kHz window from there is
/// centred on 9950.0 kHz exactly. At zoom 8 the same request gave a 117.2 kHz
/// window whose strongest bin was 9999.9 kHz — WWV on 10 MHz.
pub const WF_MAX_ZOOM: u32 = 14;

/// `flags_x_zoom_server` carries the zoom in its low half and flags in its
/// high half.
pub const WF_FLAGS_COMPRESSION: u32 = 0x0001_0000;

/// A tag and the body behind it.
#[derive(Debug, PartialEq)]
pub enum Frame<'a> {
    /// `key=value` text, space separated. The receiver's whole control channel.
    Msg(&'a str),
    Snd(&'a [u8]),
    Wf(&'a [u8]),
    /// Something else. Extensions send their own tags and are not an error.
    Other,
}

/// Split a binary WebSocket message into its tag and body.
pub fn split(msg: &[u8]) -> Frame<'_> {
    if msg.len() < 3 {
        return Frame::Other;
    }
    match &msg[..3] {
        b"MSG" => Frame::Msg(std::str::from_utf8(&msg[3..]).unwrap_or("")),
        // Three bytes here …
        b"SND" => Frame::Snd(&msg[3..]),
        // … and four here. See the module note.
        b"W/F" if msg.len() >= 4 => Frame::Wf(&msg[4..]),
        _ => Frame::Other,
    }
}

/// The `key=value` pairs of a `MSG` frame.
///
/// Values are percent-ish escaped by the receiver in a few places but not
/// consistently, and nothing this client reads is affected, so they are handed
/// over as they arrived.
pub fn msg_params(text: &str) -> impl Iterator<Item = (&str, &str)> {
    text.split_ascii_whitespace().map(|kv| match kv.split_once('=') {
        Some((k, v)) => (k, v),
        None => (kv, ""),
    })
}

/// What an audio packet says about itself, ahead of the samples.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SndHeader {
    pub flags: u8,
    pub seq: u32,
    /// Signal level in dBm, on the receiver's own calibration.
    ///
    /// This is the S-meter to use, and not one derived from the sample
    /// amplitude: the receiver's AGC sits ahead of the I/Q, so the amplitude
    /// says as much about the AGC as about the signal.
    pub smeter_dbm: f32,
    pub adc_overflow: bool,
}

/// Decode an `SND` body in `iq` mode, appending interleaved I/Q to `out`.
///
/// Samples are big-endian `i16` and are scaled to ±1.0. The ten bytes of GPS
/// timestamp between the header and the samples are skipped: sdroxide times
/// from its own clock, and the fields are zero on a receiver without a lock
/// anyway.
pub fn decode_snd_iq(body: &[u8], out: &mut Vec<f32>) -> Result<SndHeader, String> {
    // flags(1) + seq(4) + smeter(2)
    if body.len() < 7 {
        return Err(format!("short SND frame: {} bytes", body.len()));
    }
    let flags = body[0];
    let seq = u32::from_le_bytes([body[1], body[2], body[3], body[4]]);
    let smeter_raw = u16::from_be_bytes([body[5], body[6]]);
    let header = SndHeader {
        flags,
        seq,
        smeter_dbm: smeter_dbm(smeter_raw),
        adc_overflow: flags & SND_FLAG_ADC_OVFL != 0,
    };

    if flags & SND_FLAG_STEREO == 0 {
        return Err("the receiver sent mono audio where I/Q was asked for".into());
    }
    if flags & SND_FLAG_COMPRESSED != 0 {
        // The receiver does not compress a stereo stream, so this cannot
        // happen — and if it ever does, decoding it as linear would produce
        // noise that looked like a band rather than an error.
        return Err("compressed I/Q, which this protocol is not supposed to produce".into());
    }

    let rest = &body[7..];
    // last_gps_solution(1) + dummy(1) + gpssec(4) + gpsnsec(4)
    const GPS_HEADER: usize = 10;
    if rest.len() < GPS_HEADER {
        return Err(format!("SND frame with no room for its GPS header: {} bytes", rest.len()));
    }
    let samples = &rest[GPS_HEADER..];
    // I and Q must stay paired: a frame that is not a whole number of pairs
    // would swap them for the rest of the session and mirror the spectrum.
    if !samples.len().is_multiple_of(4) {
        return Err(format!("SND payload of {} bytes is not whole I/Q pairs", samples.len()));
    }
    out.reserve(samples.len() / 2);
    for s in samples.chunks_exact(2) {
        out.push(f32::from(i16::from_be_bytes([s[0], s[1]])) / 32768.0);
    }
    Ok(header)
}

/// Where a waterfall frame's 1024 bins actually sit, as `(centre, span)` in Hz.
///
/// Read from the frame rather than from what was asked for, which matters as
/// soon as the window is anything but the whole band: the receiver clamps a
/// centre that would push the window past a band edge, and a frame in flight
/// when the window moves still describes where it was. `band_center_hz` and
/// `bandwidth_hz` are the receiver's own, from its opening burst.
pub fn wf_window(h: &WfHeader, band_center_hz: f64, bandwidth_hz: f64) -> (f64, f64) {
    let span = bandwidth_hz / f64::from(1u64.wrapping_shl(h.zoom.min(WF_MAX_ZOOM)) as u32);
    let grid = bandwidth_hz / (WF_WIDTH as f64 * f64::from(1u32 << WF_MAX_ZOOM));
    let lo = band_center_hz - bandwidth_hz / 2.0 + f64::from(h.x_bin) * grid;
    (lo + span / 2.0, span)
}

/// The receiver's S-meter byte pair, in dBm.
///
/// The scale is the Kiwi's: tenths of a dB above -127 dBm. Whether it is
/// *accurate* is its operator's business — `sm_cal` in their configuration —
/// and neither the listing nor the protocol says whether it was ever set.
pub fn smeter_dbm(raw: u16) -> f32 {
    0.1 * f32::from(raw) - 127.0
}

/// What a waterfall packet says about itself.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WfHeader {
    /// The leftmost bin's position on the receiver's own zoom-14 grid.
    pub x_bin: u32,
    pub zoom: u32,
    pub seq: u32,
    pub compressed: bool,
}

/// Decode a `W/F` body, appending dBm bins to `out` from the low edge up.
///
/// `wf_cal` is the receiver operator's own waterfall calibration, which arrives
/// in the opening `MSG` burst and is applied here rather than by the server.
pub fn decode_wf(body: &[u8], wf_cal: i32, out: &mut Vec<f32>) -> Result<WfHeader, String> {
    // x_bin_server(4) + flags_x_zoom_server(4) + seq(4)
    const HEADER: usize = 12;
    if body.len() < HEADER {
        return Err(format!("short W/F frame: {} bytes", body.len()));
    }
    let u32_at = |i: usize| u32::from_le_bytes([body[i], body[i + 1], body[i + 2], body[i + 3]]);
    let flags_zoom = u32_at(4);
    let header = WfHeader {
        x_bin: u32_at(0),
        zoom: flags_zoom & 0xffff,
        seq: u32_at(8),
        compressed: flags_zoom & WF_FLAGS_COMPRESSION != 0,
    };
    if header.compressed {
        // `SET wf_comp=0` turns this off and is sent at connect; a compressed
        // frame means the receiver ignored it, and half-rate ADPCM decoded as
        // bytes would draw a convincing but wrong band.
        return Err("compressed waterfall, which this client did not ask for".into());
    }
    let bins = &body[HEADER..];
    if bins.len() < WF_WIDTH {
        return Err(format!("W/F frame carries {} bins, expected {WF_WIDTH}", bins.len()));
    }
    out.reserve(WF_WIDTH);
    // The receiver quantises each bin to one byte of dBm, biased so that 255
    // is 0 dBm. Nothing is normalised on the way: these are the same numbers
    // its own display draws.
    out.extend(bins[..WF_WIDTH].iter().map(|&b| f32::from(b) - 255.0 + wf_cal as f32));
    Ok(header)
}

// -------------------------------------------------------------------------
// The commands
// -------------------------------------------------------------------------

/// Open a session. `t=kiwi` is an ordinary user connection; the password is
/// blank on almost every public receiver.
pub fn set_auth(password: &str) -> String {
    format!("SET auth t=kiwi p={password}")
}

/// Who this end is, as the receiver's owner and its other listeners will see
/// it. Sent on both sockets.
pub fn set_ident(ident: &str) -> String {
    // Spaces separate the receiver's own key=value parsing, so a callsign with
    // one in it would be read as two parameters.
    format!("SET ident_user={}", ident.replace(' ', "_"))
}

/// Tune the audio channel and take it as I/Q.
///
/// `freq` is in **kHz**, which is the unit the protocol uses throughout. The
/// cuts are the full ±6 kHz the channel has: narrowing them would filter the
/// I/Q before sdroxide's own passband ever saw it.
pub fn set_mod_iq(freq_khz: f64) -> String {
    format!("SET mod=iq low_cut=-6000 high_cut=6000 freq={freq_khz:.3}")
}

/// The receiver's AGC, which sits ahead of the I/Q. See `KiwiConfig::agc` for
/// why it is on by default.
pub fn set_agc(on: bool, man_gain: u8) -> String {
    format!("SET agc={} hang=0 thresh=-100 slope=6 decay=1000 manGain={man_gain}", u8::from(on))
}

/// Linear samples rather than ADPCM. Required: the I/Q path has no decoder.
pub fn set_compression(on: bool) -> String {
    format!("SET compression={}", u8::from(on))
}

/// Acknowledge the audio rate. The receiver does not start sending until it
/// has this.
pub fn set_ar_ok(in_rate: u32) -> String {
    format!("SET AR OK in={in_rate} out=44100")
}

pub fn set_keepalive() -> &'static str {
    "SET keepalive"
}

/// Zoom 0 is the receiver's whole band, which is the only span this client
/// asks for: the waterfall is the band view, and the I/Q is the detail.
pub fn set_zoom_cf(zoom: u32, cf_khz: f64) -> String {
    format!("SET zoom={zoom} cf={cf_khz:.3}")
}

/// The dB window the receiver quantises its bins into before sending them, so
/// this decides the resolution of what arrives and not just how it is drawn.
pub fn set_maxdb_mindb(maxdb: i32, mindb: i32) -> String {
    format!("SET maxdb={maxdb} mindb={mindb}")
}

pub fn set_wf_comp(on: bool) -> String {
    format!("SET wf_comp={}", u8::from(on))
}

/// 1 (slowest) to 4. The receiver caps it at its own `wf_fps_max`.
pub fn set_wf_speed(speed: u8) -> String {
    format!("SET wf_speed={}", speed.clamp(1, 4))
}

/// Off: interpolation is a display nicety the receiver applies to its own
/// picture, and this end pools the bins itself.
pub fn set_interp(on: bool) -> String {
    format!("SET interp={}", u8::from(on))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The window a frame describes, against a live receiver's own answers.
    ///
    /// Measured on a KiwiSDR 1 covering 0-30 MHz: asked for zoom 4 centred on
    /// 9950 kHz it answered `x_bin=5040155 zoom=4`, and for zoom 8 on the same
    /// centre `x_bin=5531675 zoom=8`. Both have to come back centred on what
    /// was asked for, which is what says `x_bin` is counted on the
    /// maximum-zoom grid and not on the frame's own.
    #[test]
    fn a_frames_window_is_where_the_receiver_put_it() {
        let (center, bw) = (15_000_000.0, 30_000_000.0);
        let h = |x_bin, zoom| WfHeader { x_bin, zoom, seq: 0, compressed: false };

        let (c, span) = wf_window(&h(5_040_155, 4), center, bw);
        assert!((c - 9_950_000.0).abs() < 500.0, "zoom 4 landed on {c}");
        assert!((span - 1_875_000.0).abs() < 1.0, "zoom 4 spans {span}");

        let (c, span) = wf_window(&h(5_531_675, 8), center, bw);
        assert!((c - 9_950_000.0).abs() < 60.0, "zoom 8 landed on {c}");
        assert!((span - 117_187.5).abs() < 1.0, "zoom 8 spans {span}");

        // And zoom 0 is the whole band, which is what every build before
        // issue #303 drew and what this still defaults to.
        let (c, span) = wf_window(&h(0, 0), center, bw);
        assert_eq!((c, span), (center, bw));
    }

    /// The exact bytes a KiwiSDR 1 v1.902 sent in `mod=iq`: a three-byte tag,
    /// flags 0x0d, then 512 big-endian I/Q pairs behind ten bytes of GPS.
    fn snd_frame(flags: u8, pairs: &[(i16, i16)]) -> Vec<u8> {
        let mut v = b"SND".to_vec();
        v.push(flags);
        v.extend_from_slice(&7u32.to_le_bytes());
        // 0.1 * 350 - 127 = -92 dBm
        v.extend_from_slice(&350u16.to_be_bytes());
        v.extend_from_slice(&[0u8; 10]);
        for (i, q) in pairs {
            v.extend_from_slice(&i.to_be_bytes());
            v.extend_from_slice(&q.to_be_bytes());
        }
        v
    }

    /// A `W/F ` frame with its **four**-byte tag.
    fn wf_frame(zoom: u32, seq: u32, bins: &[u8]) -> Vec<u8> {
        let mut v = b"W/F ".to_vec();
        v.extend_from_slice(&0u32.to_le_bytes());
        v.extend_from_slice(&zoom.to_le_bytes());
        v.extend_from_slice(&seq.to_le_bytes());
        v.extend_from_slice(bins);
        v
    }

    #[test]
    fn a_real_iq_frame_decodes() {
        let pairs: Vec<(i16, i16)> = (0..512).map(|n| (n as i16, -(n as i16))).collect();
        let raw = snd_frame(SND_FLAG_STEREO | 0x05, &pairs);
        // 3 tag + 1 flags + 4 seq + 2 smeter + 10 gps + 2048 samples.
        assert_eq!(raw.len(), 2068, "the length measured on the wire");

        let Frame::Snd(body) = split(&raw) else { panic!("not an SND frame") };
        let mut out = Vec::new();
        let h = decode_snd_iq(body, &mut out).expect("decodes");
        assert_eq!(h.seq, 7);
        assert!((h.smeter_dbm - -92.0).abs() < 1e-4);
        assert!(!h.adc_overflow);
        assert_eq!(out.len(), 1024, "512 pairs, interleaved");
        assert_eq!(out[0], 0.0);
        assert!((out[2] - 1.0 / 32768.0).abs() < 1e-9);
        assert!((out[3] - -1.0 / 32768.0).abs() < 1e-9);
    }

    #[test]
    fn the_waterfall_tag_is_four_bytes_not_three() {
        let bins: Vec<u8> = (0..WF_WIDTH).map(|i| (i % 256) as u8).collect();
        let raw = wf_frame(0, 42, &bins);
        assert_eq!(raw.len(), 4 + 12 + WF_WIDTH, "the length measured on the wire");

        let Frame::Wf(body) = split(&raw) else { panic!("not a W/F frame") };
        assert_eq!(body.len(), 12 + WF_WIDTH);
        let mut out = Vec::new();
        let h = decode_wf(body, 0, &mut out).expect("decodes");
        assert_eq!(h.seq, 42);
        assert_eq!(h.zoom, 0);
        assert!(!h.compressed);
        assert_eq!(out.len(), WF_WIDTH);
        // Reading a three-byte tag would put the last byte of `seq` here.
        assert_eq!(out[0], 0.0 - 255.0, "the first bin, not a byte of the header");
        assert_eq!(out[1], 1.0 - 255.0);
    }

    #[test]
    fn the_operators_waterfall_calibration_is_applied() {
        let raw = wf_frame(0, 1, &[200u8; WF_WIDTH]);
        let Frame::Wf(body) = split(&raw) else { panic!() };
        let mut out = Vec::new();
        decode_wf(body, -13, &mut out).expect("decodes");
        assert_eq!(out[0], 200.0 - 255.0 - 13.0);
    }

    #[test]
    fn the_zoom_is_the_low_half_and_the_flags_the_high_half() {
        let raw = wf_frame(WF_FLAGS_COMPRESSION | 6, 1, &[128u8; WF_WIDTH]);
        let Frame::Wf(body) = split(&raw) else { panic!() };
        let mut out = Vec::new();
        // Compression this client never asked for is refused rather than
        // decoded as bytes, which would draw a convincing but wrong band.
        let e = decode_wf(body, 0, &mut out).expect_err("refused");
        assert!(e.contains("compressed"));
    }

    #[test]
    fn mono_audio_is_refused_rather_than_taken_as_iq() {
        let raw = snd_frame(0x05, &[(1, 2); 512]);
        let Frame::Snd(body) = split(&raw) else { panic!() };
        let e = decode_snd_iq(body, &mut Vec::new()).expect_err("refused");
        assert!(e.contains("mono"));
    }

    #[test]
    fn a_truncated_frame_is_an_error_not_a_panic() {
        assert!(decode_snd_iq(&[0x0d, 0, 0], &mut Vec::new()).is_err());
        assert!(decode_wf(&[0, 0, 0], 0, &mut Vec::new()).is_err());
        // Whole header, no samples: still not a frame.
        let mut short = b"SND".to_vec();
        short.extend_from_slice(&[0x0d, 0, 0, 0, 0, 0, 0]);
        let Frame::Snd(body) = split(&short) else { panic!() };
        assert!(decode_snd_iq(body, &mut Vec::new()).is_err());
        assert_eq!(split(b"XY"), Frame::Other);
    }

    #[test]
    fn msg_params_are_key_value_pairs() {
        let p: Vec<_> = msg_params("sample_rate=11998.874997 audio_init=0 wf_setup").collect();
        assert_eq!(p[0], ("sample_rate", "11998.874997"));
        assert_eq!(p[1], ("audio_init", "0"));
        assert_eq!(p[2], ("wf_setup", ""), "a bare flag is its own key");
    }

    #[test]
    fn the_commands_are_the_ones_the_receiver_answered() {
        assert_eq!(set_auth(""), "SET auth t=kiwi p=");
        assert_eq!(set_mod_iq(9950.0), "SET mod=iq low_cut=-6000 high_cut=6000 freq=9950.000");
        assert_eq!(set_agc(true, 50), "SET agc=1 hang=0 thresh=-100 slope=6 decay=1000 manGain=50");
        assert_eq!(set_ar_ok(11998), "SET AR OK in=11998 out=44100");
        assert_eq!(set_zoom_cf(0, 15000.0), "SET zoom=0 cf=15000.000");
        assert_eq!(set_wf_speed(9), "SET wf_speed=4", "clamped to what the protocol takes");
    }

    /// A space in the announced name would be read by the receiver as the start
    /// of another parameter.
    #[test]
    fn an_ident_with_a_space_cannot_split_the_command() {
        assert_eq!(set_ident("OE1ABC (sdroxide)"), "SET ident_user=OE1ABC_(sdroxide)");
    }
}
