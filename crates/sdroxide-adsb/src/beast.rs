//! The Mode-S Beast binary format: what `dump1090`, `readsb`, `tar1090`, the
//! feeders and the aggregators speak to one another over TCP.
//!
//! Written from the format as the Beast receiver's documentation publishes it
//! (Günter Köllner, "Mode-S Beast: Binary Format") and as `readsb` emits it on
//! port 30005; no code was taken from either.
//!
//! # A frame
//!
//! ```text
//! 0x1A  type  timestamp(6)  signal(1)  message(2 | 7 | 14)
//! ```
//!
//! * `type` is `'1'` for a Mode A/C reply (2 bytes), `'2'` for a short Mode S
//!   message (7 bytes) and `'3'` for a long one (14 bytes).
//! * The timestamp is a 48-bit big-endian count of a 12 MHz clock.
//! * The signal byte is `sqrt(power) * 255`, power on a 0–1 full-scale scale.
//!
//! # Escaping
//!
//! `0x1A` marks the start of a frame, so a `0x1A` that falls anywhere in the
//! timestamp, the signal or the message is sent twice. A lone `0x1A` inside a
//! frame therefore means the frame was cut short and a new one has begun —
//! which is how a reader that joins a stream mid-frame, or one fed a damaged
//! stream, finds its footing again.
//!
//! # Receiver ID
//!
//! `readsb` started with `--net-receiver-id` puts a `0x1A 0xE3` record with an
//! eight-byte receiver ID (escaped like everything else) in front of a frame,
//! naming the receiver the frame came from. The parser hands it on with the
//! frame that follows, so a station that sees its own frames come back through
//! an aggregator can recognise them. Any other type byte is skipped: the reader
//! moves to the next frame start.

/// Start of a frame, and the escape byte.
pub const ESC: u8 = 0x1A;

/// The rate of the Beast timestamp clock.
pub const CLOCK_HZ: f64 = 12_000_000.0;

/// The record `readsb --net-receiver-id` puts in front of a frame.
const RECEIVER_ID: u8 = 0xE3;

/// One frame off the wire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeastFrame {
    /// 48-bit count of the 12 MHz clock.
    pub timestamp: u64,
    /// `sqrt(power) * 255`; see [`signal_to_dbfs`].
    pub signal: u8,
    /// 2, 7 or 14 bytes.
    pub bytes: Vec<u8>,
    /// The receiver `readsb` said the frame came from, if it said.
    pub receiver: Option<u64>,
}

/// The type byte for a message of this many bytes.
fn type_for_len(len: usize) -> Option<u8> {
    match len {
        2 => Some(b'1'),
        7 => Some(b'2'),
        14 => Some(b'3'),
        _ => None,
    }
}

/// How many message bytes a type byte announces; `None` for a type this reader
/// does not take apart.
fn len_for_type(t: u8) -> Option<usize> {
    match t {
        b'1' => Some(2),
        b'2' => Some(7),
        b'3' => Some(14),
        _ => None,
    }
}

/// Append `b`, doubled if it is the escape byte.
fn push_escaped(out: &mut Vec<u8>, b: u8) {
    out.push(b);
    if b == ESC {
        out.push(ESC);
    }
}

/// Append one frame to `out`. A message of a length the format has no type
/// for (anything but 2, 7 or 14 bytes) writes nothing and returns `false`.
pub fn encode(out: &mut Vec<u8>, timestamp: u64, signal: u8, msg: &[u8]) -> bool {
    let Some(t) = type_for_len(msg.len()) else { return false };
    out.push(ESC);
    out.push(t);
    let ts = timestamp & 0xFFFF_FFFF_FFFF;
    for shift in (0..6).rev() {
        push_escaped(out, (ts >> (shift * 8)) as u8);
    }
    push_escaped(out, signal);
    for &b in msg {
        push_escaped(out, b);
    }
    true
}

