//! The rule engine: severities from `lint.toml`, findings and their messages.

use khalid_core::{Locale, Slug};
use khalid_lint::{
    Hit, Image, LintConfig, LintError, LintPost, LintSite, Linter, Report, Rule, RuleId, Severity,
};

/// A stand-in rule that fires on every post without a date.
#[derive(Debug)]
struct NoDate;

impl Rule for NoDate {
    fn id(&self) -> RuleId {
        RuleId::new("no-date")
    }

    fn default_severity(&self) -> Severity {
        Severity::Block
    }

    fn check(&self, post: &LintPost, _site: &dyn khalid_lint::SiteView) -> Vec<Hit> {
        if post.date.is_none() {
            vec![Hit::post()]
        } else {
            Vec::new()
        }
    }
}

#[cfg(test)]
fn undated_site() -> LintSite {
    let post = LintPost::new(
        Locale::try_new("en").unwrap(),
        Slug::try_new("why-internal-links-matter").unwrap(),
    );
    let mut site = LintSite::default();
    site.posts.push(post);
    site
}

#[cfg(test)]
fn linter(lint_toml: &str) -> Linter {
    let config = LintConfig::parse(lint_toml).unwrap();
    Linter::with_rules(&config, vec![Box::new(NoDate)]).unwrap()
}

#[test]
fn a_blocking_rule_blocks_the_report_and_carries_its_message() {
    let report = linter("").lint_site(&undated_site());
    assert!(report.blocks());
    let [finding] = report.findings() else {
        panic!("one finding, got {:?}", report.findings());
    };
    assert_eq!(finding.rule().as_str(), "no-date");
    assert_eq!(finding.severity(), Severity::Block);
    assert_eq!(
        finding.location().slug.as_str(),
        "why-internal-links-matter"
    );
    assert_eq!(finding.message().what(), "This post has no date.");
    assert_eq!(finding.message().guide().as_str(), "post-dates");
}

#[test]
fn lint_toml_turns_a_block_into_a_warning() {
    let report = linter("[rules]\nno-date = \"warn\"\n").lint_site(&undated_site());
    assert!(!report.blocks());
    assert_eq!(report.findings().len(), 1);
    assert_eq!(report.findings()[0].severity(), Severity::Warn);
}

#[test]
fn lint_toml_turns_a_rule_off() {
    let report = linter("[rules]\nno-date = \"off\"\n").lint_site(&undated_site());
    assert_eq!(report.findings(), []);
}

#[test]
fn findings_follow_site_order_and_name_their_subject() {
    /// Fires once per image with no description, naming the image. It
    /// borrows a catalogue ID, because this test is about order, not words.
    #[derive(Debug)]
    struct NoAlt;
    impl Rule for NoAlt {
        fn id(&self) -> RuleId {
            RuleId::new("no-date")
        }
        fn default_severity(&self) -> Severity {
            Severity::Warn
        }
        fn check(&self, post: &LintPost, _: &dyn khalid_lint::SiteView) -> Vec<Hit> {
            let missing = post.images.iter().filter(|image| image.alt.is_none());
            missing.map(|image| Hit::about(image.src.clone())).collect()
        }
    }
    let en = Locale::try_new("en").unwrap();
    let mut site = LintSite::default();
    for (slug, src) in [("second-post", "chart.png"), ("first-post", "photo.jpg")] {
        let mut post = LintPost::new(en.clone(), Slug::try_new(slug).unwrap());
        post.images.push(Image::new(src, None));
        post.images
            .push(Image::new("logo.png", Some("Khalid logo".to_owned())));
        site.posts.push(post);
    }
    let linter = Linter::with_rules(&LintConfig::default(), vec![Box::new(NoAlt)]).unwrap();
    let report = linter.lint_site(&site);
    let seen: Vec<(&str, Option<&str>)> = report
        .findings()
        .iter()
        .map(|f| (f.location().slug.as_str(), f.location().subject.as_deref()))
        .collect();
    assert_eq!(
        seen,
        [
            ("second-post", Some("chart.png")),
            ("first-post", Some("photo.jpg"))
        ]
    );
    assert!(!report.blocks(), "warnings never block");
}

#[test]
fn an_empty_report_does_not_block() {
    assert!(!Report::default().blocks());
}

#[test]
fn a_rule_without_a_catalogue_message_is_refused() {
    #[derive(Debug)]
    struct Unwritten;
    impl Rule for Unwritten {
        fn id(&self) -> RuleId {
            RuleId::new("unwritten")
        }
        fn default_severity(&self) -> Severity {
            Severity::Warn
        }
        fn check(&self, _: &LintPost, _: &dyn khalid_lint::SiteView) -> Vec<Hit> {
            Vec::new()
        }
    }
    let error = Linter::with_rules(&LintConfig::default(), vec![Box::new(Unwritten)]).unwrap_err();
    assert!(
        matches!(&error, LintError::MissingMessage { rule } if rule.as_str() == "unwritten"),
        "{error:?}"
    );
}
