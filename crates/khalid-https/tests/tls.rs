//! HTTPS end to end: a page served through `khalid_server::serve` on the
//! TLS listener, fetched by the outbound client trusting a test
//! certificate.

use std::time::Duration;

use axum::Router;
use axum::routing::get;
use khalid_core::AbsoluteUrl;
use khalid_https::{Certificate, Certificates, TlsListener};
use khalid_outbound::{Limits, Outbound, Reach, Resolver};
use rcgen::CertifiedKey;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;

/// The name the test certificate is for, resolved to loopback.
const NAME: &str = "blog.test";

/// A self-signed certificate for `NAME`, as PEM: the certificate, then its
/// key.
#[cfg(test)]
fn self_signed() -> (String, String) {
    let CertifiedKey { cert, signing_key } =
        rcgen::generate_simple_self_signed(vec![NAME.to_owned()]).unwrap();
    (cert.pem(), signing_key.serialize_pem())
}

/// A server answering `/` with "hello" over TLS, its port, and the switch
/// that stops it.
#[cfg(test)]
async fn serve(certificates: Certificates) -> (u16, oneshot::Sender<()>) {
    let tcp = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = tcp.local_addr().unwrap().port();
    let listener = TlsListener::new(tcp, certificates).unwrap();
    let app = Router::new().route("/", get(|| async { "hello" }));
    let (stop, stopped) = oneshot::channel::<()>();
    let shutdown = async move {
        let _ = stopped.await;
    };
    tokio::spawn(khalid_server::serve(
        listener,
        app,
        shutdown,
        Duration::from_secs(1),
    ));
    (port, stop)
}

/// The outbound client, trusting `root` alone and finding `NAME` on
/// loopback.
#[cfg(test)]
fn client(root: &str) -> Outbound {
    let hosts = vec![(NAME.to_owned(), vec!["127.0.0.1".parse().unwrap()])];
    let resolver = Resolver::for_tests(hosts, Reach::PublicAndLoopback);
    Outbound::trusting(Limits::default(), resolver, root.as_bytes()).unwrap()
}

#[cfg(test)]
fn home(port: u16) -> AbsoluteUrl {
    AbsoluteUrl::parse(&format!("https://{NAME}:{port}/")).unwrap()
}

#[tokio::test]
async fn a_page_is_answered_over_tls_with_the_certificate() {
    let (chain, key) = self_signed();
    let certificates = Certificates::new();
    certificates.set(Certificate::from_pem(chain.as_bytes(), key.as_bytes()).unwrap());
    let (port, stop) = serve(certificates).await;

    let fetched = client(&chain).get(&home(port)).await.unwrap();

    assert_eq!(fetched.status(), 200);
    assert_eq!(fetched.body(), b"hello");
    stop.send(()).unwrap();
}

#[tokio::test]
async fn a_new_certificate_takes_effect_without_a_restart() {
    let certificates = Certificates::new();
    let (port, stop) = serve(certificates.clone()).await;
    let (chain, key) = self_signed();
    let outbound = client(&chain);

    let before = outbound.get(&home(port)).await;
    certificates.set(Certificate::from_pem(chain.as_bytes(), key.as_bytes()).unwrap());
    let after = outbound.get(&home(port)).await.unwrap();

    assert!(before.is_err(), "no certificate yet: {before:?}");
    assert_eq!(after.body(), b"hello");
    stop.send(()).unwrap();
}

#[tokio::test]
async fn a_stalled_handshake_does_not_hold_up_the_next_visitor() {
    let (chain, key) = self_signed();
    let certificates = Certificates::new();
    certificates.set(Certificate::from_pem(chain.as_bytes(), key.as_bytes()).unwrap());
    let (port, stop) = serve(certificates).await;
    let mut stalled = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    stalled.write_all(&[0x16, 0x03, 0x01]).await.unwrap();

    let fetched = tokio::time::timeout(Duration::from_secs(5), client(&chain).get(&home(port)))
        .await
        .expect("the second visitor is answered while the first stalls")
        .unwrap();

    assert_eq!(fetched.body(), b"hello");
    drop(stalled);
    stop.send(()).unwrap();
}