/// The signal byte for a level in dBFS: `sqrt(power) * 255`, which is the
/// amplitude, `10^(dBFS / 20)`, on a 0–255 scale.
///
/// At least 1: `readsb` reads a zero as "no signal level reported".
pub fn dbfs_to_signal(dbfs: f32) -> u8 {
    let amp = 10f64.powf(f64::from(dbfs) / 20.0);
    (amp * 255.0).round().clamp(1.0, 255.0) as u8
}

/// The level in dBFS a signal byte stands for. `None` for zero, which is how a
/// sender says it measured nothing.
pub fn signal_to_dbfs(signal: u8) -> Option<f32> {
    (signal > 0).then(|| (20.0 * (f64::from(signal) / 255.0).log10()) as f32)
}

/// Samples at `rate_hz` counted since the stream began, as a Beast timestamp.
///
/// Good for ordering and for `readsb`'s bookkeeping; useless for
/// multilateration, which needs every receiver's clock tied to GPS time.
pub fn timestamp_from_samples(samples: u64, rate_hz: f64) -> u64 {
    if rate_hz <= 0.0 {
        return 0;
    }
    ((samples as f64) * CLOCK_HZ / rate_hz) as u64 & 0xFFFF_FFFF_FFFF
}

/// An incremental reader: feed it whatever the socket returned, take the whole
/// frames out. A frame split over two reads is held until the rest arrives.
#[derive(Debug, Default)]
pub struct Parser {
    buf: Vec<u8>,
    /// A receiver ID record seen, waiting for the frame it belongs to.
    receiver: Option<u64>,
    /// Bytes thrown away while looking for a frame start, or in a frame that
    /// was cut short — a count worth showing when a stream is not what it was
    /// said to be (a raw or SBS port given as Beast).
    pub skipped: u64,
}

/// What reading one record from the front of the buffer came to.
enum Step {
    /// A frame, and how many buffer bytes it took.
    Frame(BeastFrame, usize),
    /// A receiver ID record, and its length.
    Receiver(u64, usize),
    /// Not a frame: throw away this many bytes and look again.
    Skip(usize),
    /// Not enough bytes yet.
    More,
}

impl Parser {
    pub fn new() -> Parser {
        Parser::default()
    }

    /// Append `data` and return every frame now complete.
    pub fn push(&mut self, data: &[u8]) -> Vec<BeastFrame> {
        self.buf.extend_from_slice(data);
        let mut out = Vec::new();
        let mut at = 0;
        while at < self.buf.len() {
            match read_one(&self.buf[at..]) {
                Step::Frame(mut f, n) => {
                    f.receiver = self.receiver.take();
                    out.push(f);
                    at += n;
                }
                Step::Receiver(id, n) => {
                    self.receiver = Some(id);
                    at += n;
                }
                Step::Skip(n) => {
                    self.skipped += n as u64;
                    at += n;
                }
                Step::More => break,
            }
        }
        self.buf.drain(..at);
        out
    }
}

/// Read `n` unescaped bytes starting at `from`. Returns the bytes and the index
/// after them, `Err(None)` if the buffer ends first, or `Err(Some(i))` if a
/// lone escape at `i` says a new frame starts there.
fn take_unescaped(b: &[u8], from: usize, n: usize) -> Result<(Vec<u8>, usize), Option<usize>> {
    let mut out = Vec::with_capacity(n);
    let mut i = from;
    while out.len() < n {
        let Some(&x) = b.get(i) else { return Err(None) };
        if x == ESC {
            match b.get(i + 1) {
                None => return Err(None),
                Some(&ESC) => i += 2,
                Some(_) => return Err(Some(i)),
            }
        } else {
            i += 1;
        }
        out.push(x);
    }
    Ok((out, i))
}

