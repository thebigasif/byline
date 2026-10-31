//! The speed and uptime watch's checks (C6): each turns the looks
//! `khalid serve` took at the site's public address into findings.

use chrono::{DateTime, Duration, Utc};

use crate::{Finding, Kind};

/// This many failed looks in a row mean the site is down. One or two can
/// be a dropped connection on the way.
const DOWN_IN_A_ROW: usize = 3;

/// The answer times of fewer looks than this in the hour say nothing.
const FEWEST_ANSWERS: usize = 3;

/// What one look at a page got back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Answer {
    /// The page answered with this HTTP status.
    Status(u16),
    /// Nothing answered in time: no connection, a failed name or
    /// certificate, or a timeout.
    Unreached,
}

/// One look at a page of the site, from outside, as a reader would.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Probe {
    /// When the look started.
    pub at: DateTime<Utc>,
    /// What it got back.
    pub answer: Answer,
    /// How long it took, in milliseconds, until the whole answer was read
    /// or the look gave up.
    pub millis: u32,
}

impl Probe {
    /// Whether a reader got the page: a status from 200 to 399, as a
    /// redirect leads on to it.
    pub fn is_up(&self) -> bool {
        matches!(self.answer, Answer::Status(status) if (200..400).contains(&status))
    }
}

/// A finding when the last three of `probes`, oldest first, at the page
/// `page` all failed. Its line says since when, from the first failure of
/// that run. The key names the page alone, so an outage is raised once,
/// and again after the site has answered in between.
pub fn site_down(probes: &[Probe], page: &str) -> Vec<Finding> {
    let failed = probes
        .iter()
        .rev()
        .take_while(|probe| !probe.is_up())
        .count();
    let (Some(first), Some(last)) = (
        probes.len().checked_sub(failed).and_then(|i| probes.get(i)),
        probes.last(),
    ) else {
        return Vec::new();
    };
    if failed < DOWN_IN_A_ROW {
        return Vec::new();
    }
    let since = first.at.format("%H:%M UTC on %Y-%m-%d");
    let got = match last.answer {
        Answer::Status(status) => format!("got status {status}"),
        Answer::Unreached => "got no answer".to_owned(),
    };
    vec![Finding {
        key: Kind::SiteDown.key(page),
        line: format!("{page} has not answered since {since}; the last look {got}."),
    }]
}

/// A finding when the middle answer time of the `probes` at `page` that
/// got the page in the hour before `at` is over `limit_ms`. Fewer than
/// three answers say nothing, and 0 switches the check off.
pub fn slow_site(probes: &[Probe], limit_ms: u32, page: &str, at: DateTime<Utc>) -> Vec<Finding> {
    if limit_ms == 0 {
        return Vec::new();
    }
    let since = at - Duration::hours(1);
    let mut times: Vec<u32> = probes
        .iter()
        .filter(|probe| probe.at > since && probe.is_up())
        .map(|probe| probe.millis)
        .collect();
    if times.len() < FEWEST_ANSWERS {
        return Vec::new();
    }
    times.sort_unstable();
    let count = times.len();
    match times.get(count / 2) {
        Some(&middle) if middle > limit_ms => vec![Finding {
            key: Kind::SlowSite.key(page),
            line: format!(
                "{page} took {middle} ms in the middle of {count} answers in the last hour; the limit is {limit_ms} ms."
            ),
        }],
        _ => Vec::new(),
    }
}
