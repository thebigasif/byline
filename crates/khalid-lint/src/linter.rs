//! The engine that runs every rule on every post.

use crate::{
    Catalogue, Finding, LintConfig, LintError, LintPost, Location, Message, Rule, Severity,
    SiteView,
};
use crate::{blocking, link_text};

/// The configured rules, each with its severity and message.
#[derive(Debug)]
pub struct Linter {
    rules: Vec<Configured>,
}

/// A rule that is switched on, with what the linter shows when it fires.
#[derive(Debug)]
struct Configured {
    rule: Box<dyn Rule>,
    severity: Severity,
    message: Message,
}

impl Linter {
    /// A linter running every built-in rule with the severities in `config`.
    ///
    /// # Errors
    ///
    /// Returns [`LintError::Catalogue`] or [`LintError::MissingMessage`] if
    /// the built-in catalogue is broken, which the crate's tests rule out.
    pub fn new(config: &LintConfig) -> Result<Self, LintError> {
        Self::with_rules(config, builtin_rules())
    }

    /// A linter running `rules` with the severities in `config`.
    ///
    /// # Errors
    ///
    /// Returns [`LintError::MissingMessage`] for a rule the catalogue has no
    /// message for.
    pub fn with_rules(config: &LintConfig, rules: Vec<Box<dyn Rule>>) -> Result<Self, LintError> {
        let catalogue = Catalogue::builtin()?;
        let mut configured = Vec::with_capacity(rules.len());
        for rule in rules {
            let id = rule.id();
            let message = catalogue
                .get(id.as_str())
                .cloned()
                .ok_or(LintError::MissingMessage { rule: id })?;
            let severity = config
                .severity(id)
                .unwrap_or_else(|| rule.default_severity());
            if severity != Severity::Off {
                configured.push(Configured {
                    rule,
                    severity,
                    message,
                });
            }
        }
        Ok(Self { rules: configured })
    }

    /// Runs every rule on one post.
    pub fn lint_post(&self, post: &LintPost, site: &dyn SiteView) -> Report {
        let mut findings = Vec::new();
        for configured in &self.rules {
            for hit in configured.rule.check(post, site) {
                let location = Location {
                    locale: post.locale.clone(),
                    slug: post.slug.clone(),
                    subject: hit.into_subject(),
                };
                findings.push(Finding::new(
                    configured.rule.id(),
                    configured.severity,
                    location,
                    configured.message.clone(),
                ));
            }
        }
        Report { findings }
    }

    /// Runs every rule on every post of `site`, post by post in site order.
    pub fn lint_site(&self, site: &dyn SiteView) -> Report {
        let findings = site
            .posts()
            .iter()
            .flat_map(|post| self.lint_post(post, site).findings)
            .collect();
        Report { findings }
    }
}

/// Every rule this crate ships, in the order their findings are listed.
fn builtin_rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(blocking::NoTitle),
        Box::new(blocking::ManyHeadlines),
        Box::new(blocking::NoDate),
        Box::new(blocking::NoAuthor),
        Box::new(blocking::NoInternalLink),
        Box::new(link_text::ReusedLinkText),
        Box::new(link_text::RepeatedLinkText),
        Box::new(link_text::VagueLinkText),
        Box::new(link_text::SameTargetTwice),
        Box::new(link_text::EmptyLinkText),
    ]
}

/// The findings of one run, in the order the linter found them.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Report {
    findings: Vec<Finding>,
}

impl Report {
    /// Every finding, blocks and warnings together.
    pub fn findings(&self) -> &[Finding] {
        &self.findings
    }

    /// Whether any finding stops the post from being published.
    pub fn blocks(&self) -> bool {
        self.findings
            .iter()
            .any(|finding| finding.severity() == Severity::Block)
    }
}
