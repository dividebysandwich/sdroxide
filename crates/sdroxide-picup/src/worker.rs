//! The thread uploads run on, so a slow or dead server never stalls the
//! engine that hands it pictures.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use crossbeam_channel::{Receiver, Sender, TrySendError, bounded, unbounded};
use sdroxide_types::{ImageKind, PictureUploadConfig, PictureUploadStatus, UploadNaming};
use tracing::{debug, warn};

use crate::ftp::{Failure, Session};
use crate::rotate::{self, Op};
use crate::stamp::{self, Caption};

/// Pictures waiting while one is going up. Small on purpose: if the server
/// cannot keep up, the newest pictures are the ones worth having.
const QUEUE: usize = 4;
/// How long closing the program waits for the "offline" marker.
const OFFLINE_WAIT: Duration = Duration::from_secs(5);
/// What a picture is uploaded as before it is renamed into place, so a web
/// page never shows half of one.
const PART: &str = "sdroxide-upload.part";
const TEST_FILE: &str = "sdroxide-test.txt";
const CHECKNEW: &str = "checknew.jpg";

enum Job {
    Picture { kind: ImageKind, png: Vec<u8>, name: String, caption: Caption },
    Test(Box<PictureUploadConfig>),
    Offline(Sender<()>),
}

/// Owns the upload thread. Dropping it lets the thread finish what it is doing
/// and exit.
pub struct PictureUploader {
    cfg: Arc<Mutex<PictureUploadConfig>>,
    tx: Sender<Job>,
    /// A second handle on the queue, so a full one can lose its oldest job.
    rx: Receiver<Job>,
    results: Receiver<PictureUploadStatus>,
}

impl Default for PictureUploader {
    fn default() -> Self {
        Self::new()
    }
}

impl PictureUploader {
    pub fn new() -> PictureUploader {
        let cfg = Arc::new(Mutex::new(PictureUploadConfig::default()));
        let (tx, rx) = bounded::<Job>(QUEUE);
        let (res_tx, results) = unbounded();
        let (thread_cfg, thread_rx) = (cfg.clone(), rx.clone());
        let spawned = std::thread::Builder::new()
            .name("picture-upload".into())
            .spawn(move || run(thread_cfg, thread_rx, res_tx));
        if let Err(e) = spawned {
            warn!("picture upload thread: {e}");
        }
        PictureUploader { cfg, tx, rx, results }
    }

    pub fn set_config(&self, cfg: PictureUploadConfig) {
        if let Ok(mut c) = self.cfg.lock() {
            *c = cfg;
        }
    }

    fn config(&self) -> PictureUploadConfig {
        self.cfg.lock().map(|c| c.clone()).unwrap_or_default()
    }

    /// Whether a picture of this kind would be uploaded at all — so the engine
    /// can skip the copy when it would not.
    pub fn wants(&self, kind: ImageKind) -> bool {
        let cfg = self.config();
        cfg.enabled
            && cfg.is_usable()
            && match kind {
                ImageKind::Sstv => cfg.upload_sstv,
                ImageKind::Wefax => cfg.upload_wefax,
            }
    }

    /// Queue a picture just saved as `name`.
    pub fn picture(&self, kind: ImageKind, png: Vec<u8>, name: String, caption: Caption) {
        self.push(Job::Picture { kind, png, name, caption });
    }

    /// Try `cfg` without adopting it; the answer comes back through
    /// [`Self::poll`] with `is_test` set.
    pub fn test(&self, cfg: PictureUploadConfig) {
        self.push(Job::Test(Box::new(cfg)));
    }

    /// Upload QSSTV's "offline" marker and wait a little for it. Called when
    /// the program closes, so it never waits long.
    pub fn offline(&self) {
        let cfg = self.config();
        if !(cfg.enabled && cfg.heartbeat && cfg.is_usable()) {
            return;
        }
        let (done_tx, done_rx) = bounded(1);
        self.push(Job::Offline(done_tx));
        let _ = done_rx.recv_timeout(OFFLINE_WAIT);
    }

    pub fn poll(&self) -> Vec<PictureUploadStatus> {
        self.results.try_iter().collect()
    }

