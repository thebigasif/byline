//! Helpers the HTTPS tests share: a TLS client that reads whatever
//! certificate a server answers with.

use std::sync::Arc;
use std::time::Duration;

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::ring::default_provider;
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{ClientConfig, DigitallySignedStruct, SignatureScheme};
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;

/// Accepts any server certificate: the test reads what it was given.
#[derive(Debug)]
struct AnyCertificate;

impl ServerCertVerifier for AnyCertificate {
    fn verify_server_cert(
        &self,
        _: &CertificateDer<'_>,
        _: &[CertificateDer<'_>],
        _: &ServerName<'_>,
        _: &[u8],
        _: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _: &[u8],
        _: &CertificateDer<'_>,
        _: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _: &[u8],
        _: &CertificateDer<'_>,
        _: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}

/// Connects to loopback `port` for `name`, offering `protocol`, and
/// returns the certificate the server answered with and whether it then
/// closed the connection, or nothing when the handshake failed.
#[cfg(test)]
pub(crate) async fn tls_handshake(
    port: u16,
    name: &str,
    protocol: &[u8],
) -> Option<(Vec<u8>, bool)> {
    let mut config = ClientConfig::builder_with_provider(Arc::new(default_provider()))
        .with_safe_default_protocol_versions()
        .unwrap()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(AnyCertificate))
        .with_no_client_auth();
    config.alpn_protocols = vec![protocol.to_vec()];
    let tcp = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    let name = ServerName::try_from(name.to_owned()).unwrap();
    let mut tls = TlsConnector::from(Arc::new(config))
        .connect(name, tcp)
        .await
        .ok()?;
    let certificate = tls.get_ref().1.peer_certificates()?.first()?.to_vec();
    let mut byte = [0; 1];
    let read = tokio::time::timeout(Duration::from_millis(300), tls.read(&mut byte)).await;
    let closed = matches!(read, Ok(Ok(0) | Err(_)));
    Some((certificate, closed))
}
