//! Taking an FDM-DUO out of stand-alone, which is what makes it stream wider
//! than 192 kHz.
//!
//! # What stand-alone is
//!
//! The DUO boots its own FPGA, and the image it boots is the stand-alone one:
//! the radio's own receiver and front panel, with a 192 kHz I/Q channel on the
//! side. Every wider rate is a *remote* image that the host uploads over USB,
//! and the radio then streams at that rate. Nothing in the radio's menus
//! selects a rate, which is why a DUO left alone reads 192 kHz whatever SPAN
//! asks for.
//!
//! # Where this comes from
//!
//! Four USB captures of SDR Console opening the DUO at 384, 768, 1536, 3072
//! and 6144 kHz, on the owner's own radio. Every request below
//! is one that capture contains, in its order:
//!
//! ```text
//! E1 idx F600 in 1        → F6
//! E1 idx F900 in 2        → F9 00
//! E1 val 1 idx E300 in 1  → E3            begin the load
//! E1 val n idx E400 out n                 the image, 64 bytes a request,
//!                                         then its last byte on its own
//! E1 idx E500 in 2        → E5 10         the load is done
//! E1 idx FD00 in 3        → FD 32 31
//! E1 F2 tune, data [word >> 24, 0x02]     0x02 rather than stand-alone's 0x00
//! CAT CF…; IQ1;  tune again;  E1 E9 val 1 start
//! CAT IQ1; about every eight seconds while streaming, IQ0; on the way out
//! ```
//!
//! Ported from AJ3G's custom ELAD control and signal analysis software
//! (`crates/elad-iq/src/remote.rs`), whose ELAD driver is itself a port of
//! this crate, and where the sequence was worked out from USB captures of
//! SDR Console and FDM-SW2 and run on an FDM-DUO at every rate. One
//! difference from SDR Console is kept from
//! there: `IQ1;` (the DUO's I/Q *transmit* mode, CAT manual v2.13 p51) is never
//! sent. Stopping and re-initialising the FIFO after the load is what makes a
//! wide image stream, and `IQ0;` is sent after every load and on close.
//!
//! # The images are ELAD's
//!
//! They are not in this crate and must not be. They are read at run time out
//! of `ExtIOmc_ELAD_FDMDUO.dll`, the ELAD driver that SDR Console and FDM-SW2
//! install, found by scanning it for the Xilinx sync word and checked against
//! the SHA-256 of the exact bytes SDR Console uploaded. Two builds of that DLL
//! have the images at different offsets, which is why it is the hash that is
//! trusted and never a position.
//!
//! # Not while transmitting
//!
//! Reloading the FPGA reconfigures the radio, and its transmitter goes through
//! that FPGA as well. No load or restore starts while the source says the
//! radio is transmitting, and a load that sees a transmission begin stops
//! between two requests. See [`set_transmitting`].
//!
//! # Back to stand-alone
//!
//! A DUO left on a wide image stays there after the program that loaded it
//! has gone, and every other program (FDM-SW2, WSJT-X on the DUO's own audio)
//! then finds a radio that streams at a rate it does not expect. So going back
//! to 192 kHz, and closing sdroxide, put it back on its own image: `E3` with
//! [`RESTORE_BEGIN`] and no upload, as FDM-SW2 does.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// The rate the DUO streams at without a host-loaded image.
pub const STANDALONE_RATE_HZ: u32 = 192_000;

/// How long the FPGA is given to clear itself after `E3` before the image
/// goes in. SDR Console waits 205 to 222 ms in every capture.
pub const ERASE_WAIT: Duration = Duration::from_millis(250);

/// The quiet after a load before the first tune. SDR Console keeps about
/// 1.05 s; FDM-SW2 sends `F9` 0.5 s after `E5` and its first CAT 0.9 s after.
pub const SETTLE_BEFORE_TUNE: Duration = Duration::from_millis(1100);
/// The wait between the front-end commands and the stream's start after a
/// load. SDR Console takes 2.3 to 2.8 s from the load to the start, but
/// FDM-SW2 starts the FIFO 0.1 s after its first tune (SW2_Capture 1), so
/// this was cut from 1.5 s.
pub const SETTLE_BEFORE_START: Duration = Duration::from_millis(300);

/// How many times an image is sent before the open gives up.
pub const LOAD_ATTEMPTS: u32 = 2;

