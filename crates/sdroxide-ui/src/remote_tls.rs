//! `wss://` to a station whose certificate no public authority vouches for,
//! by trusting that one certificate and nothing else (issue #736).
//!
//! A station reached through an HTTPS reverse proxy on a private network has
//! no public name to get a real certificate for, so the proxy's is very often
//! self-signed — and the browser client needs that proxy, because a page
//! served over plain http may not open the microphone. The native client used
//! to refuse such a server outright (`invalid peer certificate: …`), which left
//! the operator choosing between the browser's sound and the native client.
//!
//! The answer is the one the Wavelog connection already gives
//! (`sdroxide-net`'s `tls_pin`): pin. A certificate the public roots accept is
//! accepted exactly as before, without asking. One they refuse has its SHA-256
//! remembered for the attempt; the radio's screen shows it, the operator
//! compares it with the server's and, if it is theirs, trusts it — and from
//! then on that exact certificate is accepted from that `host:port` and every
//! other one is refused. Name and expiry are not checked against a pin — a
//! self-signed certificate reached by IP address fails both, and the pin is a
//! stronger statement than either — but the handshake signature still is:
//! holding a copy of the certificate is not enough, the peer must hold its
//! private key.
//!
//! Switching verification off was never on the table: that accepts any
//! certificate from anyone who can answer at the address, which on a hotel
//! Wi-Fi is not only the operator's station.

use std::collections::BTreeMap;
use std::sync::{Arc, LazyLock, Mutex};

use rustls::client::WebPkiServerVerifier;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{CryptoProvider, WebPkiSupportedAlgorithms};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{ClientConfig, DigitallySignedStruct, RootCertStore, SignatureScheme};

use crate::remote::RefusedCert;

/// The pins, by `host:port`. Read from disk once and kept here, because every
/// tab of a station asks on every frame whether the certificate it was refused
/// for has since been trusted — by the operator, on another of its tabs.
static TRUSTED: LazyLock<Mutex<BTreeMap<String, String>>> =
    LazyLock::new(|| Mutex::new(sdroxide_config::load_trusted_certs()));

/// The public-roots check every certificate meets first. The same roots
/// tungstenite uses by itself, so a server it accepted is still accepted.
static WEBPKI: LazyLock<Result<Arc<WebPkiServerVerifier>, String>> = LazyLock::new(|| {
    let roots = RootCertStore { roots: webpki_roots::TLS_SERVER_ROOTS.to_vec() };
    WebPkiServerVerifier::builder_with_provider(Arc::new(roots), provider())
        .build()
        .map_err(|e| e.to_string())
});

/// `ring` only, as everything else here uses — rustls's default would pull in
/// aws-lc as a second crypto backend.
fn provider() -> Arc<CryptoProvider> {
    Arc::new(rustls::crypto::ring::default_provider())
}

/// SHA-256 of a DER certificate as 64 lowercase hex digits — the form the pin
/// is stored in, and what `openssl x509 -fingerprint -sha256` prints once its
/// colons are taken out.
fn sha256_hex(der: &[u8]) -> String {
    let digest = ring::digest::digest(&ring::digest::SHA256, der);
    digest.as_ref().iter().map(|b| format!("{b:02x}")).collect()
}

/// Which server a pin belongs to: the `host:port` of a `wss://` address, with
/// the port spelled out when the address leaves it to the scheme. `None` for
/// anything that is not `wss://`, which has no certificate to pin.
///
/// The several radios of one station, and its solar feed, are one server and
/// one certificate, so they come out the same; the path is not part of it.
pub(crate) fn authority(url: &str) -> Option<String> {
    let (scheme, rest) = url.trim().split_once("://")?;
    if !scheme.eq_ignore_ascii_case("wss") {
        return None;
    }
    let host_port = rest.split(['/', '?', '#']).next().unwrap_or_default();
    // Credentials in the address are not part of which server it is.
    let host_port = host_port.rsplit_once('@').map_or(host_port, |(_, h)| h).to_ascii_lowercase();
    if host_port.is_empty() {
        return None;
    }
    // An IPv6 literal is bracketed, and its colons are not a port.
    let has_port = match host_port.rfind(']') {
        Some(close) => host_port[close..].contains(':'),
        None => host_port.contains(':'),
    };
    Some(if has_port { host_port } else { format!("{host_port}:443") })
}

/// The pinned certificate for a server, if the operator has trusted one.
pub(crate) fn trusted(authority: &str) -> Option<String> {
    TRUSTED.lock().ok()?.get(authority).cloned()
}

