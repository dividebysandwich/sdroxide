//! HTTPS to a server that signed its own certificate, by trusting that one
//! certificate and nothing else.
//!
//! A home Wavelog is very often behind a certificate nobody vouches for — a
//! reverse proxy's self-signed default, a LAN-only box with no public name to
//! get a real one for. The two easy answers are both wrong: refusing it locks
//! the operator out of their own server, and switching verification off
//! accepts *any* certificate from *anyone* who can answer at that address,
//! which on a hotel Wi-Fi is not only the operator's server.
//!
//! So this pins. The operator is shown the SHA-256 of the certificate the
//! server presented, compares it with their server's, and if it is theirs it
//! is stored; from then on that exact certificate is accepted and every other
//! one is refused. Name and expiry are not checked against it — a self-signed
//! certificate reached by IP address fails both, and the pin is a stronger
//! statement than either — but the handshake signature still is: holding a
//! copy of the certificate is not enough, the peer must hold its private key.
//!
//! ureq has no hook for a custom verifier, so this is a connector of our own
//! slotted into its transport chain where its rustls one would sit, built on
//! ureq's `unversioned` transport API. That API may change between ureq minor
//! releases; if an update stops this compiling, this file is the whole of it.

use std::fmt;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::WebPkiSupportedAlgorithms;
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{
    CertificateError, ClientConfig, ClientConnection, DigitallySignedStruct, SignatureScheme,
    StreamOwned,
};
use ureq::unversioned::resolver::DefaultResolver;
use ureq::unversioned::transport::{
    Buffers, ConnectProxyConnector, ConnectionDetails, Connector, Either, LazyBuffers, NextTimeout,
    TcpConnector, Transport, TransportAdapter,
};

/// SHA-256 of a DER certificate as 64 lowercase hex digits.
pub fn sha256_hex(der: &[u8]) -> String {
    let digest = ring::digest::digest(&ring::digest::SHA256, der);
    digest.as_ref().iter().map(|b| format!("{b:02x}")).collect()
}

/// Reduce whatever the operator pasted — `AB:CD:…`, spaced, mixed case — to
/// the 64 lowercase hex digits [`sha256_hex`] produces, or `None` if it is not
/// a SHA-256 at all.
pub fn normalize_pin(pin: &str) -> Option<String> {
    let hex: String =
        pin.chars().filter(|c| c.is_ascii_hexdigit()).map(|c| c.to_ascii_lowercase()).collect();
    let separators_only = pin.chars().all(|c| c.is_ascii_hexdigit() || ":- ".contains(c));
    (hex.len() == 64 && separators_only).then_some(hex)
}

/// The certificate check: accept the pinned certificate, refuse every other,
/// and remember what was presented either way so it can be shown.
#[derive(Debug)]
struct PinVerifier {
    /// `None` to refuse everything — a probe that only wants to see the
    /// certificate and must not go on to talk to the server.
    pin: Option<String>,
    seen: Arc<Mutex<Option<String>>>,
    algorithms: WebPkiSupportedAlgorithms,
}

impl ServerCertVerifier for PinVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        let presented = sha256_hex(end_entity.as_ref());
        if let Ok(mut seen) = self.seen.lock() {
            *seen = Some(presented.clone());
        }
        if self.pin.as_deref() == Some(presented.as_str()) {
            Ok(ServerCertVerified::assertion())
        } else {
            Err(rustls::Error::InvalidCertificate(CertificateError::ApplicationVerificationFailure))
        }
    }

    // The signatures are checked for real: this is what makes a copied
    // certificate useless without its private key.
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

/// ureq connector that wraps the TCP stream in TLS checked by a [`PinVerifier`].
struct PinnedTlsConnector {
    config: Arc<ClientConfig>,
}

impl fmt::Debug for PinnedTlsConnector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PinnedTlsConnector").finish()
    }
}

impl<In: Transport> Connector<In> for PinnedTlsConnector {
    type Out = Either<In, PinnedTlsTransport>;

