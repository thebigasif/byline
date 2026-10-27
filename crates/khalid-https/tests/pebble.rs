//! The whole flow against pebble, Let's Encrypt's own test server, when it
//! runs on this machine (decision 3 of the Phase 5 answers). The tests are
//! ignored otherwise. To run them:
//!
//! 1. Start `pebble-challtestsrv -defaultIPv4 127.0.0.1 -defaultIPv6 ""`,
//!    so every name resolves to this machine.
//! 2. Start `pebble -dnsserver 127.0.0.1:8053` with
//!    `PEBBLE_VA_NOSLEEP=1` and `PEBBLE_WFE_NONCEREJECT=0`. It listens on
//!    `localhost:14000` and makes its checks on ports 5001 (TLS-ALPN-01)
//!    and 5002 (HTTP-01).
//! 3. Save pebble's `test/certs/pebble.minica.pem` as
//!    `target/pebble/pebble.minica.pem`.
//! 4. Run `cargo test -p khalid-https --test pebble -- --ignored`.

use khalid_https::{
    Acme, AgreedToTerms, Certificate, Certificates, Check, Directory, Domain, TlsListener, port_80,
};
use khalid_outbound::{Limits, Outbound, Reach, Resolver};
use tokio::net::TcpListener;

/// Pebble's directory.
const DIRECTORY: &str = "https://localhost:14000/dir";

/// The certificate pebble's own address is signed with.
const ROOT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../target/pebble/pebble.minica.pem"
);

/// The domain to get a certificate for. The test DNS server answers it
/// with this machine.
const NAME: &str = "blog.khalid.test";

/// Signs up at pebble and gets a certificate for `NAME` with `check`,
/// answered through `certificates`.
#[cfg(test)]
async fn issue(certificates: &Certificates, check: Check) -> Certificate {
    let root = std::fs::read(ROOT).expect("pebble's root certificate is saved");
    let hosts = vec![("localhost".to_owned(), vec!["127.0.0.1".parse().unwrap()])];
    let resolver = Resolver::for_tests(hosts, Reach::PublicAndLoopback);
    let outbound = Outbound::trusting(Limits::default(), resolver, &root).unwrap();
    let directory = Directory::Other(DIRECTORY.to_owned());
    let (acme, _) = Acme::sign_up(&outbound, &directory, AgreedToTerms)
        .await
        .unwrap();
    let domain = Domain::try_new(NAME).unwrap();
    let issued = acme
        .issue(&domain, certificates.challenges(), &[check])
        .await
        .unwrap();
    Certificate::from_pem(issued.chain.as_bytes(), issued.key.as_bytes()).unwrap()
}

#[tokio::test]
#[ignore = "needs pebble on this machine; see this file's first lines"]
async fn pebble_issues_a_certificate_after_the_tls_check() {
    let certificates = Certificates::new();
    let tcp = TcpListener::bind("127.0.0.1:5001").await.unwrap();
    let listener = TlsListener::new(tcp, certificates.clone()).unwrap();
    tokio::spawn(async move { axum::serve(listener, axum::Router::new()).await });

    let certificate = issue(&certificates, Check::TlsAlpn).await;

    assert!(certificate.covers(NAME));
}

#[tokio::test]
#[ignore = "needs pebble on this machine; see this file's first lines"]
async fn pebble_issues_a_certificate_after_the_file_check() {
    let certificates = Certificates::new();
    let tcp = TcpListener::bind("127.0.0.1:5002").await.unwrap();
    let domain = Domain::try_new(NAME).unwrap();
    let router = port_80(&domain, certificates.challenges().clone());
    tokio::spawn(async move { axum::serve(tcp, router).await });

    let certificate = issue(&certificates, Check::Http).await;

    assert!(certificate.covers(NAME));
}
