//! The redirect table: moved, gone and legally blocked addresses, with
//! chains collapsed into one hop.

use std::collections::BTreeMap;
use std::fmt;

use khalid_core::AbsoluteUrl;

use crate::{PatternRedirect, PatternRule, SeoError, UrlPath};

/// The longest chain followed before a table is refused as a loop.
const MAX_HOPS: usize = 32;

/// How a moved address answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Move {
    /// 301: moved for good. Search engines move the ranking to the new address.
    Permanent,
    /// 302: found elsewhere for now.
    Found,
    /// 307: elsewhere for now, and the browser must repeat the same request.
    Temporary,
}

impl Move {
    pub(crate) fn status(self) -> u16 {
        match self {
            Self::Permanent => 301,
            Self::Found => 302,
            Self::Temporary => 307,
        }
    }
}

/// Where a moved address now lives.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Destination {
    /// A path on this site.
    Path(UrlPath),
    /// An address on another site.
    External(AbsoluteUrl),
}

impl fmt::Display for Destination {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Path(path) => f.write_str(path.as_str()),
            Self::External(url) => f.write_str(url.as_str()),
        }
    }
}

/// What an old address answers.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Redirect {
    /// 301, 302 or 307 to a new address.
    Move {
        /// Which kind of move.
        kind: Move,
        /// The new address.
        to: Destination,
    },
    /// 410: removed on purpose, so search engines drop it quickly.
    Gone,
    /// 451: blocked for a legal reason.
    LegalBlock,
}

impl Redirect {
    /// The HTTP status code.
    pub fn status(&self) -> u16 {
        match self {
            Self::Move { kind, .. } => kind.status(),
            Self::Gone => 410,
            Self::LegalBlock => 451,
        }
    }

    /// The `Location` header, for a move.
    pub fn location(&self) -> Option<&Destination> {
        match self {
            Self::Move { to, .. } => Some(to),
            Self::Gone | Self::LegalBlock => None,
        }
    }
}

/// One row of the table: an old path and what it answers.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct RedirectRule {
    /// The old path, inside the mount folder.
    pub from: UrlPath,
    /// What it answers.
    pub redirect: Redirect,
}

impl RedirectRule {
    /// A rule for `from`.
    pub fn new(from: UrlPath, redirect: Redirect) -> Self {
        Self { from, redirect }
    }
}

/// One line of a redirect list: a rule for one path, or a pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RedirectEntry {
    /// A rule for exactly one path.
    Exact(RedirectRule),
    /// A rule for every path its regular expression matches.
    Pattern(PatternRule),
}

/// The answer for a path, with any chain already followed to its end.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Resolution {
    /// The position of the rule that matched, for the debug view.
    pub rule: usize,
    /// The answer after the chain: one hop to the final address.
    pub answer: Redirect,
}

impl Resolution {
    /// The HTTP status code to send.
    pub fn status(&self) -> u16 {
        self.answer.status()
    }

    /// The `Location` header, for a move.
    pub fn location(&self) -> Option<&Destination> {
        self.answer.location()
    }
}

/// The site's redirects, checked for duplicates and loops.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedirectTable {
    rules: Vec<RedirectRule>,
    by_source: BTreeMap<UrlPath, usize>,
    patterns: Vec<PatternRule>,
}

impl RedirectTable {
    /// Checks the rules and builds the table.
    ///
    /// # Errors
    ///
    /// [`SeoError::RedirectDuplicate`] when one path has two rules, and
    /// [`SeoError::RedirectLoop`] when following a chain comes back to a
    /// path already seen.
    pub fn new(rules: Vec<RedirectRule>) -> Result<Self, SeoError> {
        let mut by_source = BTreeMap::new();
        for (index, rule) in rules.iter().enumerate() {
            if by_source.insert(rule.from.clone(), index).is_some() {
                return Err(SeoError::RedirectDuplicate {
                    from: rule.from.as_str().to_owned(),
                });
            }
        }
        let table = Self {
            rules,
            by_source,
            patterns: Vec::new(),
        };
        for rule in &table.rules {
            if table.follow(&rule.redirect).is_none() {
                return Err(SeoError::RedirectLoop {
                    from: rule.from.as_str().to_owned(),
                });
            }
        }
        Ok(table)
    }

