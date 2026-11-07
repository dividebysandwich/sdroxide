//! Pushing received pictures to an FTP server (`picupload.json`).
//!
//! The QSSTV feature of the same purpose, so a station's web page can show
//! what it is hearing: every SSTV picture (and, optionally, every weather
//! chart) goes up as it is saved, with the same `image1`…`imageN` rotation,
//! header stamp and `checknew.jpg` heartbeat the pages written for QSSTV
//! already expect.
//!
//! Engine-owned like the rest of [`crate::StationConfig`]: the pictures are
//! decoded and saved on the machine the radio is attached to, so that is
//! where they are sent from, whichever screen set it up.

use serde::{Deserialize, Serialize};

/// How the control connection is protected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum FtpSecurity {
    /// Plain FTP. The password crosses the network as typed.
    Plain,
    /// Explicit FTPS: connect in the clear, then `AUTH TLS` before logging in.
    /// What almost every server that offers TLS at all offers, on port 21.
    #[default]
    ExplicitTls,
    /// Implicit FTPS: TLS from the first byte, conventionally on port 990.
    ImplicitTls,
}

impl FtpSecurity {
    pub const ALL: [FtpSecurity; 3] =
        [FtpSecurity::Plain, FtpSecurity::ExplicitTls, FtpSecurity::ImplicitTls];

    pub fn label(self) -> &'static str {
        match self {
            FtpSecurity::Plain => "FTP (no encryption)",
            FtpSecurity::ExplicitTls => "FTPS explicit (AUTH TLS)",
            FtpSecurity::ImplicitTls => "FTPS implicit",
        }
    }

    /// The port a server offering this conventionally listens on.
    pub fn default_port(self) -> u16 {
        match self {
            FtpSecurity::ImplicitTls => 990,
            _ => 21,
        }
    }

    pub fn is_tls(self) -> bool {
        self != FtpSecurity::Plain
    }
}

/// How the server's certificate is judged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TlsVerify {
    /// The operating system's trust store, as a browser on this machine would
    /// judge it, plus [`PictureUploadConfig::extra_ca_pem`] when one is given.
    #[default]
    System,
    /// Exactly one certificate, named by the SHA-256 of its DER encoding. The
    /// way to trust a self-signed home server without trusting anything else.
    Pinned,
    /// Any certificate at all. The connection is still encrypted, but nothing
    /// stops somebody in the path from reading the password.
    Off,
}

impl TlsVerify {
    pub const ALL: [TlsVerify; 3] = [TlsVerify::System, TlsVerify::Pinned, TlsVerify::Off];

    pub fn label(self) -> &'static str {
        match self {
            TlsVerify::System => "System trust store",
            TlsVerify::Pinned => "Pinned fingerprint",
            TlsVerify::Off => "Don't verify",
        }
    }
}

/// What the uploaded file is called.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum UploadNaming {
    /// QSSTV's image sequence: the newest is `image1`, the one before it
    /// `image2`, and so on to [`PictureUploadConfig::keep_images`].
    #[default]
    Sequence,
    /// The name the picture was filed under locally (`sstv-<ms>.png`), so the
    /// server keeps everything.
    Original,
}

impl UploadNaming {
    pub const ALL: [UploadNaming; 2] = [UploadNaming::Sequence, UploadNaming::Original];

    pub fn label(self) -> &'static str {
        match self {
            UploadNaming::Sequence => "image1 … imageN (rotate)",
            UploadNaming::Original => "Original file name",
        }
    }
}

/// The encoding the picture is uploaded in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum UploadFormat {
    #[default]
    Png,
    Jpeg,
}

impl UploadFormat {
    pub const ALL: [UploadFormat; 2] = [UploadFormat::Png, UploadFormat::Jpeg];

    pub fn label(self) -> &'static str {
        match self {
            UploadFormat::Png => "PNG",
            UploadFormat::Jpeg => "JPEG",
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            UploadFormat::Png => "png",
            UploadFormat::Jpeg => "jpg",
        }
    }
}

