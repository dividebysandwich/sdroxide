//! The TLS half of FTPS: how a server's certificate is judged.
//!
//! Three answers, one per [`TlsVerify`]:
//!
//! - **System** hands the chain to the operating system's own verifier
//!   (Security.framework, CryptoAPI, or the distribution's CA bundle), so a
//!   certificate is trusted exactly when a browser on this machine would trust
//!   it — including a company CA the operator has installed — plus any PEM the
//!   operator pasted.
//! - **Pinned** trusts one certificate by the SHA-256 of its DER encoding, and
//!   nothing else. Expiry and name are deliberately not looked at: the point
//!   is a self-signed home server, and the pin already says which key it is.
//! - **Off** trusts anything. The link is still encrypted, but an attacker in
//!   the path can present a certificate of their own and read the password.
//!
//! Pinned and Off still check the handshake signature against the presented
//! certificate. Without that, a pin would only prove somebody *sent* the
//! certificate, not that they hold its key.
//!
//! Every verifier records the fingerprint it was shown, so a test can offer
//! "trust this certificate" even when verification failed.

use std::sync::{Arc, Mutex};

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{CryptoProvider, verify_tls12_signature, verify_tls13_signature};
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{ClientConfig, DigitallySignedStruct, Error, SignatureScheme};
use sdroxide_types::{PictureUploadConfig, TlsVerify, normalize_fingerprint};
use sha2::{Digest, Sha256};

/// The fingerprint the server presented, once it has presented one.
pub type Seen = Arc<Mutex<Option<String>>>;

/// The SHA-256 of a certificate's DER encoding, in lowercase hex.
pub fn fingerprint(cert: &[u8]) -> String {
    Sha256::digest(cert).iter().map(|b| format!("{b:02x}")).collect()
}

fn provider() -> Arc<CryptoProvider> {
    Arc::new(rustls::crypto::ring::default_provider())
}

/// A client configuration for these settings, and where the presented
/// certificate's fingerprint will be recorded.
///
/// One configuration serves the control connection and every data
/// connection after it. That is not only economy: it holds the session cache,
/// and servers set up to require TLS session reuse (vsftpd's
/// `require_ssl_reuse`, FileZilla Server) refuse a data connection that does
/// not resume the control connection's session.
pub fn client_config(cfg: &PictureUploadConfig) -> Result<(Arc<ClientConfig>, Seen), String> {
    let seen: Seen = Arc::default();
    let inner: Arc<dyn ServerCertVerifier> = match cfg.verify {
        TlsVerify::System => Arc::new(system_verifier(&cfg.extra_ca_pem)?),
        TlsVerify::Pinned => {
            let pin = normalize_fingerprint(&cfg.pinned_sha256);
            if pin.len() != 64 {
                return Err("the pinned fingerprint must be 64 hex digits (SHA-256)".into());
            }
            Arc::new(Lenient { pin: Some(pin), provider: provider() })
        }
        TlsVerify::Off => Arc::new(Lenient { pin: None, provider: provider() }),
    };
    let verifier = Arc::new(Recording { inner, seen: seen.clone() });
    let config = ClientConfig::builder_with_provider(provider())
        .with_safe_default_protocol_versions()
        .map_err(|e| e.to_string())?
        .dangerous()
        .with_custom_certificate_verifier(verifier)
        .with_no_client_auth();
    Ok((Arc::new(config), seen))
}

fn system_verifier(extra_pem: &str) -> Result<rustls_platform_verifier::Verifier, String> {
    let extra = parse_pem(extra_pem)?;
    if extra.is_empty() {
        rustls_platform_verifier::Verifier::new(provider())
    } else {
        rustls_platform_verifier::Verifier::new_with_extra_roots(extra, provider())
    }
    .map_err(|e| format!("system certificate store: {e}"))
}

