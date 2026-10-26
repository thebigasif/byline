//! The engine that runs every rule on every post.

use khalid_core::Clock;
use serde::ser::{Serialize, SerializeStruct, Serializer};

use crate::builtin;
use crate::{
    Catalogue, Finding, Hit, LintConfig, LintError, LintPost, Location, Message, Rule, RuleId,
    Severity, SiteRule, SiteView,
};

/// The configured rules, each with its severity and message.
#[derive(Debug)]
pub struct Linter {
    catalogue: Catalogue,
    rules: Vec<Configured<dyn Rule>>,
    site_rules: Vec<Configured<dyn SiteRule>>,
}

/// A rule that is switched on, with what the linter shows when it fires.
#[derive(Debug)]
struct Configured<R: ?Sized> {
    severity: Severity,
    message: Message,
    rule: Box<R>,
}

impl Linter {
    /// A linter running every built-in rule with the severities and
    /// thresholds in `config`. `clock` gives today's date, read once here,
    /// for the rules that judge a post's age.
    ///
    /// # Errors
    ///
    /// Returns [`LintError::Catalogue`] or [`LintError::MissingMessage`] if
    /// the built-in catalogue is broken, which the crate's tests rule out.
    pub fn new(config: &LintConfig, clock: &dyn Clock) -> Result<Self, LintError> {
        let thresholds = config.thresholds();
        let today = clock.now().date_naive();
        Self::with_rules(config, builtin::rules(thresholds))?
            .with_site_rules(config, builtin::site_rules(thresholds, today))
    }

    /// A linter running the post rules `rules` with the severities in
    /// `config`, and no site rules.
    ///
    /// # Errors
    ///
    /// Returns [`LintError::MissingMessage`] for a rule the catalogue has no
    /// message for.
    pub fn with_rules(config: &LintConfig, rules: Vec<Box<dyn Rule>>) -> Result<Self, LintError> {
        let mut linter = Self {
            catalogue: Catalogue::builtin()?,
            rules: Vec::new(),
            site_rules: Vec::new(),
        };
        for rule in rules {
            let (id, default) = (rule.id(), rule.default_severity());
            if let Some((severity, message)) = linter.configure(config, id, default)? {
                linter.rules.push(Configured {
                    severity,
                    message,
                    rule,
                });
            }
        }
        Ok(linter)
    }

    /// This linter with the site rules `rules` added, with the severities in
    /// `config`.
    ///
    /// # Errors
    ///
    /// Returns [`LintError::MissingMessage`] for a rule the catalogue has no
    /// message for.
    pub fn with_site_rules(
        mut self,
        config: &LintConfig,
        rules: Vec<Box<dyn SiteRule>>,
    ) -> Result<Self, LintError> {
        for rule in rules {
            let (id, default) = (rule.id(), rule.default_severity());
            if let Some((severity, message)) = self.configure(config, id, default)? {
                self.site_rules.push(Configured {
                    severity,
                    message,
                    rule,
                });
            }
        }
        Ok(self)
    }

    /// The severity and message for rule `id`, or `None` when it is off.
    fn configure(
        &self,
        config: &LintConfig,
        id: RuleId,
        default: Severity,
    ) -> Result<Option<(Severity, Message)>, LintError> {
        let message = self
            .catalogue
            .get(id.as_str())
            .cloned()
            .ok_or(LintError::MissingMessage { rule: id })?;
        let severity = config.severity(id).unwrap_or(default);
        Ok((severity != Severity::Off).then_some((severity, message)))
    }

    /// Runs the post rules on one post: the check before it is published.
    pub fn lint_post(&self, post: &LintPost, site: &dyn SiteView) -> Report {
        let mut findings = Vec::new();
        for configured in &self.rules {
            for hit in configured.rule.check(post, site) {
                findings.push(configured.finding(configured.rule.id(), post, hit));
            }
        }
        Report { findings }
    }

    /// Whether the publish gate would refuse `post`, the same answer as
    /// [`Linter::gate`] gives. It runs only the rules set to block and stops
    /// at the first hit, so checking every post of a large site stays fast:
    /// the warnings, several of which compare a post with every other post,
    /// cannot change the answer.
    pub fn refuses(&self, post: &LintPost, site: &dyn SiteView) -> bool {
        self.rules
            .iter()
            .filter(|configured| configured.severity == Severity::Block)
            .any(|configured| !configured.rule.check(post, site).is_empty())
    }

    /// Runs the site rules once over the whole site: the checks that only
    /// make sense after posts are published, such as orphan posts.
    pub fn site_health(&self, site: &dyn SiteView) -> Report {
        let mut findings = Vec::new();
        for configured in &self.site_rules {
            for (post, hit) in configured.rule.check(site) {
                findings.push(configured.finding(configured.rule.id(), post, hit));
            }
        }
        Report { findings }
    }

    /// Every rule on every post: the post rules post by post in site order,
    /// then the site rules.
    pub fn lint_site(&self, site: &dyn SiteView) -> Report {
        let mut findings: Vec<Finding> = site
            .posts()
            .iter()
            .flat_map(|post| self.lint_post(post, site).findings)
            .collect();
        findings.extend(self.site_health(site).findings);
        Report { findings }
    }
}

impl<R: ?Sized> Configured<R> {
    fn finding(&self, rule: RuleId, post: &LintPost, hit: Hit) -> Finding {
        let (subject, suggestions) = hit.into_parts();
        let location = Location {
            locale: post.locale.clone(),
            slug: post.slug.clone(),
            subject,
        };
        Finding::new(
            rule,
            self.severity,
            location,
            self.message.clone(),
            suggestions,
        )
    }
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

/// Serialised as `{"blocks": true, "findings": [...]}`.
impl Serialize for Report {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut body = serializer.serialize_struct("Report", 2)?;
        body.serialize_field("blocks", &self.blocks())?;
        body.serialize_field("findings", &self.findings)?;
        body.end()
    }
}
