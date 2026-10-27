//! A post's score out of 100: a summary of its findings for the dashboard,
//! which points at the findings and never replaces them.

use serde::Serialize;

use crate::{Finding, LintPost, RuleId, Severity};

/// What a rule set to block costs, once, however often it fires.
const BLOCK_POINTS: u8 = 30;

/// What a rule set to warn costs, once, however often it fires.
const WARN_POINTS: u8 = 5;

/// The highest score a post the publish gate refuses can reach, so a post
/// that cannot go live never looks nearly done.
const REFUSED_MAX: u8 = 40;

/// A post's score out of 100, with the rules that cost it points.
///
/// Each rule that found a problem in the post costs points once: 30 for a
/// block, 5 for a warning. Twenty images without a description cost as much
/// as one, because the writer fixes them with one lesson. A post with a
/// block scores at most 40. The score never goes below 0.
///
/// Serialised as `{"value": 85, "lost": [{"rule": "missing-alt", "points": 5}]}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Score {
    value: u8,
    lost: Vec<Loss>,
}

/// The points one rule cost a post.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Loss {
    rule: RuleId,
    points: u8,
}

impl Loss {
    /// The rule whose findings cost the points.
    pub fn rule(&self) -> RuleId {
        self.rule
    }

    /// How many points the rule cost.
    pub fn points(&self) -> u8 {
        self.points
    }
}

impl Score {
    /// The score of `post` from `findings`, which may hold findings about
    /// other posts too; only those about `post` count.
    pub(crate) fn of(post: &LintPost, findings: &[Finding]) -> Self {
        let mut lost: Vec<Loss> = Vec::new();
        let mut refused = false;
        let own = findings.iter().filter(|finding| {
            let at = finding.location();
            at.locale == post.locale && at.slug == post.slug
        });
        for finding in own {
            let points = match finding.severity() {
                Severity::Block => BLOCK_POINTS,
                Severity::Warn => WARN_POINTS,
                Severity::Off => 0,
            };
            refused |= finding.severity() == Severity::Block;
            if points > 0 && lost.iter().all(|loss| loss.rule != finding.rule()) {
                lost.push(Loss {
                    rule: finding.rule(),
                    points,
                });
            }
        }
        let total = lost
            .iter()
            .fold(0_u8, |total, loss| total.saturating_add(loss.points));
        let value = 100_u8.saturating_sub(total);
        let value = if refused {
            value.min(REFUSED_MAX)
        } else {
            value
        };
        Self { value, lost }
    }

    /// The score, from 0 to 100.
    pub fn value(&self) -> u8 {
        self.value
    }

    /// The rules that cost points, in the order of their first finding, so
    /// the dashboard can link each one to its message.
    pub fn lost(&self) -> &[Loss] {
        &self.lost
    }
}
