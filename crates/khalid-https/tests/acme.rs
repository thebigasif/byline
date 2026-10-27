//! An account with a certificate authority over ACME, and a certificate
//! got with it, against a small test service that checks the answers for
//! real: the TLS-ALPN-01 certificate on the TLS listener, or the HTTP-01
//! file on the port 80 router.

#[cfg(test)]
mod acme_service;
#[cfg(test)]
mod common;

use std::time::Duration;

use acme_service::{AcmeService, Offer, start_ordering};
use khalid_https::{
    AccountKey, Acme, AgreedToTerms, Certificate, Certificates, Check, Directory, Domain,
    HttpsError, TlsListener, port_80,
};
use khalid_outbound::{Limits, Outbound, Reach, Resolver};
use tokio::net::TcpListener;

/// The domain the certificates are for.
const NAME: &str = "blog.test";

/// The TLS listener and the port 80 router answering with `certificates`,
/// and their ports.
#[cfg(test)]
async fn serve(certificates: &Certificates) -> (u16, u16) {
    let tls = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let plain = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let ports = (
        tls.local_addr().unwrap().port(),
        plain.local_addr().unwrap().port(),
    );
    let listener = TlsListener::new(tls, certificates.clone()).unwrap();
    tokio::spawn(async move { axum::serve(listener, axum::Router::new()).await });
    let domain = Domain::try_new(NAME).unwrap();
    let router = port_80(&domain, certificates.challenges().clone());
    tokio::spawn(async move { axum::serve(plain, router).await });
    ports
}

/// Signs up at `service`, then gets a certificate for `NAME` answering
/// through `certificates` with `checks`.
#[cfg(test)]
async fn issue(
    service: &AcmeService,
    certificates: &Certificates,
    checks: &[Check],
) -> Result<Certificate, HttpsError> {
    let directory = Directory::Other(service.directory.clone());
    let (acme, _) = Acme::sign_up(&outbound(), &directory, AgreedToTerms).await?;
    let domain = Domain::try_new(NAME).unwrap();
    let ordering = acme.issue(&domain, certificates.challenges(), checks);
    let issued = tokio::time::timeout(Duration::from_secs(30), ordering)
        .await
        .expect("the order ends")?;
    Certificate::from_pem(issued.chain.as_bytes(), issued.key.as_bytes())
}

#[tokio::test]
async fn a_certificate_is_issued_after_the_tls_check() {
    let certificates = Certificates::new();
    let service = start_ordering(NAME, Offer::TlsAlpn, serve(&certificates).await).await;

    let certificate = issue(&service, &certificates, &[Check::TlsAlpn, Check::Http])
        .await
        .unwrap();

    assert!(certificate.covers(NAME));
    assert_eq!(service.issued(), 1);
}

#[tokio::test]
async fn a_certificate_is_issued_after_the_file_check_whose_answer_is_then_cleared() {
    let certificates = Certificates::new();
    let service = start_ordering(NAME, Offer::Http, serve(&certificates).await).await;

    let certificate = issue(&service, &certificates, &[Check::TlsAlpn, Check::Http])
        .await
        .unwrap();

    assert!(certificate.covers(NAME));
    assert_eq!(certificates.challenges().http_answer("token-1"), None);
}

#[tokio::test]
async fn an_order_offering_no_check_this_server_answers_is_refused() {
    let certificates = Certificates::new();
    let service = start_ordering(NAME, Offer::Http, serve(&certificates).await).await;

    let error = issue(&service, &certificates, &[Check::TlsAlpn])
        .await
        .unwrap_err();

    assert!(matches!(error, HttpsError::NoCheck), "{error:?}");
}

#[tokio::test]
async fn a_wrong_answer_to_either_check_is_refused_and_nothing_is_issued() {
    for (offer, check) in [(Offer::TlsAlpn, Check::TlsAlpn), (Offer::Http, Check::Http)] {
        let served = Certificates::new();
        let service = start_ordering(NAME, offer, serve(&served).await).await;
        let elsewhere = Certificates::new();

        let error = issue(&service, &elsewhere, &[check]).await.unwrap_err();

        assert!(
            matches!(error, HttpsError::Refused { .. }),
            "{offer:?}: {error:?}"
        );
        assert_eq!(service.issued(), 0);
    }
}

/// The outbound client, reaching the test service on loopback.
#[cfg(test)]
fn outbound() -> Outbound {
    let resolver = Resolver::for_tests(Vec::new(), Reach::PublicAndLoopback);
    Outbound::with_resolver(Limits::default(), resolver).unwrap()
}

#[tokio::test]
async fn a_new_account_agrees_to_the_terms_and_gets_its_address() {
    let service = acme_service::start().await;
    let directory = Directory::Other(service.directory.clone());

    let (acme, key) = Acme::sign_up(&outbound(), &directory, AgreedToTerms)
        .await
        .unwrap();

    assert!(acme.id().ends_with("/account/1"), "{}", acme.id());
    assert_eq!(service.accounts(), 1);
    assert_eq!(
        format!("{key:?}"),
        "AccountKey(hidden)",
        "the key never shows"
    );
}

#[tokio::test]
async fn a_saved_account_key_signs_in_again_without_a_new_account() {
    let service = acme_service::start().await;
    let directory = Directory::Other(service.directory.clone());
    let (first, key) = Acme::sign_up(&outbound(), &directory, AgreedToTerms)
        .await
        .unwrap();

    let saved = key.to_json().unwrap();
    let again = Acme::sign_in(&outbound(), AccountKey::from_json(&saved).unwrap())
        .await
        .unwrap();

    assert_eq!(again.id(), first.id());
    assert_eq!(service.accounts(), 1);
}

#[test]
fn a_file_that_is_not_a_saved_key_is_refused() {
    let error = AccountKey::from_json(b"{\"id\": 1}").unwrap_err();

    assert!(matches!(error, HttpsError::AccountFile { .. }), "{error:?}");
}

#[tokio::test]
async fn an_inward_directory_is_refused_by_the_outbound_guard() {
    let public_only = Outbound::with_resolver(
        Limits::default(),
        Resolver::for_tests(Vec::new(), Reach::Public),
    )
    .unwrap();
    let directory = Directory::Other("http://127.0.0.1:9/dir".to_owned());

    let error = Acme::sign_up(&public_only, &directory, AgreedToTerms)
        .await
        .unwrap_err();

    assert!(matches!(error, HttpsError::Acme { .. }), "{error:?}");
}
