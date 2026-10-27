//! A certificate read from PEM: its dates, its names, when to renew it,
//! and the shared set the server answers with.

use chrono::{DateTime, Utc};
use khalid_https::{Certificate, Certificates, HttpsError};
use rcgen::{CertificateParams, KeyPair, date_time_ymd};

/// The name the test certificates are for.
const NAME: &str = "blog.test";

/// A self-signed certificate for `NAME` valid from `from` to `until`, as
/// PEM: the certificate, then its key.
#[cfg(test)]
fn self_signed(from: (i32, u8, u8), until: (i32, u8, u8)) -> (String, String) {
    let mut params = CertificateParams::new(vec![NAME.to_owned()]).unwrap();
    params.not_before = date_time_ymd(from.0, from.1, from.2);
    params.not_after = date_time_ymd(until.0, until.1, until.2);
    let key = KeyPair::generate().unwrap();
    let certificate = params.self_signed(&key).unwrap();
    (certificate.pem(), key.serialize_pem())
}

#[cfg(test)]
fn ninety_days() -> Certificate {
    let (chain, key) = self_signed((2026, 1, 1), (2026, 4, 1));
    Certificate::from_pem(chain.as_bytes(), key.as_bytes()).unwrap()
}

#[cfg(test)]
fn at(text: &str) -> DateTime<Utc> {
    text.parse().unwrap()
}

#[test]
fn a_certificate_renews_when_two_thirds_of_its_life_have_passed() {
    let certificate = ninety_days();

    assert_eq!(certificate.not_after(), at("2026-04-01T00:00:00Z"));
    assert_eq!(certificate.renew_at(), at("2026-03-02T00:00:00Z"));
}

#[test]
fn a_shorter_certificate_renews_at_the_same_share_of_its_life() {
    let (chain, key) = self_signed((2026, 1, 1), (2026, 1, 7));

    let certificate = Certificate::from_pem(chain.as_bytes(), key.as_bytes()).unwrap();

    assert_eq!(certificate.renew_at(), at("2026-01-05T00:00:00Z"));
}

#[test]
fn a_certificate_covers_its_own_name_only() {
    let certificate = ninety_days();

    assert!(certificate.covers("blog.test"));
    assert!(certificate.covers("BLOG.test"));
    assert!(!certificate.covers("shop.test"));
}

#[test]
fn a_file_with_no_certificate_is_refused() {
    let (_, key) = self_signed((2026, 1, 1), (2026, 4, 1));

    let error = Certificate::from_pem(b"not a certificate", key.as_bytes()).unwrap_err();

    assert!(matches!(error, HttpsError::NoCertificate), "{error:?}");
}

#[test]
fn a_key_that_is_not_pem_is_refused() {
    let (chain, _) = self_signed((2026, 1, 1), (2026, 4, 1));

    let error = Certificate::from_pem(chain.as_bytes(), b"not a key").unwrap_err();

    assert!(matches!(error, HttpsError::Pem { .. }), "{error:?}");
}

#[test]
fn the_key_never_shows_in_debug_output() {
    let shown = format!("{:?}", ninety_days());

    assert!(shown.contains("blog.test"), "{shown}");
    assert!(!shown.contains("CertifiedKey"), "{shown}");
}

#[test]
fn every_clone_of_the_set_sees_a_new_certificate() {
    let certificates = Certificates::new();
    let listener_side = certificates.clone();

    let before = listener_side.current();
    certificates.set(ninety_days());

    assert!(before.is_none());
    assert_eq!(
        listener_side.current().map(|c| c.not_after()),
        Some(at("2026-04-01T00:00:00Z"))
    );
}