fn read_one(b: &[u8]) -> Step {
    // Anything in front of a frame start is not ours.
    if b[0] != ESC {
        let n = b.iter().position(|&x| x == ESC).unwrap_or(b.len());
        return Step::Skip(n);
    }
    let Some(&t) = b.get(1) else { return Step::More };
    if t == ESC {
        // An escaped data byte, so we are inside a frame we never saw start.
        return Step::Skip(2);
    }
    if t == RECEIVER_ID {
        return match take_unescaped(b, 2, 8) {
            Ok((id, end)) => {
                Step::Receiver(id.iter().fold(0u64, |a, &x| a << 8 | u64::from(x)), end)
            }
            Err(None) => Step::More,
            Err(Some(i)) => Step::Skip(i),
        };
    }
    let Some(len) = len_for_type(t) else {
        // A record this reader does not take apart: on to the next start.
        let n = b[1..].iter().position(|&x| x == ESC).map_or(b.len(), |p| p + 1);
        return Step::Skip(n);
    };
    match take_unescaped(b, 2, 6 + 1 + len) {
        Ok((v, end)) => {
            let timestamp = v[..6].iter().fold(0u64, |a, &x| a << 8 | u64::from(x));
            Step::Frame(
                BeastFrame { timestamp, signal: v[6], bytes: v[7..].to_vec(), receiver: None },
                end,
            )
        }
        Err(None) => Step::More,
        Err(Some(i)) => Step::Skip(i),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(s: &str) -> Vec<u8> {
        (0..s.len() / 2).map(|i| u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).unwrap()).collect()
    }

    const LONG: &str = "8D40621D58C382D690C8AC2863A7";
    const SHORT: &str = "5D4CA2D4A5A0F6";

    /// What goes out comes back: every field, for both message lengths.
    #[test]
    fn a_frame_written_is_the_frame_read() {
        let mut wire = Vec::new();
        assert!(encode(&mut wire, 0x0123_4567_89AB, 200, &hex(LONG)));
        assert!(encode(&mut wire, 7, 9, &hex(SHORT)));
        let got = Parser::new().push(&wire);
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].timestamp, 0x0123_4567_89AB);
        assert_eq!((got[0].signal, got[0].bytes.clone()), (200, hex(LONG)));
        assert_eq!((got[1].timestamp, got[1].signal, got[1].bytes.clone()), (7, 9, hex(SHORT)));
    }

    /// The layout byte for byte, so this is the format and not merely a format
    /// this reader and this writer agree on.
    #[test]
    fn the_bytes_on_the_wire_are_the_published_layout() {
        let mut wire = Vec::new();
        encode(&mut wire, 0x0000_0000_0102, 0x80, &hex(SHORT));
        let mut want = vec![0x1A, b'2', 0, 0, 0, 0, 1, 2, 0x80];
        want.extend(hex(SHORT));
        assert_eq!(wire, want);
    }

    /// An escape byte in every field — each timestamp byte, the signal, every
    /// message byte — is doubled on the way out and single on the way in.
    #[test]
    fn an_escape_byte_anywhere_survives_the_round_trip() {
        let mut msg = hex(LONG);
        for pos in 0..msg.len() {
            msg[pos] = ESC;
            let mut wire = Vec::new();
            encode(&mut wire, 0x1A1A_1A1A_1A1A, ESC, &msg);
            // One start byte, the type, then everything doubled.
            assert_eq!(wire.len(), 2 + 2 * 7 + (msg.len() - pos - 1) + 2 * (pos + 1));
            let got = Parser::new().push(&wire);
            assert_eq!(got.len(), 1, "escape at message byte {pos}");
            assert_eq!(got[0].timestamp, 0x1A1A_1A1A_1A1A);
            assert_eq!((got[0].signal, &got[0].bytes), (ESC, &msg));
        }
    }

    /// A socket hands over whatever it has, so a frame arrives in pieces —
    /// including between the two bytes of an escape. Split at every point.
    #[test]
    fn a_frame_split_anywhere_is_put_back_together() {
        let mut msg = hex(LONG);
        msg[5] = ESC;
        let mut wire = Vec::new();
        encode(&mut wire, 0x1A, 3, &msg);
        for cut in 0..=wire.len() {
            let mut p = Parser::new();
            let mut got = p.push(&wire[..cut]);
            got.extend(p.push(&wire[cut..]));
            assert_eq!(got.len(), 1, "cut at {cut}");
            assert_eq!(got[0].bytes, msg);
            assert_eq!(p.skipped, 0, "cut at {cut}");
        }
    }

    /// Joining a stream mid-frame: the tail of somebody else's frame, garbage,
    /// then a frame cut short by the start of the next — only whole frames come
    /// out, and the stream is back in step at the next start.
    #[test]
    fn garbage_and_a_cut_frame_are_skipped_and_the_next_frame_is_read() {
        let mut good = Vec::new();
        encode(&mut good, 42, 100, &hex(SHORT));
        let mut cut = Vec::new();
        encode(&mut cut, 1, 1, &hex(LONG));
        cut.truncate(12);

        let mut wire = vec![0x55, 0xAA, 0x1A, 0x1A, 0x00];
        wire.extend(&cut);
        wire.extend(&good);
        let mut p = Parser::new();
        let got = p.push(&wire);
        assert_eq!(got.len(), 1);
        assert_eq!((got[0].timestamp, got[0].bytes.clone()), (42, hex(SHORT)));
        assert_eq!(p.skipped as usize, 5 + cut.len());
    }

    /// A record of a type this reader does not know is passed over without
    /// losing the frame behind it.
    #[test]
    fn an_unknown_record_is_passed_over() {
        let mut wire = vec![0x1A, b'4', 1, 2, 3, 4, 5, 6, 7];
        encode(&mut wire, 5, 5, &hex(SHORT));
        let got = Parser::new().push(&wire);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].bytes, hex(SHORT));
    }

    /// `readsb --net-receiver-id`: the ID goes with the frame after it, and
    /// only that one.
    #[test]
    fn a_receiver_id_belongs_to_the_next_frame() {
        let mut wire = vec![0x1A, 0xE3, 0, 0, 0, 0, 0, 0x1A, 0x1A, 0, 9];
        encode(&mut wire, 1, 1, &hex(SHORT));
        encode(&mut wire, 2, 2, &hex(SHORT));
        let got = Parser::new().push(&wire);
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].receiver, Some(0x1A_0009));
        assert_eq!(got[1].receiver, None);
    }

    /// A message of no Beast length is refused rather than written as
    /// something a reader would misread.
    #[test]
    fn a_message_of_the_wrong_length_is_not_written() {
        let mut wire = Vec::new();
        assert!(!encode(&mut wire, 0, 1, &[1, 2, 3]));
        assert!(wire.is_empty());
    }

    /// The signal byte both ways: full scale is 255, the byte is amplitude, and
    /// the conversion back lands within the byte's resolution — which is coarse
    /// below -30 dBFS, where a step of one is more than a decibel.
    #[test]
    fn the_signal_byte_is_amplitude_on_a_255_scale() {
        assert_eq!(dbfs_to_signal(0.0), 255);
        assert_eq!(dbfs_to_signal(-8.1308), 100, "amplitude 100/255");
        assert_eq!(dbfs_to_signal(-200.0), 1, "never zero, which means no level");
        assert_eq!(signal_to_dbfs(0), None);
        assert_eq!(signal_to_dbfs(255), Some(0.0));
        for db in [-3.0f32, -12.5, -20.0, -30.0] {
            let back = signal_to_dbfs(dbfs_to_signal(db)).unwrap();
            assert!((back - db).abs() < 0.9, "{db} dBFS came back as {back}");
        }
    }

    /// Samples become 12 MHz ticks: one second at 2.4 Msps is twelve million.
    #[test]
    fn a_second_of_samples_is_twelve_million_ticks() {
        assert_eq!(timestamp_from_samples(2_400_000, 2_400_000.0), 12_000_000);
        assert_eq!(timestamp_from_samples(1, 0.0), 0);
    }
}
