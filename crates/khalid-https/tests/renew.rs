//! When to renew the certificate.

use std::time::Duration;

use chrono::{DateTime, Utc};
use khalid_https::{Certificate, Domain, Issued, Next, next};
use rcgen::{CertificateParams, KeyPair, date_time_ymd};

/// A certificate for `name` valid through the first three months of 2026,
/// as it is issued: the chain, then its key.
#[cfg(test)]
fn issued(name: &str) -> Issued {
    let mut params = CertificateParams::new(vec![name.to_owned()]).unwrap();
    params.not_before = date_time_ymd(2026, 1, 1);
    params.not_after = date_time_ymd(2026, 4, 1);
    let key = KeyPair::generate().unwrap();
    let chain = params.self_signed(&key).unwrap().pem();
    Issued {
        chain,
        key: key.serialize_pem(),
    }
}

#[cfg(test)]
fn certificate(name: &str) -> Certificate {
    let issued = issued(name);
    Certificate::from_pem(issued.chain.as_bytes(), issued.key.as_bytes()).unwrap()
}

#[cfg(test)]
fn at(text: &str) -> DateTime<Utc> {
    text.parse().unwrap()
}

#[cfg(test)]
fn blog() -> Domain {
    Domain::try_new("blog.test").unwrap()
}

#[test]
fn with_no_certificate_one_is_got_at_once() {
    assert_eq!(next(None, &blog(), at("2026-02-01T00:00:00Z")), Next::Renew);
}

#[test]
fn a_certificate_for_another_name_is_replaced_at_once() {
    let other = certificate("shop.test");

    assert_eq!(
        next(Some(&other), &blog(), at("2026-01-02T00:00:00Z")),
        Next::Renew
    );
}

#[test]
fn a_fresh_certificate_is_looked_at_again_within_half_a_day() {
    let fresh = certificate("blog.test");

    assert_eq!(
        next(Some(&fresh), &blog(), at("2026-01-02T00:00:00Z")),
        Next::Wait(Duration::from_hours(12))
    );
}

#[test]
fn a_certificate_close_to_its_renewal_time_waits_only_until_then() {
    let fresh = certificate("blog.test");

    assert_eq!(
        next(Some(&fresh), &blog(), at("2026-03-01T22:00:00Z")),
        Next::Wait(Duration::from_hours(2))
    );
}

#[test]
fn a_certificate_past_its_renewal_time_is_renewed() {
    let old = certificate("blog.test");

    assert_eq!(
        next(Some(&old), &blog(), at("2026-03-02T00:00:00Z")),
        Next::Renew
    );
}