/// Whether the `E5` end-of-load answer says the image configured. SDR Console
/// always got `E5 10`; a load sent without [`ERASE_WAIT`] got `E5 00` and left
/// the radio with no receiver until it was power-cycled.
pub fn load_done(reply: &[u8]) -> bool {
    reply.get(1).is_some_and(|b| b & 0x10 != 0)
}

/// Bytes per upload request.
pub const CHUNK: usize = 64;

/// The name of ELAD's driver DLL, which carries the images.
pub const DLL_NAME: &str = "ExtIOmc_ELAD_FDMDUO.dll";

/// An environment variable naming the DLL, for an install somewhere else.
pub const DLL_ENV: &str = "SDROXIDE_ELAD_DLL";

/// Where the DLL is looked for after [`DLL_ENV`]: SDR Console's folder, then
/// FDM-SW2's under both program folders.
const DLL_DIRS: &[&str] = &[
    r"C:\Program Files\SDR-Radio.com (V3)",
    r"C:\Program Files (x86)\ELAD\ELAD FDM-SW2",
    r"C:\Program Files\ELAD\ELAD FDM-SW2",
];

/// The Xilinx sync word as it sits in ELAD's images (bit-reversed bytes).
const SYNC: [u8; 4] = [0x55, 0x99, 0xAA, 0x66];

/// One remote image this crate can recognise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Image {
    pub rate_hz: u32,
    /// Bytes uploaded: sixteen `0xFF`, the image from its sync word, and one
    /// more `0xFF`.
    pub upload_len: usize,
    /// SHA-256 of those bytes, as SDR Console sent them.
    pub sha256: [u8; 32],
}

/// The images seen in the captures, the rates named by the bandwidth chosen in
/// SDR Console as each was loaded. Nothing on the wire says which rate an image
/// is, so each is tied to one by a capture. All five sit in SDR Console's copy
/// of the DLL in rising order of rate.
///
/// The 192 kHz entry is the DUO's own stand-alone image (the DLL's "default
/// stand-alone and single channel" configuration, first in the DLL). It was
/// not in a capture: it is found by the same end-of-image rule that gives all
/// five captured images their exact bytes (AJ3G's DLL, 25 Oct). Loaded from
/// here it does not put the radio back in stand-alone: it stayed in remote
/// and streamed zeros (AJ3G, 25 Oct), so nothing loads it now.
pub const IMAGES: [Image; 6] = [
    Image {
        rate_hz: STANDALONE_RATE_HZ,
        upload_len: 802_019,
        sha256: hex32("b96188c54b50ca169c470ad067d7d8f1809afab779a3336beacd1930bdd05875"),
    },
    Image {
        rate_hz: 384_000,
        upload_len: 802_295,
        sha256: hex32("d9a7e0913cc6216e3b1877d3373ae8897204ab6c5498fdb645fbe68564b3525f"),
    },
    Image {
        rate_hz: 768_000,
        upload_len: 802_295,
        sha256: hex32("cd1c73a6f3acd672bc726e58282c3610098210c121d897b3f59d5d7acfa3a554"),
    },
    Image {
        rate_hz: 1_536_000,
        upload_len: 802_571,
        sha256: hex32("1d5efa829738e7a1f96c6e2cb1cb405403bbde6358cd98c86c2b973cf837f2c6"),
    },
    Image {
        rate_hz: 3_072_000,
        upload_len: 801_463,
        sha256: hex32("3a06429c44b6769ee25f29ab48656dccb25b669e16821eb8a4a4c644fa8918db"),
    },
    Image {
        rate_hz: 6_144_000,
        upload_len: 801_463,
        sha256: hex32("d37cdae628d361b85e1a954ca14d194490e797439931463041580746646972ed"),
    },
];

/// The image for `rate_hz`, if one is known.
pub fn image(rate_hz: u32) -> Option<Image> {
    IMAGES.iter().copied().find(|i| i.rate_hz == rate_hz)
}

/// Whether the DUO needs a remote image for `rate_hz`.
pub fn wanted(rate_hz: u32) -> bool {
    rate_hz > STANDALONE_RATE_HZ
}

static TRANSMITTING: AtomicBool = AtomicBool::new(false);

