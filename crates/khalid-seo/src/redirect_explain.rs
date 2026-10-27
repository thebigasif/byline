//! The debug view for one address: which rule matched, each hop its chain
//! takes, and the one step a visitor gets after the chain is collapsed.

use std::fmt;

use crate::{Move, Redirect, UrlPath};

/// The rule an address matched first.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Matched {
    /// The exact rule at position `rule`.
    Exact {
        /// The rule's position in the table.
        rule: usize,
    },
    /// The pattern at position `rule`, numbered after the exact rules.
    Pattern {
        /// The rule's position in the table.
        rule: usize,
        /// The pattern as written.
        pattern: String,
    },
}

/// One hop of a chain: an address and what the rule for it answers.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Hop {
    /// The address this hop starts from.
    pub from: UrlPath,
    /// The position of the rule that answered it.
    pub rule: usize,
    /// What that rule answers on its own.
    pub answer: Redirect,
}

/// Why an address answers what it answers.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Explanation {
    /// The address asked about.
    pub path: UrlPath,
    /// The rule it matched, or `None` when no rule did.
    pub matched: Option<Matched>,
    /// Each hop from the address to the end of its chain, in order.
    pub hops: Vec<Hop>,
    /// The one answer a visitor gets, or `None` when no rule matched.
    pub answer: Option<Redirect>,
}

impl Explanation {
    pub(crate) fn unmatched(path: UrlPath) -> Self {
        Self {
            path,
            matched: None,
            hops: Vec::new(),
            answer: None,
        }
    }
}

/// The view in plain words: the match, a numbered line per hop, then what
/// a visitor gets.
impl fmt::Display for Explanation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let path = &self.path;
        let (Some(matched), Some(answer)) = (&self.matched, &self.answer) else {
            return writeln!(
                f,
                "{path} matches no redirect rule. \
                 The page there answers, or the not-found page when there is none."
            );
        };
        match matched {
            Matched::Exact { rule } => {
                writeln!(f, "{path} matches rule {rule}, the exact path {path}.")?;
            }
            Matched::Pattern { rule, pattern } => {
                writeln!(f, "{path} matches rule {rule}, the pattern {pattern}.")?;
            }
        }
        for (number, hop) in self.hops.iter().enumerate() {
            let (step, from, rule) = (number + 1, &hop.from, hop.rule);
            writeln!(f, "{step}. {from} {}, by rule {rule}.", Plain(&hop.answer))?;
        }
        writeln!(f, "A visitor gets one step: {}.", Plain(answer))
    }
}

/// A redirect in plain words, such as `moves for good (301) to /blog/new/`.
struct Plain<'a>(&'a Redirect);

impl fmt::Display for Plain<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Redirect::Move { kind, to } => {
                let how = match kind {
                    Move::Permanent => "moves for good (301)",
                    Move::Found => "moves for now (302)",
                    Move::Temporary => "moves for now, sent again as is (307)",
                };
                write!(f, "{how} to {to}")
            }
            Redirect::Gone => f.write_str("is gone (410)"),
            Redirect::LegalBlock => f.write_str("is blocked for a legal reason (451)"),
        }
    }
}
