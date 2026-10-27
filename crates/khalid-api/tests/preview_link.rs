//! Preview links: signed, expiring addresses that show a draft to someone
//! without an account. A link checks out until it expires, and only for
//! its post (CTO answers 2026-10-27 section 4).

use std::time::Duration;

use chrono::{DateTime, TimeZone, Utc};
use khalid_api::preview_link::{PreviewError, PreviewSigner};
use khalid_core::{AbsoluteUrl, FixedClock, Secret};

#[cfg(test)]
fn at(day: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, day, 9, 0, 0).unwrap()
}

/// A signer for the demo blog with a made-up key on `day` October,
/// making links that work for 7 days.
#[cfg(test)]
fn signer(day: u32) -> PreviewSigner {
    let blog = AbsoluteUrl::parse("https://www.example.com/blog/").unwrap();
    let key = Secret::new([7_u8; 32]);
    let week = Duration::from_hours(7 * 24);
    PreviewSigner::new(&key, &blog, week, FixedClock::new(at(day)))
}

/// The `until` and `sig` of a preview link.
#[cfg(test)]
fn parts(url: &str) -> (i64, String) {
    let query = url.split_once('?').unwrap().1;
    let (until, sig) = query.split_once('&').unwrap();
    let until = until.strip_prefix("until=").unwrap().parse().unwrap();
    (until, sig.strip_prefix("sig=").unwrap().to_owned())
}

#[test]
fn a_link_names_the_post_and_checks_out_until_it_expires() {
    let link = signer(27).link("en.choosing-anchor-text").unwrap();
    let url = link.url().as_str();
    assert!(
        url.starts_with("https://www.example.com/blog/preview/en.choosing-anchor-text/?until="),
        "{url}"
    );
    assert_eq!(link.expires_at(), at(27) + chrono::TimeDelta::days(7));
    let (until, sig) = parts(url);
    assert_eq!(
        signer(27).check("en.choosing-anchor-text", until, &sig),
        Ok(())
    );
    assert_eq!(
        signer(31).check("en.choosing-anchor-text", until, &sig),
        Ok(())
    );
}

#[test]
fn a_link_that_expired_or_was_changed_is_refused() {
    let link = signer(20).link("en.choosing-anchor-text").unwrap();
    let (until, sig) = parts(link.url().as_str());
    let id = "en.choosing-anchor-text";
    assert_eq!(
        signer(27).check(id, until, &sig),
        Err(PreviewError::Expired)
    );
    let later = until + 86_400;
    assert_eq!(signer(20).check(id, later, &sig), Err(PreviewError::Forged));
    let other = "en.what-a-sitemap-does";
    assert_eq!(
        signer(20).check(other, until, &sig),
        Err(PreviewError::Forged)
    );
    assert_eq!(signer(20).check(id, until, "zz"), Err(PreviewError::Forged));
}
