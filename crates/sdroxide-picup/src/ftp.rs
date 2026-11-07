//! One FTP session: connect, secure, log in, and the handful of operations an
//! upload needs.
//!
//! Every session is opened for one job and closed after it, as QSSTV does. A
//! picture arrives every couple of minutes at most, and a connection held open
//! in between would mostly be one the server had already timed out.

use std::io::Cursor;
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;

use sdroxide_types::{FtpSecurity, PictureUploadConfig};
use suppaftp::types::FileType;
use suppaftp::{FtpError, Mode, RustlsConnector, RustlsFtpStream};

use crate::tls::{self, Seen};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const IO_TIMEOUT: Duration = Duration::from_secs(30);

/// An open, logged-in session.
pub struct Session {
    ftp: RustlsFtpStream,
}

/// Why a session could not be opened, and the certificate fingerprint the
/// server presented if it got that far.
pub struct Failure {
    pub message: String,
    pub fingerprint: Option<String>,
}

fn seen(s: &Option<Seen>) -> Option<String> {
    s.as_ref().and_then(|s| s.lock().ok().and_then(|g| g.clone()))
}

fn resolve(host: &str, port: u16) -> Result<SocketAddr, String> {
    (host, port)
        .to_socket_addrs()
        .map_err(|e| format!("cannot resolve {host}: {e}"))?
        .next()
        .ok_or_else(|| format!("cannot resolve {host}"))
}

/// A data connection with the same timeouts as the control one, so a server
/// that stops answering mid-transfer cannot hold the worker forever.
fn data_stream(addr: SocketAddr) -> Result<TcpStream, FtpError> {
    let s =
        TcpStream::connect_timeout(&addr, CONNECT_TIMEOUT).map_err(FtpError::ConnectionError)?;
    let _ = s.set_read_timeout(Some(IO_TIMEOUT));
    let _ = s.set_write_timeout(Some(IO_TIMEOUT));
    Ok(s)
}

impl Session {
    /// Connect, secure, log in and change to `dir` (empty: stay put).
    ///
    /// Returns the fingerprint of the server's certificate alongside the
    /// session, for a test to show.
    pub fn open(
        cfg: &PictureUploadConfig,
        dir: &str,
    ) -> Result<(Session, Option<String>), Failure> {
        let mut recorded = None;
        let fail = |step: &str, e: String, recorded: &Option<Seen>| Failure {
            message: format!("{step}: {e}"),
            fingerprint: seen(recorded),
        };
        let host = cfg.host.trim();
        let addr = resolve(host, cfg.port).map_err(|e| fail("connect", e, &recorded))?;

        let ftp = match cfg.security {
            FtpSecurity::Plain => RustlsFtpStream::connect_timeout(addr, CONNECT_TIMEOUT)
                .map_err(|e| fail("connect", said(e), &recorded))?,
            FtpSecurity::ExplicitTls => {
                let (config, s) = tls::client_config(cfg).map_err(|e| fail("TLS", e, &recorded))?;
                recorded = Some(s);
                let ftp = RustlsFtpStream::connect_timeout(addr, CONNECT_TIMEOUT)
                    .map_err(|e| fail("connect", said(e), &recorded))?;
                set_timeouts(&ftp);
                ftp.into_secure(RustlsConnector::from(config), host)
                    .map_err(|e| fail("TLS", said(e), &recorded))?
            }
            FtpSecurity::ImplicitTls => {
                let (config, s) = tls::client_config(cfg).map_err(|e| fail("TLS", e, &recorded))?;
                recorded = Some(s);
                // suppaftp's implicit connect has no timeout of its own, so
                // find out first whether anything answers at all — otherwise
                // a dead host holds the worker for the OS's minute or more.
                TcpStream::connect_timeout(&addr, CONNECT_TIMEOUT)
                    .map_err(|e| fail("connect", e.to_string(), &recorded))?;
                RustlsFtpStream::connect_secure_implicit(addr, RustlsConnector::from(config), host)
                    .map_err(|e| fail("TLS", said(e), &recorded))?
            }
        };
        set_timeouts(&ftp);
        let mut ftp = ftp.passive_stream_builder(data_stream);
        ftp.set_mode(Mode::Passive);
        ftp.set_passive_nat_workaround(cfg.passive_nat_workaround);

        let user = if cfg.user.trim().is_empty() { "anonymous" } else { cfg.user.trim() };
        ftp.login(user, cfg.password.as_str()).map_err(|e| fail("login", said(e), &recorded))?;
        ftp.transfer_type(FileType::Binary).map_err(|e| fail("binary mode", said(e), &recorded))?;
        let dir = dir.trim();
        if !dir.is_empty() {
            ftp.cwd(dir).map_err(|e| fail(&format!("directory {dir}"), said(e), &recorded))?;
        }
        let fp = seen(&recorded);
        Ok((Session { ftp }, fp))
    }