/// Every certificate in a pasted PEM block. Empty text is no certificates;
/// text that holds none, or a broken one, is an error worth showing.
pub fn parse_pem(text: &str) -> Result<Vec<CertificateDer<'static>>, String> {
    if text.trim().is_empty() {
        return Ok(Vec::new());
    }
    let certs = CertificateDer::pem_slice_iter(text.as_bytes())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("extra CA certificate: {e}"))?;
    if certs.is_empty() {
        return Err("extra CA certificate: no BEGIN CERTIFICATE block found".into());
    }
    Ok(certs)
}

/// Constant-time comparison, so a wrong pin does not say how much of it was
/// right. (A fingerprint is not a secret, but there is no reason to be sloppy.)
fn same(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// Pinned and Off: no chain, no names, no dates — one key, or any key.
#[derive(Debug)]
struct Lenient {
    pin: Option<String>,
    provider: Arc<CryptoProvider>,
}

impl ServerCertVerifier for Lenient {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, Error> {
        match &self.pin {
            Some(pin) if !same(&fingerprint(end_entity), pin) => Err(Error::General(
                "the server's certificate does not match the pinned fingerprint".into(),
            )),
            Some(_) => Ok(ServerCertVerified::assertion()),
            None => {
                tracing::warn!("picture upload: accepting the server certificate unverified");
                Ok(ServerCertVerified::assertion())
            }
        }
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        verify_tls12_signature(message, cert, dss, &self.provider.signature_verification_algorithms)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        verify_tls13_signature(message, cert, dss, &self.provider.signature_verification_algorithms)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.provider.signature_verification_algorithms.supported_schemes()
    }
}

/// Notes the presented certificate's fingerprint, then defers to `inner`.
#[derive(Debug)]
struct Recording {
    inner: Arc<dyn ServerCertVerifier>,
    seen: Seen,
}

impl ServerCertVerifier for Recording {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        server_name: &ServerName<'_>,
        ocsp_response: &[u8],
        now: UnixTime,
    ) -> Result<ServerCertVerified, Error> {
        if let Ok(mut seen) = self.seen.lock() {
            *seen = Some(fingerprint(end_entity));
        }
        self.inner.verify_server_cert(end_entity, intermediates, server_name, ocsp_response, now)
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        self.inner.verify_tls12_signature(message, cert, dss)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        self.inner.verify_tls13_signature(message, cert, dss)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.inner.supported_verify_schemes()
    }

    fn requires_raw_public_keys(&self) -> bool {
        self.inner.requires_raw_public_keys()
    }

    fn root_hint_subjects(&self) -> Option<&[rustls::DistinguishedName]> {
        self.inner.root_hint_subjects()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cert() -> CertificateDer<'static> {
        CertificateDer::from(b"not really a certificate".to_vec())
    }

    fn check(v: &Lenient) -> Result<ServerCertVerified, Error> {
        let name = ServerName::try_from("ftp.example.org").unwrap();
        v.verify_server_cert(&cert(), &[], &name, &[], UnixTime::now())
    }

    #[test]
    fn a_pin_accepts_its_own_certificate_and_nothing_else() {
        let good = Lenient { pin: Some(fingerprint(&cert())), provider: provider() };
        assert!(check(&good).is_ok());
        let bad = Lenient { pin: Some("00".repeat(32)), provider: provider() };
        assert!(check(&bad).is_err());
    }

    #[test]
    fn off_accepts_anything() {
        assert!(check(&Lenient { pin: None, provider: provider() }).is_ok());
    }

    #[test]
    fn a_short_pin_is_refused_before_connecting() {
        let cfg = PictureUploadConfig {
            verify: TlsVerify::Pinned,
            pinned_sha256: "AB:CD".into(),
            ..Default::default()
        };
        assert!(client_config(&cfg).is_err());
    }

    #[test]
    fn fingerprints_are_sha256_hex() {
        // SHA-256 of the empty string.
        assert_eq!(
            fingerprint(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn pasted_pem_without_a_certificate_is_an_error() {
        assert!(parse_pem("").unwrap().is_empty());
        assert!(parse_pem("hello").is_err());
    }
}