    /// The table with `patterns` added, tried in order after every exact
    /// rule. A pattern's answer is then followed through the exact rules,
    /// like any other chain.
    #[must_use]
    pub fn with_patterns(mut self, patterns: Vec<PatternRule>) -> Self {
        self.patterns = patterns;
        self
    }

    /// Builds the table from a list such as a CSV file holds: the exact
    /// rules in their order, then the patterns in theirs.
    ///
    /// # Errors
    ///
    /// As [`RedirectTable::new`] for the exact rules.
    pub fn from_entries(entries: Vec<RedirectEntry>) -> Result<Self, SeoError> {
        let mut rules = Vec::new();
        let mut patterns = Vec::new();
        for entry in entries {
            match entry {
                RedirectEntry::Exact(rule) => rules.push(rule),
                RedirectEntry::Pattern(pattern) => patterns.push(pattern),
            }
        }
        Ok(Self::new(rules)?.with_patterns(patterns))
    }

    /// Every rule as a list entry: the exact rules, then the patterns, in
    /// the order they are tried.
    pub fn entries(&self) -> Vec<RedirectEntry> {
        let exact = self.rules.iter().cloned().map(RedirectEntry::Exact);
        let patterns = self.patterns.iter().cloned().map(RedirectEntry::Pattern);
        exact.chain(patterns).collect()
    }

    /// The exact rules, in the order given.
    pub fn rules(&self) -> &[RedirectRule] {
        &self.rules
    }

    /// Follows `start` through the table to its end. A move stays a 301
    /// only if every hop is a 301; otherwise it takes the first other kind.
    /// `None` means the chain loops.
    fn follow(&self, start: &Redirect) -> Option<Redirect> {
        let mut current = start;
        let mut kind = Move::Permanent;
        for _ in 0..MAX_HOPS {
            let Redirect::Move { kind: hop, to } = current else {
                return Some(current.clone());
            };
            if kind == Move::Permanent {
                kind = *hop;
            }
            let next = match to {
                Destination::Path(path) => {
                    self.by_source.get(path).and_then(|&i| self.rules.get(i))
                }
                Destination::External(_) => None,
            };
            match next {
                Some(rule) => current = &rule.redirect,
                None => {
                    return Some(Redirect::Move {
                        kind,
                        to: to.clone(),
                    });
                }
            }
        }
        None
    }

    /// The answer for `path`, or `None` when no rule matches it. Exact
    /// rules are tried first, then patterns, which are numbered after the
    /// exact rules in [`Resolution::rule`].
    pub fn resolve(&self, path: &UrlPath) -> Option<Resolution> {
        let (rule, redirect) = match self.by_source.get(path) {
            Some(&index) => (index, self.rules.get(index)?.redirect.clone()),
            None => self
                .patterns
                .iter()
                .enumerate()
                .find_map(|(index, pattern)| {
                    pattern
                        .apply(path)
                        .map(|hit| (self.rules.len() + index, hit))
                })?,
        };
        let answer = self.follow(&redirect).unwrap_or(redirect);
        Some(Resolution { rule, answer })
    }
}

/// The debug view: each rule, then the answer a visitor gets after chains.
impl fmt::Display for RedirectTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, rule) in self.rules.iter().enumerate() {
            let Some(hit) = self.resolve(&rule.from) else {
                continue;
            };
            write!(f, "{index}: {} answers {}", rule.from, hit.status())?;
            if let Some(location) = hit.location() {
                write!(f, " {location}")?;
            }
            if hit.answer != rule.redirect {
                f.write_str(" (chain collapsed)")?;
            }
            writeln!(f)?;
        }
        for (offset, rule) in self.patterns.iter().enumerate() {
            let index = self.rules.len() + offset;
            write!(f, "{index}: pattern {} answers ", rule.pattern.as_str())?;
            match &rule.answer {
                PatternRedirect::Move { kind, template } => {
                    writeln!(f, "{} {template}", kind.status())?;
                }
                PatternRedirect::Gone => writeln!(f, "410")?,
                PatternRedirect::LegalBlock => writeln!(f, "451")?,
            }
        }
        Ok(())
    }
}