    pub fn pwd(&mut self) -> Result<String, String> {
        self.ftp.pwd().map_err(|e| format!("current directory: {}", said(e)))
    }

    pub fn cwd(&mut self, dir: &str) -> Result<(), String> {
        self.ftp.cwd(dir).map_err(|e| format!("directory {dir}: {}", said(e)))
    }

    pub fn put(&mut self, name: &str, bytes: &[u8]) -> Result<(), String> {
        self.ftp
            .put_file(name, &mut Cursor::new(bytes))
            .map(|_| ())
            .map_err(|e| format!("upload {name}: {}", said(e)))
    }

    pub fn delete(&mut self, name: &str) -> Result<(), String> {
        self.ftp.rm(name).map_err(|e| format!("delete {name}: {}", said(e)))
    }

    pub fn rename(&mut self, from: &str, to: &str) -> Result<(), String> {
        self.ftp.rename(from, to).map_err(|e| format!("rename {from} → {to}: {}", said(e)))
    }

    /// The bare names in the current directory.
    pub fn names(&mut self) -> Result<Vec<String>, String> {
        match self.ftp.nlst(None) {
            Ok(v) => Ok(v),
            // An empty directory is a 550 on some servers rather than an empty
            // list — nothing to rotate either way.
            Err(FtpError::UnexpectedResponse(r)) if r.status.code() == 550 => Ok(Vec::new()),
            Err(e) => Err(format!("list directory: {}", said(e))),
        }
    }

    pub fn quit(mut self) {
        let _ = self.ftp.quit();
    }
}

/// An error as the operator should read it. suppaftp and rustls word theirs
/// for a programmer — `Invalid response: [530] 530 Login incorrect.`,
/// `invalid peer certificate: Other(OtherError("…"))` — and the part worth
/// showing is the server's sentence, or the operating system's.
fn said(e: FtpError) -> String {
    tidy(&e.to_string())
}

fn tidy(s: &str) -> String {
    let s = s.strip_prefix("Connection error: ").unwrap_or(s);
    let s = s.strip_prefix("Secure error: ").unwrap_or(s);
    let s = s.replace("unexpected error: ", "");
    if let Some(rest) = s.strip_prefix("Invalid response: [")
        && let Some((_, text)) = rest.split_once("] ")
    {
        return text.trim().to_string();
    }
    if let Some((_, rest)) = s.split_once("invalid peer certificate: ") {
        // The platform verifier's own sentence, when it gave one.
        let inner = match (rest.find('"'), rest.rfind('"')) {
            (Some(a), Some(b)) if b > a => &rest[a + 1..b],
            _ => rest,
        };
        return format!("certificate not trusted — {inner}");
    }
    s
}

fn set_timeouts(ftp: &RustlsFtpStream) {
    let s = ftp.get_ref();
    let _ = s.set_read_timeout(Some(IO_TIMEOUT));
    let _ = s.set_write_timeout(Some(IO_TIMEOUT));
}

#[cfg(test)]
mod tests {
    use super::tidy;

    #[test]
    fn errors_read_as_the_server_or_the_system_said_them() {
        assert_eq!(
            tidy("Invalid response: [530] 530 Authentication failed."),
            "530 Authentication failed."
        );
        assert_eq!(
            tidy(
                "Connection error: invalid peer certificate: Other(OtherError(\"“localhost” certificate is not trusted: -67843\"))"
            ),
            "certificate not trusted — “localhost” certificate is not trusted: -67843"
        );
        assert_eq!(
            tidy(
                "Connection error: unexpected error: the server's certificate does not match the pinned fingerprint"
            ),
            "the server's certificate does not match the pinned fingerprint"
        );
        assert_eq!(
            tidy("Connection error: Connection refused (os error 61)"),
            "Connection refused (os error 61)"
        );
    }
}