/// Tell this crate whether the radio is transmitting, as often as that is
/// known. A load does not start while it is, and stops if it starts.
pub fn set_transmitting(on: bool) {
    TRANSMITTING.store(on, Ordering::Relaxed);
}

pub fn transmitting() -> bool {
    TRANSMITTING.load(Ordering::Relaxed)
}

/// Whether the radio is (or may be) in remote mode, so a later open at 192 kHz
/// knows to put it back on its own image first: set when this process loads a
/// wide image, and when a stream read at 192 kHz turns out to be arriving at
/// another rate — a DUO left on a wide image by another program.
static WENT_REMOTE: AtomicBool = AtomicBool::new(false);

pub(crate) fn note_remote() {
    WENT_REMOTE.store(true, Ordering::Relaxed);
}

pub(crate) fn went_remote() -> bool {
    WENT_REMOTE.load(Ordering::Relaxed)
}

/// After the radio has gone back to its own image.
pub(crate) fn forget_remote() {
    WENT_REMOTE.store(false, Ordering::Relaxed);
    LOADED_HZ.store(0, Ordering::Relaxed);
}

/// The wide image this process last put in the radio, 0 for none (or after a
/// restore). Only for the diagnostic report.
static LOADED_HZ: AtomicU32 = AtomicU32::new(0);

/// After a wide image has been loaded and acknowledged.
pub(crate) fn note_loaded(rate_hz: u32) {
    LOADED_HZ.store(rate_hz, Ordering::Relaxed);
}

/// The FDM-DUO's lines for the diagnostic report, in place of the FDM-S
/// loader's: the DUO boots its own 192 kHz FPGA image and never uses
/// `elad-firmware`; wider spans are images taken from ELAD's DLL.
pub fn status_line() -> String {
    let image = match LOADED_HZ.load(Ordering::Relaxed) {
        0 if went_remote() => "radio may still be on a wide image (restore pending)".to_string(),
        0 => format!(
            "radio's own stand-alone image ({} kHz) — nothing loaded by sdroxide",
            STANDALONE_RATE_HZ / 1000
        ),
        hz => format!("{} kHz image loaded by sdroxide this session", hz / 1000),
    };
    let dll = match dll_paths().into_iter().next() {
        Some(p) => format!("{DLL_NAME}: {}", p.display()),
        None => format!(
            "{DLL_NAME}: not found (only needed for spans above {} kHz)",
            STANDALONE_RATE_HZ / 1000
        ),
    };
    format!("{image}\n{dll}")
}

/// Whether a restore has already been asked for because the stream arrived at
/// the wrong rate. Once per process: a DUO that is still wrong after one has
/// something else the matter with it, and reopening it again and again would
/// hide that behind a radio that never settles.
static RESTORE_ASKED: AtomicBool = AtomicBool::new(false);

/// Ask for one restore to stand-alone, because a stream read at 192 kHz is
/// arriving at another rate. `false` if one has been asked for already.
pub(crate) fn ask_restore() -> bool {
    if RESTORE_ASKED.swap(true, Ordering::Relaxed) {
        return false;
    }
    note_remote();
    true
}

/// The `E3` `wValue` that has the DUO reconfigure from its own stand-alone
/// image instead of taking an upload. FDM-SW2 sends it going back to 192 kHz,
/// with no image after it, then polls `E5` until it answers `E5 10`; the DUO
/// was back in STD ALONE without a power cycle (AJ3G, 25 Oct, SW2_Capture 1).
pub const RESTORE_BEGIN: u16 = 0x0100;

/// How often `E5` is asked while the DUO restores itself. FDM-SW2 asks about
/// every 0.5 s and got `E5 10` on the eighth, 3.6 s after `E3`.
pub const RESTORE_POLL: Duration = Duration::from_millis(500);

/// How long the restore is given before the open gives up on it.
pub const RESTORE_TIMEOUT: Duration = Duration::from_secs(12);

/// Images already pulled out of the DLL in this process.
static CACHE: Mutex<Vec<(u32, Arc<Vec<u8>>)>> = Mutex::new(Vec::new());