    fn push(&self, mut job: Job) {
        loop {
            match self.tx.try_send(job) {
                Ok(()) => return,
                Err(TrySendError::Full(j)) => {
                    if self.rx.try_recv().is_ok() {
                        warn!("picture upload: the server is not keeping up; dropped the oldest");
                    }
                    job = j;
                }
                Err(TrySendError::Disconnected(_)) => return,
            }
        }
    }
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

fn run(cfg: Arc<Mutex<PictureUploadConfig>>, rx: Receiver<Job>, out: Sender<PictureUploadStatus>) {
    for job in rx.iter() {
        let current = cfg.lock().map(|c| c.clone()).unwrap_or_default();
        let status = match job {
            Job::Picture { kind, png, name, caption } => {
                if !current.enabled || !current.is_usable() {
                    continue;
                }
                match upload(&current, kind, &png, &name, &caption) {
                    Ok(message) => ok(message, None, false),
                    Err(message) => fail(message, None, false),
                }
            }
            Job::Test(test_cfg) => match test(&test_cfg) {
                Ok((message, fp)) => ok(message, fp, true),
                Err(f) => fail(f.message, f.fingerprint, true),
            },
            Job::Offline(done) => {
                if let Err(e) = offline(&current) {
                    debug!("picture upload offline marker: {e}");
                }
                let _ = done.send(());
                continue;
            }
        };
        if !status.ok {
            warn!("picture upload: {}", status.message);
        }
        if out.send(status).is_err() {
            return;
        }
    }
}

fn ok(message: String, fingerprint: Option<String>, is_test: bool) -> PictureUploadStatus {
    PictureUploadStatus { ok: true, message, unix: now(), fingerprint, is_test }
}

fn fail(message: String, fingerprint: Option<String>, is_test: bool) -> PictureUploadStatus {
    PictureUploadStatus { ok: false, message, unix: now(), fingerprint, is_test }
}

fn dir_for(cfg: &PictureUploadConfig, kind: ImageKind) -> &str {
    match kind {
        ImageKind::Sstv => &cfg.sstv_dir,
        ImageKind::Wefax => &cfg.wefax_dir,
    }
}

/// The directories this configuration uploads to, each once.
fn dirs(cfg: &PictureUploadConfig) -> Vec<&str> {
    let mut v = Vec::new();
    if cfg.upload_sstv {
        v.push(cfg.sstv_dir.trim());
    }
    if cfg.upload_wefax && !v.contains(&cfg.wefax_dir.trim()) {
        v.push(cfg.wefax_dir.trim());
    }
    if v.is_empty() {
        v.push("");
    }
    v
}

fn upload(
    cfg: &PictureUploadConfig,
    kind: ImageKind,
    png: &[u8],
    name: &str,
    caption: &Caption,
) -> Result<String, String> {
    let bytes = stamp::prepare(png, cfg.stamp.then_some(caption), cfg.downscale, cfg.format)?;
    let dir = dir_for(cfg, kind);
    let (mut s, _) = Session::open(cfg, dir).map_err(|f| f.message)?;
    let result = (|| -> Result<String, String> {
        if cfg.heartbeat {
            s.put(CHECKNEW, &stamp::checknew(1, caption.unix))?;
        }
        s.put(PART, &bytes)?;
        let ext = cfg.format.extension();
        let target = match cfg.naming {
            UploadNaming::Sequence => {
                let ext = if cfg.add_extension { format!(".{ext}") } else { String::new() };
                let listing = s.names()?;
                for op in rotate::plan(&listing, cfg.keep_images, &ext) {
                    match op {
                        Op::Delete(n) => s.delete(&n)?,
                        Op::Rename(a, b) => s.rename(&a, &b)?,
                    }
                }
                rotate::slot_name(1, &ext)
            }
            UploadNaming::Original => {
                let stem = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
                format!("{stem}.{ext}")
            }
        };
        if s.rename(PART, &target).is_err() {
            // A server that will not rename onto an existing name.
            let _ = s.delete(&target);
            s.rename(PART, &target)?;
        }
        Ok(target)
    })();
    s.quit();
    let target = result?;
    let place = if dir.trim().is_empty() { String::new() } else { format!("{}/", dir.trim()) };
    Ok(format!("uploaded {place}{target}"))
}

/// Log in, and in every directory that would be used put and delete a small
/// file — the same rights an upload needs, without touching any picture.
fn test(cfg: &PictureUploadConfig) -> Result<(String, Option<String>), Failure> {
    if !cfg.is_usable() {
        return Err(Failure { message: "no server set".into(), fingerprint: None });
    }
    let (mut s, fp) = Session::open(cfg, "")?;
    let failure = |message: String| Failure { message, fingerprint: fp.clone() };
    let home = s.pwd().map_err(failure)?;
    for dir in dirs(cfg) {
        let r = (|| -> Result<(), String> {
            if !dir.is_empty() {
                s.cwd(dir)?;
            }
            s.put(TEST_FILE, b"sdroxide picture upload test\n")?;
            s.delete(TEST_FILE)?;
            s.cwd(&home)
        })();
        if let Err(e) = r {
            s.quit();
            return Err(failure(e));
        }
    }
    s.quit();
    let tls = if cfg.security.is_tls() { " over TLS" } else { "" };
    Ok((format!("logged in{tls} and wrote to the upload directory"), fp))
}

/// QSSTV's "offline" marker: `checknew.jpg` two pixels wide.
fn offline(cfg: &PictureUploadConfig) -> Result<(), String> {
    let (mut s, _) = Session::open(cfg, "").map_err(|f| f.message)?;
    let home = s.pwd()?;
    let marker = stamp::checknew(2, now());
    let r = (|| -> Result<(), String> {
        for dir in dirs(cfg) {
            if !dir.is_empty() {
                s.cwd(dir)?;
            }
            s.put(CHECKNEW, &marker)?;
            s.cwd(&home)?;
        }
        Ok(())
    })();
    s.quit();
    r
}
