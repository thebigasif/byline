//! The checks: each turns what the binary read into findings, and reads
//! nothing itself.

use std::cmp::Reverse;
use std::collections::BTreeMap;

use chrono::{DateTime, Duration, Utc};

use crate::{Finding, Kind};

/// The most addresses a 404 spike names; the rest are noise.
const MOST_ADDRESSES: usize = 10;

/// A certificate with fewer days than this left is raised. Renewal starts
/// at 30, so 14 means two weeks of failed tries.
const CERTIFICATE_DAYS: i64 = 14;

/// The addresses whose 404 count grew most since the last reading, when
/// together they grew by `spike` or more. `before` and `now` map each
/// address to its count, as the 404 log keeps it; an address new since
/// `before` grew by its whole count. Keys name the hour of `at`, so a
/// spike that goes on is raised again each hour it is seen.
pub fn not_found_spike(
    before: &BTreeMap<String, u64>,
    now: &BTreeMap<String, u64>,
    spike: u32,
    at: DateTime<Utc>,
) -> Vec<Finding> {
    let mut grown: Vec<(&String, u64)> = now
        .iter()
        .map(|(path, hits)| {
            let was = before.get(path).copied().unwrap_or(0);
            (path, hits.saturating_sub(was))
        })
        .filter(|(_, grew)| *grew > 0)
        .collect();
    let total = grown
        .iter()
        .fold(0_u64, |sum, (_, grew)| sum.saturating_add(*grew));
    if total < u64::from(spike) {
        return Vec::new();
    }
    grown.sort_by_key(|(path, grew)| (Reverse(*grew), *path));
    let hour = at.format("%Y-%m-%dT%H");
    grown
        .into_iter()
        .take(MOST_ADDRESSES)
        .map(|(path, grew)| Finding {
            key: Kind::NotFoundSpike.key(&format!("{hour}:{path}")),
            line: format!("{path}: {grew} more 404s"),
        })
        .collect()
}

/// The certificate `khalid serve --domain` answers with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServedCertificate {
    /// The domain it is for, such as `blog.example.com`.
    pub domain: String,
    /// When it stops being valid.
    pub not_after: DateTime<Utc>,
}

/// A finding when `served` is past its end at `at`, or has fewer than 14
/// days left. The key names the end date, so a renewed certificate that
/// later fails again is raised again.
pub fn certificate(served: &ServedCertificate, at: DateTime<Utc>) -> Vec<Finding> {
    let domain = &served.domain;
    let day = served.not_after.format("%Y-%m-%d");
    let (subject, line) = if served.not_after <= at {
        (
            format!("{domain}:{day}:ended"),
            format!("The certificate for {domain} ended on {day}."),
        )
    } else if served.not_after - at < Duration::days(CERTIFICATE_DAYS) {
        (
            format!("{domain}:{day}"),
            format!("The certificate for {domain} ends on {day}."),
        )
    } else {
        return Vec::new();
    };
    let key = Kind::Certificate.key(&subject);
    vec![Finding { key, line }]
}

/// A post that is live, with when it last changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LivePost {
    /// Its identity, `<language>.<slug>`.
    pub id: String,
    /// Its title.
    pub title: String,
    /// When it was last published or changed.
    pub changed: DateTime<Utc>,
}

/// The posts in `live` that have not changed in more than `after_days`
/// days at `at`, oldest first. 0 days switches the check off.
pub fn stale_posts(live: &[LivePost], after_days: u32, at: DateTime<Utc>) -> Vec<Finding> {
    if after_days == 0 {
        return Vec::new();
    }
    let since = at - Duration::days(i64::from(after_days));
    let mut stale: Vec<&LivePost> = live.iter().filter(|post| post.changed < since).collect();
    stale.sort_by_key(|post| (post.changed, &post.id));
    stale
        .into_iter()
        .map(|post| Finding {
            key: Kind::StalePost.key(&post.id),
            line: format!(
                "{} ({}), last changed {}",
                post.title,
                post.id,
                post.changed.format("%Y-%m-%d")
            ),
        })
        .collect()
}
