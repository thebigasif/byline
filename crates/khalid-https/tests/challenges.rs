//! The answer to Let's Encrypt's TLS check (TLS-ALPN-01): a special
//! certificate in the handshake, while every other visitor gets the site's
//! own certificate.

mod common;

use axum::serve::Listener;
use common::tls_handshake;
use khalid_https::{Certificate, Certificates, TlsListener};
use tokio::net::TcpListener;

/// The name the checks are for.
const NAME: &str = "blog.test";

/// A digest as a check would carry it.
const DIGEST: [u8; 32] = [7; 32];

/// The site's own certificate, set in a new `Certificates`, and its DER.
#[cfg(test)]
fn site() -> (Certificates, Vec<u8>) {
    let made = rcgen::generate_simple_self_signed(vec![NAME.to_owned()]).unwrap();
    let certificates = Certificates::new();
    let pem = (made.cert.pem(), made.signing_key.serialize_pem());
    certificates.set(Certificate::from_pem(pem.0.as_bytes(), pem.1.as_bytes()).unwrap());
    (certificates, made.cert.der().to_vec())
}

/// A TLS listener answering with `certificates`, accepting in the
/// background and closing what it accepts, and its port.
#[cfg(test)]
async fn listen(certificates: Certificates) -> u16 {
    let tcp = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = tcp.local_addr().unwrap().port();
    let mut listener = TlsListener::new(tcp, certificates).unwrap();
    tokio::spawn(async move {
        loop {
            drop(listener.accept().await);
        }
    });
    port
}

#[tokio::test]
async fn a_tls_check_gets_the_certificate_carrying_its_digest_then_is_closed() {
    let (certificates, _) = site();
    certificates.challenges().answer_tls(NAME, &DIGEST).unwrap();
    let port = listen(certificates).await;

    let (certificate, closed) = tls_handshake(port, NAME, b"acme-tls/1").await.unwrap();

    let mut extension = vec![0x04, 0x20];
    extension.extend_from_slice(&DIGEST);
    assert!(
        certificate.windows(extension.len()).any(|w| w == extension),
        "the acmeIdentifier extension carries the digest"
    );
    assert!(closed, "a check's connection never reaches the site");
}

#[tokio::test]
async fn a_visitor_gets_the_site_certificate_while_a_check_is_open() {
    let (certificates, site_der) = site();
    certificates.challenges().answer_tls(NAME, &DIGEST).unwrap();
    let port = listen(certificates).await;

    let (certificate, _) = tls_handshake(port, NAME, b"http/1.1").await.unwrap();

    assert_eq!(certificate, site_der);
}

#[tokio::test]
async fn a_tls_check_for_a_closed_order_gets_no_certificate() {
    let (certificates, _) = site();
    certificates.challenges().answer_tls(NAME, &DIGEST).unwrap();
    certificates.challenges().clear();
    let port = listen(certificates).await;

    let answered = tls_handshake(port, NAME, b"acme-tls/1").await;

    assert!(answered.is_none(), "the handshake fails");
}