/// The upload for `rate_hz`, read from the installed DLL, or the sentence the
/// operator needs.
pub fn upload_for(rate_hz: u32) -> Result<Arc<Vec<u8>>, String> {
    let Some(img) = image(rate_hz) else {
        return Err(format!(
            "The FDM-DUO's {} kHz image is not known, so it stays at {} kHz.",
            rate_hz / 1000,
            STANDALONE_RATE_HZ / 1000
        ));
    };
    let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((_, u)) = cache.iter().find(|(r, _)| *r == rate_hz) {
        return Ok(Arc::clone(u));
    }
    let dlls = dll_paths();
    for path in &dlls {
        let Ok(bytes) = std::fs::read(path) else { continue };
        if let Some(u) = find_upload(&bytes, &img) {
            let u = Arc::new(u);
            cache.push((rate_hz, Arc::clone(&u)));
            return Ok(u);
        }
    }
    Err(if dlls.is_empty() {
        format!(
            "To stream wider than {} kHz the FDM-DUO needs ELAD's {DLL_NAME}, which SDR Console \
             or FDM-SW2 installs on Windows; it was not found, so it stays at {} kHz. On Linux \
             or macOS, copy that file into sdroxide's configuration folder.",
            STANDALONE_RATE_HZ / 1000,
            STANDALONE_RATE_HZ / 1000
        )
    } else {
        format!(
            "{DLL_NAME} was found but has no {} kHz image this program recognises, so the \
             FDM-DUO stays at {} kHz.",
            rate_hz / 1000,
            STANDALONE_RATE_HZ / 1000
        )
    })
}

/// The DLLs that exist, in the order they are tried.
/// A folder the host program also looks in — sdroxide's own configuration
/// folder. On Linux and macOS, where neither ELAD program installs, a copy of
/// the DLL dropped there is all the wide rates need: only its bytes are read,
/// it is never loaded as code.
static EXTRA_DIR: std::sync::Mutex<Option<PathBuf>> = std::sync::Mutex::new(None);

/// Also look for the DLL in `dir`.
pub fn set_dll_dir(dir: PathBuf) {
    *EXTRA_DIR.lock().unwrap_or_else(|e| e.into_inner()) = Some(dir);
}

fn dll_paths() -> Vec<PathBuf> {
    let env = std::env::var_os(DLL_ENV).map(PathBuf::from);
    let extra = EXTRA_DIR.lock().unwrap_or_else(|e| e.into_inner()).clone();
    let exe_dir = std::env::current_exe().ok().and_then(|e| e.parent().map(PathBuf::from));
    env.into_iter()
        .chain(extra.into_iter().chain(exe_dir).map(|d| d.join(DLL_NAME)))
        .chain(DLL_DIRS.iter().map(|d| PathBuf::from(d).join(DLL_NAME)))
        .filter(|p| p.is_file())
        .collect()
}

/// The upload for `img`, if `dll` holds it.
///
/// Every sync word is a candidate; the one whose bytes hash right is the image.
pub fn find_upload(dll: &[u8], img: &Image) -> Option<Vec<u8>> {
    let body_len = img.upload_len.checked_sub(17)?;
    let mut from = 0;
    while let Some(at) = dll[from..].windows(4).position(|w| w == SYNC).map(|p| p + from) {
        from = at + 1;
        let Some(body) = dll.get(at..at + body_len) else { break };
        let mut u = Vec::with_capacity(img.upload_len);
        u.extend_from_slice(&[0xFF; 16]);
        u.extend_from_slice(body);
        u.push(0xFF);
        if sha256(&u) == img.sha256 {
            return Some(u);
        }
    }
    None
}

/// The upload as SDR Console splits it: 64-byte requests, a short one for the
/// rest, and the last byte alone.
pub fn chunks(upload: &[u8]) -> impl Iterator<Item = &[u8]> {
    let (body, last) = upload.split_at(upload.len().saturating_sub(1));
    body.chunks(CHUNK).chain(std::iter::once(last).filter(|l| !l.is_empty()))
}

const fn hex32(s: &str) -> [u8; 32] {
    const fn nib(c: u8) -> u8 {
        match c {
            b'0'..=b'9' => c - b'0',
            b'a'..=b'f' => c - b'a' + 10,
            _ => panic!("not lower-case hex"),
        }
    }
    let b = s.as_bytes();
    assert!(b.len() == 64);
    let mut out = [0u8; 32];
    let mut i = 0;
    while i < 32 {
        out[i] = nib(b[2 * i]) << 4 | nib(b[2 * i + 1]);
        i += 1;
    }
    out
}