/// Everything the picture upload needs (`picupload.json`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PictureUploadConfig {
    /// Off by default: publishing what the station hears is the operator's
    /// decision.
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    pub security: FtpSecurity,
    /// Only consulted when [`Self::security`] is a TLS one.
    pub verify: TlsVerify,
    /// PEM certificates added to the system roots in [`TlsVerify::System`] —
    /// a private CA the operating system does not know about. Pasted rather
    /// than a path, so a remote screen can set it without a file on the
    /// station's disk.
    pub extra_ca_pem: String,
    /// The SHA-256 of the server's certificate for [`TlsVerify::Pinned`], in
    /// hex. Colons, spaces and case are ignored.
    pub pinned_sha256: String,
    pub user: String,
    /// Stored in the clear, as every other credential in the station config is.
    pub password: String,
    /// Ignore the address a passive-mode reply names when it is a private one
    /// and use the server's own instead — what a server behind NAT that does
    /// not know its public address needs.
    pub passive_nat_workaround: bool,
    /// SSTV pictures, including RIFP ones (they share the store).
    pub upload_sstv: bool,
    /// Remote directory for SSTV pictures; empty is the login directory.
    pub sstv_dir: String,
    pub upload_wefax: bool,
    /// Remote directory for weather charts; empty is the login directory.
    pub wefax_dir: String,
    pub naming: UploadNaming,
    /// How many pictures the `image1`…`imageN` rotation keeps.
    pub keep_images: u16,
    /// Name the rotated files `image1.png` rather than QSSTV's bare `image1`.
    pub add_extension: bool,
    pub format: UploadFormat,
    /// Draw QSSTV's black caption bar — time, frequency, mode and callsign —
    /// across the top of the uploaded copy. The stored picture is untouched.
    pub stamp: bool,
    /// Shrink the uploaded copy to fit 960 × 768, as QSSTV does.
    pub downscale: bool,
    /// Upload QSSTV's `checknew.jpg` beside every picture, and its "offline"
    /// marker when the program closes.
    pub heartbeat: bool,
}

impl Default for PictureUploadConfig {
    fn default() -> Self {
        PictureUploadConfig {
            enabled: false,
            host: String::new(),
            port: 21,
            security: FtpSecurity::default(),
            verify: TlsVerify::default(),
            extra_ca_pem: String::new(),
            pinned_sha256: String::new(),
            user: String::new(),
            password: String::new(),
            passive_nat_workaround: true,
            upload_sstv: true,
            sstv_dir: String::new(),
            upload_wefax: false,
            wefax_dir: String::new(),
            naming: UploadNaming::default(),
            keep_images: 30,
            add_extension: true,
            format: UploadFormat::default(),
            stamp: true,
            downscale: true,
            heartbeat: true,
        }
    }
}

impl PictureUploadConfig {
    /// Whether there is enough here to try a connection at all.
    pub fn is_usable(&self) -> bool {
        !self.host.trim().is_empty() && self.port != 0
    }
}

/// The outcome of one upload, or of a test asked for by
/// [`crate::Command::TestPictureUpload`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PictureUploadStatus {
    pub ok: bool,
    pub message: String,
    /// When it finished, in Unix seconds.
    pub unix: i64,
    /// The SHA-256 of the certificate the server presented, when the
    /// connection got that far — offered so a self-signed one can be pinned.
    pub fingerprint: Option<String>,
    /// A connection test rather than a picture.
    pub is_test: bool,
}

/// Reduce a fingerprint as typed or pasted to bare lowercase hex.
///
/// Browsers and `openssl x509 -fingerprint` print colon-separated upper case;
/// what is compared is the 64 hex digits alone.
pub fn normalize_fingerprint(s: &str) -> String {
    s.chars().filter(|c| c.is_ascii_hexdigit()).map(|c| c.to_ascii_lowercase()).collect()
}

/// A fingerprint as it is shown: upper-case byte pairs separated by colons.
pub fn format_fingerprint(hex: &str) -> String {
    let hex = normalize_fingerprint(hex).to_ascii_uppercase();
    hex.as_bytes()
        .chunks(2)
        .map(|c| std::str::from_utf8(c).unwrap_or_default())
        .collect::<Vec<_>>()
        .join(":")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprints_compare_however_they_were_written() {
        let a = "AB:cd:01 23";
        assert_eq!(normalize_fingerprint(a), "abcd0123");
        assert_eq!(format_fingerprint("abcd0123"), "AB:CD:01:23");
        assert_eq!(normalize_fingerprint(&format_fingerprint("abcd0123")), "abcd0123");
    }

    #[test]
    fn an_older_file_reads_with_defaults() {
        let cfg: PictureUploadConfig = serde_json::from_str(r#"{"enabled":true}"#).unwrap();
        assert!(cfg.enabled);
        assert_eq!(cfg.keep_images, 30);
        assert_eq!(cfg.security, FtpSecurity::ExplicitTls);
    }
}