    fn connect(
        &self,
        details: &ConnectionDetails,
        chained: Option<In>,
    ) -> Result<Option<Self::Out>, ureq::Error> {
        let Some(transport) = chained else { return Ok(None) };
        if !details.needs_tls() || transport.is_tls() {
            return Ok(Some(Either::A(transport)));
        }
        // An IPv6 literal arrives bracketed, as it is written in a URL.
        let host = details.uri.host().unwrap_or_default();
        let host = host.trim_start_matches('[').trim_end_matches(']').to_string();
        let name = ServerName::try_from(host).map_err(|_| ureq::Error::Tls("invalid host name"))?;
        let conn = ClientConnection::new(self.config.clone(), name)?;
        let stream = StreamOwned { conn, sock: TransportAdapter::new(transport.boxed()) };
        let buffers = LazyBuffers::new(
            details.config.input_buffer_size(),
            details.config.output_buffer_size(),
        );
        Ok(Some(Either::B(PinnedTlsTransport { buffers, stream })))
    }
}

/// One TLS connection, read and written through ureq's buffers. The same shape
/// as ureq's own rustls transport.
struct PinnedTlsTransport {
    buffers: LazyBuffers,
    stream: StreamOwned<ClientConnection, TransportAdapter>,
}

impl fmt::Debug for PinnedTlsTransport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PinnedTlsTransport").finish()
    }
}

impl Transport for PinnedTlsTransport {
    fn buffers(&mut self) -> &mut dyn Buffers {
        &mut self.buffers
    }

    fn transmit_output(&mut self, amount: usize, timeout: NextTimeout) -> Result<(), ureq::Error> {
        self.stream.get_mut().set_timeout(timeout);
        let output = &self.buffers.output()[..amount];
        self.stream.write_all(output)?;
        Ok(())
    }

    fn await_input(&mut self, timeout: NextTimeout) -> Result<bool, ureq::Error> {
        self.stream.get_mut().set_timeout(timeout);
        let input = self.buffers.input_append_buf();
        let amount = self.stream.read(input)?;
        self.buffers.input_appended(amount);
        Ok(amount > 0)
    }

    fn is_open(&mut self) -> bool {
        self.stream.get_mut().get_mut().is_open()
    }

    fn is_tls(&self) -> bool {
        true
    }
}

/// An agent whose HTTPS accepts only the certificate whose SHA-256 is `pin`,
/// or — with `pin` `None` — no certificate at all. Either way the fingerprint
/// of the certificate presented is left in the returned slot, which is how a
/// connection check learns what it is asking the operator to trust.
///
/// Plain `http://` URLs go through it unchanged.
pub fn pinned_agent(
    config: ureq::config::Config,
    pin: Option<String>,
) -> (ureq::Agent, Arc<Mutex<Option<String>>>) {
    let seen = Arc::new(Mutex::new(None));
    let provider = rustls::crypto::ring::default_provider();
    let verifier = PinVerifier {
        pin,
        seen: seen.clone(),
        algorithms: provider.signature_verification_algorithms,
    };
    let tls = ClientConfig::builder_with_provider(Arc::new(provider))
        .with_safe_default_protocol_versions()
        .expect("ring supports the default protocol versions")
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(verifier))
        .with_no_client_auth();
    let connector =
        ().chain(ConnectProxyConnector::default())
            .chain(TcpConnector::default())
            .chain(PinnedTlsConnector { config: Arc::new(tls) });
    (ureq::Agent::with_parts(config, connector, DefaultResolver::default()), seen)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pasted_fingerprint_is_read_however_it_is_punctuated() {
        let plain = "ab".repeat(32);
        assert_eq!(normalize_pin(&plain).as_deref(), Some(plain.as_str()));
        let colons = vec!["AB"; 32].join(":");
        assert_eq!(normalize_pin(&colons).as_deref(), Some(plain.as_str()));
        assert_eq!(normalize_pin(&format!(" {colons} ")).as_deref(), Some(plain.as_str()));
    }

    #[test]
    fn something_that_is_not_a_sha256_is_not_a_pin() {
        assert_eq!(normalize_pin(""), None);
        assert_eq!(normalize_pin(&"ab".repeat(20)), None, "a SHA-1 is too short");
        assert_eq!(normalize_pin(&format!("{}zz", "ab".repeat(31))), None);
    }

    #[test]
    fn the_fingerprint_is_the_sha256_of_the_der() {
        // SHA-256 of the empty string, the standard test vector.
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