/// SHA-256, so checking an image needs no crate for one hash per open.
pub fn sha256(data: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&((data.len() as u64) * 8).to_be_bytes());
    for block in msg.chunks_exact(64) {
        let mut w = [0u32; 64];
        for (i, word) in block.chunks_exact(4).enumerate() {
            w[i] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let t1 = hh.wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (x, y) in h.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
            *x = x.wrapping_add(y);
        }
    }
    let mut out = [0u8; 32];
    for (i, x) in h.iter().enumerate() {
        out[4 * i..4 * i + 4].copy_from_slice(&x.to_be_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_e5_10_is_a_finished_load() {
        assert!(load_done(&[0xE5, 0x10]));
        assert!(!load_done(&[0xE5, 0x00]));
        assert!(!load_done(&[0xE5]));
    }

    #[test]
    fn the_hash_is_sha_256() {
        assert_eq!(
            sha256(b""),
            hex32("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")
        );
        assert_eq!(
            sha256(b"abc"),
            hex32("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")
        );
        // Two blocks, so the length lands in the second.
        assert_eq!(
            sha256(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
            hex32("248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1")
        );
    }

    /// A made-up image behind a decoy sync word: the decoy hashes wrong and is
    /// passed over, and the real one comes back framed the way it is sent.
    #[test]
    fn the_image_is_the_sync_word_whose_bytes_hash_right() {
        let body: Vec<u8> = SYNC.iter().copied().chain((0..300u32).map(|i| i as u8)).collect();
        let mut want = vec![0xFF; 16];
        want.extend_from_slice(&body);
        want.push(0xFF);
        let img = Image { rate_hz: 1, upload_len: want.len(), sha256: sha256(&want) };

        let mut dll = vec![0u8; 50];
        dll.extend_from_slice(&SYNC);
        dll.extend_from_slice(&[7u8; 400]);
        dll.extend_from_slice(&body);
        dll.extend_from_slice(&[9u8; 20]);
        assert_eq!(find_upload(&dll, &img), Some(want));

        // Cut short before the image ends: nothing, rather than a slice past
        // the end.
        let short = &dll[..dll.len() - 25];
        assert_eq!(find_upload(short, &img), None);
    }

    /// SDR Console's split of the 1536 kHz image: 12,540 full requests, ten
    /// bytes, then the closing `0xFF` alone.
    #[test]
    fn the_upload_goes_in_sixty_four_byte_requests_and_its_last_byte_alone() {
        let u = vec![0u8; 802_571];
        let lens: Vec<usize> = chunks(&u).map(<[u8]>::len).collect();
        assert_eq!(lens.len(), 12_542);
        assert!(lens[..12_540].iter().all(|&l| l == 64));
        assert_eq!(&lens[12_540..], &[10, 1]);
        // The 802,295-byte one ends 54 then 1.
        let u = vec![0u8; 802_295];
        let lens: Vec<usize> = chunks(&u).map(<[u8]>::len).collect();
        assert_eq!(lens.len(), 12_537);
        assert_eq!(&lens[12_535..], &[54, 1]);
    }

    #[test]
    fn every_rate_above_stand_alone_has_its_own_image() {
        assert!(!wanted(192_000));
        assert!(wanted(384_000));
        assert_eq!(image(1_536_000).map(|i| i.upload_len), Some(802_571));
        assert_eq!(image(3_072_000).map(|i| i.upload_len), Some(801_463));
        assert_eq!(image(6_144_000).map(|i| i.upload_len), Some(801_463));
        assert_eq!(image(384_000).map(|i| i.upload_len), Some(802_295));
        assert_eq!(image(768_000).map(|i| i.upload_len), Some(802_295));
        // Every rate above stand-alone has one now, and no two share a hash.
        for r in sdroxide_types::ELAD_SAMPLE_RATES.into_iter().filter(|&r| wanted(r)) {
            assert!(image(r).is_some(), "{r}");
        }
        let mut h: Vec<_> = IMAGES.iter().map(|i| i.sha256).collect();
        h.sort();
        h.dedup();
        assert_eq!(h.len(), IMAGES.len());
        let e = upload_for(2_000_000).unwrap_err();
        assert!(e.contains("2000 kHz") && e.contains("192 kHz"), "{e}");
    }
}