/// Every pin, for the list in Settings → Remote.
pub(crate) fn all_trusted() -> Vec<(String, String)> {
    TRUSTED
        .lock()
        .map(|t| t.iter().map(|(a, p)| (a.clone(), p.clone())).collect())
        .unwrap_or_default()
}

/// Trust `sha256` from `authority` from now on, replacing whatever was trusted
/// there before. Takes effect for this run whether or not it could be saved.
pub(crate) fn trust(authority: &str, sha256: &str) -> Result<(), String> {
    let mut map = TRUSTED.lock().map_err(|_| "certificate store poisoned".to_string())?;
    map.insert(authority.to_string(), sha256.to_ascii_lowercase());
    sdroxide_config::save_trusted_certs(&map).map_err(|e| e.to_string())
}

/// Stop trusting whatever was pinned for `authority`: that server goes back to
/// being checked against the public authorities alone.
pub(crate) fn forget(authority: &str) -> Result<(), String> {
    let mut map = TRUSTED.lock().map_err(|_| "certificate store poisoned".to_string())?;
    if map.remove(authority).is_some() {
        sdroxide_config::save_trusted_certs(&map).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// The options to dial `url` with: for a `wss://` address, a TLS configuration
/// that also accepts the certificate pinned for it, and that leaves the
/// fingerprint of a certificate it refused in `refused`. Anything else dials
/// exactly as before.
pub(crate) fn options_for(url: &str, refused: &RefusedCert) -> ewebsock::Options {
    let mut options = ewebsock::Options::default();
    if let Some(authority) = authority(url) {
        match client_config(authority, refused.clone()) {
            Ok(config) => options.tls_config = Some(ewebsock::TlsConfig(Arc::new(config))),
            // Not expected — the roots are compiled in — and not worth failing
            // the dial over: tungstenite's own configuration still works for
            // every server that does not need a pin.
            Err(e) => tracing::warn!("remote TLS: {e}; dialling without pinned certificates"),
        }
    }
    options
}

fn client_config(authority: String, refused: RefusedCert) -> Result<ClientConfig, String> {
    let webpki = WEBPKI.as_ref().map_err(Clone::clone)?.clone();
    let provider = provider();
    let verifier = StationVerifier {
        authority,
        webpki,
        algorithms: provider.signature_verification_algorithms,
        refused,
    };
    Ok(ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|e| e.to_string())?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(verifier))
        .with_no_client_auth())
}

/// The certificate check: what the public roots accept, and otherwise the one
/// certificate pinned for this server.
#[derive(Debug)]
struct StationVerifier {
    authority: String,
    webpki: Arc<WebPkiServerVerifier>,
    algorithms: WebPkiSupportedAlgorithms,
    refused: RefusedCert,
}

impl ServerCertVerifier for StationVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        server_name: &ServerName<'_>,
        ocsp_response: &[u8],
        now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        let refusal = match self.webpki.verify_server_cert(
            end_entity,
            intermediates,
            server_name,
            ocsp_response,
            now,
        ) {
            Ok(ok) => return Ok(ok),
            Err(e) => e,
        };
        let presented = sha256_hex(end_entity.as_ref());
        if trusted(&self.authority).as_deref() == Some(presented.as_str()) {
            return Ok(ServerCertVerified::assertion());
        }
        if let Ok(mut slot) = self.refused.lock() {
            *slot = Some(presented);
        }
        Err(refusal)
    }

    // The signatures are checked for real, pinned or not: this is what makes a
    // copied certificate useless without its private key.
    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(message, cert, dss, &self.algorithms)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(message, cert, dss, &self.algorithms)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.algorithms.supported_schemes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pin_belongs_to_the_host_and_port_whatever_the_path() {
        assert_eq!(authority("wss://Shack.example/ws").as_deref(), Some("shack.example:443"));
        assert_eq!(
            authority("wss://shack.example:8443/sdr/ws/2").as_deref(),
            Some("shack.example:8443")
        );
        assert_eq!(authority("wss://192.168.1.10/solar-ws").as_deref(), Some("192.168.1.10:443"));
        assert_eq!(authority("wss://[fe80::1]/ws").as_deref(), Some("[fe80::1]:443"));
        assert_eq!(authority("wss://[fe80::1]:4443/ws").as_deref(), Some("[fe80::1]:4443"));
        assert_eq!(authority("wss://me@shack.example/ws").as_deref(), Some("shack.example:443"));
    }

    #[test]
    fn only_a_tls_address_has_a_certificate_to_pin() {
        assert_eq!(authority("ws://shack.example:4950/ws"), None);
        assert_eq!(authority("shack.example"), None);
        assert_eq!(authority("wss://"), None);
    }

    #[test]
    fn the_fingerprint_is_the_sha256_of_the_der() {
        // SHA-256 of the empty string, the standard test vector.
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn a_plain_address_dials_with_tungstenite_s_own_tls() {
        let refused = RefusedCert::default();
        assert!(options_for("ws://shack.example:4950/ws", &refused).tls_config.is_none());
        assert!(options_for("wss://shack.example/ws", &refused).tls_config.is_some());
    }
}

