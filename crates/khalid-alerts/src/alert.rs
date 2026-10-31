//! An alert: its kind, the findings it raises, and the words an owner
//! reads.

use std::fmt::Write as _;

use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Serialize, Serializer};

/// What an alert is about. The wire name, such as `stale-post`, is the
/// `kind` of the `alert.raised` webhook and starts each finding's key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Kind {
    /// Links on the blog that lead to broken pages, from `khalid audit`.
    BrokenLinks,
    /// Far more 404s in an hour than usual.
    NotFoundSpike,
    /// The HTTPS certificate is close to its end, or past it.
    Certificate,
    /// A post whose publish time came was refused by the linter.
    FailedSchedule,
    /// A live post unchanged for longer than the site allows.
    StalePost,
}

impl Kind {
    /// Every kind, in the order alerts are sent in one round.
    pub const ALL: [Self; 5] = [
        Self::BrokenLinks,
        Self::NotFoundSpike,
        Self::Certificate,
        Self::FailedSchedule,
        Self::StalePost,
    ];

    /// The wire name, such as `stale-post`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::BrokenLinks => "broken-links",
            Self::NotFoundSpike => "not-found-spike",
            Self::Certificate => "certificate",
            Self::FailedSchedule => "failed-schedule",
            Self::StalePost => "stale-post",
        }
    }

    /// A key of this kind for `subject`, such as `stale-post:en.old-post`.
    pub fn key(self, subject: &str) -> String {
        format!("{}:{subject}", self.as_str())
    }

    /// Whether `key` was made by [`Kind::key`] of this kind.
    pub fn owns(self, key: &str) -> bool {
        key.strip_prefix(self.as_str())
            .is_some_and(|rest| rest.starts_with(':'))
    }

    /// What happened, why it matters, and what to do: one short sentence
    /// each (RULES.md BL-ERR-4).
    fn words(self) -> [&'static str; 3] {
        match self {
            Self::BrokenLinks => [
                "Links on the blog lead to pages that are broken.",
                "Readers hit a dead end, and search engines trust the site less.",
                "Run khalid audit to see each link, then fix or remove it.",
            ],
            Self::NotFoundSpike => [
                "Far more visits than usual ended on a missing page in the last hour.",
                "A broken link or a moved page may be sending readers nowhere.",
                "Add a redirect for each address below that should still work.",
            ],
            Self::Certificate => [
                "The site's HTTPS certificate is close to its end, or past it.",
                "Khalid renews it 30 days early, so renewal is failing, and browsers will warn readers away.",
                "Check that ports 80 and 443 reach this server, and read the server log for the renewal error.",
            ],
            Self::FailedSchedule => [
                "A scheduled post did not go live, because the linter refused it.",
                "It stays a draft, so readers cannot see it.",
                "Run khalid lint on the post, fix what it finds, and schedule it again.",
            ],
            Self::StalePost => [
                "Some live posts have not changed for a long time.",
                "Old facts and dead links make readers and search engines trust a post less.",
                "Read each post below, update what has changed, and save it.",
            ],
        }
    }
}

impl Serialize for Kind {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

/// One problem a check found, such as one stale post.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finding {
    /// Stays the same while the problem lasts, so it is raised once, such
    /// as `stale-post:en.old-post`.
    pub key: String,
    /// The problem in one line, such as `Old post (en.old-post), last
    /// changed 2024-03-01`.
    pub line: String,
}

/// Findings of one kind, raised together at one time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Alert {
    kind: Kind,
    what: &'static str,
    why: &'static str,
    fix: &'static str,
    findings: Vec<Finding>,
    #[serde(serialize_with = "rfc3339")]
    raised_at: DateTime<Utc>,
}

impl Alert {
    /// An alert of `kind` for `findings`, raised `at`, or `None` when there
    /// is nothing to raise.
    pub fn new(kind: Kind, findings: Vec<Finding>, at: DateTime<Utc>) -> Option<Self> {
        if findings.is_empty() {
            return None;
        }
        let [what, why, fix] = kind.words();
        Some(Self {
            kind,
            what,
            why,
            fix,
            findings,
            raised_at: at,
        })
    }

    /// What the alert is about.
    pub fn kind(&self) -> Kind {
        self.kind
    }

    /// The problems it raises.
    pub fn findings(&self) -> &[Finding] {
        &self.findings
    }

    /// An email's subject line for the site named `site`.
    pub fn subject(&self, site: &str) -> String {
        format!("{site}: {}", self.what.trim_end_matches('.'))
    }

    /// An email's plain text: what, why and fix, then one line per finding.
    pub fn text(&self) -> String {
        let mut text = format!("{}\n\n{}\n\n{}\n\n", self.what, self.why, self.fix);
        for finding in &self.findings {
            let _ = writeln!(text, "- {}", finding.line);
        }
        text
    }
}

/// `at` as RFC 3339 in UTC, to the second.
fn rfc3339<S: Serializer>(at: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&at.to_rfc3339_opts(SecondsFormat::Secs, true))
}