/// The dial against a real `wss://` server on loopback, behind the kind of
/// certificate issue #736 was about: self-signed, and marked as a CA, which
/// the public-roots check refuses as `CaUsedAsEndEntity` before it gets as far as
/// asking who signed it. What is under test is the whole path the remote
/// client takes — ewebsock's patched connect, this verifier, the pin store —
/// so the server is a few lines of rustls and tungstenite that say hello.
#[cfg(test)]
mod loopback_tests {
    use std::net::TcpListener;
    use std::time::{Duration, Instant};

    use ewebsock::{WsEvent, WsMessage};
    use rustls::pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer};

    use super::*;

    const CERT: &[u8] = include_bytes!("../tests/fixtures/remote-selfsigned.crt.der");
    const KEY: &[u8] = include_bytes!("../tests/fixtures/remote-selfsigned.key.der");

    /// Start a server; returns its `wss://` address. Each connection that
    /// completes the handshake is sent one "hello".
    fn server() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(KEY.to_vec()));
        let config = Arc::new(
            rustls::ServerConfig::builder_with_provider(provider())
                .with_safe_default_protocol_versions()
                .unwrap()
                .with_no_client_auth()
                .with_single_cert(vec![CertificateDer::from(CERT.to_vec())], key)
                .unwrap(),
        );
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let config = config.clone();
                std::thread::spawn(move || {
                    let Ok(conn) = rustls::ServerConnection::new(config) else { return };
                    // A refused handshake ends here.
                    let Ok(mut ws) = tungstenite::accept(rustls::StreamOwned::new(conn, stream))
                    else {
                        return;
                    };
                    let _ = ws.send(tungstenite::Message::Text("hello".into()));
                    while ws.read().is_ok() {}
                });
            }
        });
        format!("wss://127.0.0.1:{port}/ws")
    }

    /// Dial `url` and wait for what comes of it: the greeting, or the error
    /// the connection failed with.
    fn dial(url: &str, refused: &RefusedCert) -> Result<String, String> {
        let (_sender, receiver) = ewebsock::connect(url, options_for(url, refused)).unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            match receiver.try_recv() {
                Some(WsEvent::Message(WsMessage::Text(text))) => return Ok(text),
                Some(WsEvent::Error(e)) => return Err(e),
                Some(WsEvent::Closed) => return Err("closed".into()),
                Some(_) => {}
                None => std::thread::sleep(Duration::from_millis(5)),
            }
        }
        Err("timed out".into())
    }

    #[test]
    fn a_self_signed_station_is_refused_with_its_fingerprint_then_trusted() {
        let url = server();
        let here = authority(&url).unwrap();

        // Not trusted yet: refused as before, and the certificate is on hand
        // for the operator to compare.
        let refused = RefusedCert::default();
        let err = dial(&url, &refused).unwrap_err();
        assert!(err.contains("certificate"), "{err}");
        assert_eq!(refused.lock().unwrap().as_deref(), Some(sha256_hex(CERT).as_str()));

        // Trusted (in memory only — a test does not write the config).
        TRUSTED.lock().unwrap().insert(here.clone(), sha256_hex(CERT));
        let refused = RefusedCert::default();
        assert_eq!(dial(&url, &refused).as_deref(), Ok("hello"));
        assert_eq!(*refused.lock().unwrap(), None);

        // A pin for some other certificate is no pin for this one.
        TRUSTED.lock().unwrap().insert(here.clone(), "ab".repeat(32));
        let refused = RefusedCert::default();
        assert!(dial(&url, &refused).is_err());
        assert_eq!(refused.lock().unwrap().as_deref(), Some(sha256_hex(CERT).as_str()));

        TRUSTED.lock().unwrap().remove(&here);
    }
}
