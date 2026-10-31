//! The rule engine: severities from `lint.toml`, findings and their messages.

use chrono::{TimeZone, Utc};
use khalid_core::{FixedClock, Locale, Slug};
use khalid_lint::{
    Finding, Hit, Image, LinkKind, LintConfig, LintError, LintLink, LintPost, LintSite, Linter,
    Report, Rule, RuleId, Severity,
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

/// The day every test runs on.
#[cfg(test)]
fn clock() -> FixedClock {
    FixedClock::new(Utc.with_ymd_and_hms(2026, 10, 26, 9, 0, 0).unwrap())
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

#[test]
fn a_threshold_in_lint_toml_reaches_its_rule() {
    let en = Locale::try_new("en").unwrap();
    let mut site = LintSite::default();
    for slug in ["first-post", "second-post"] {
        let mut post = LintPost::new(en.clone(), Slug::try_new(slug).unwrap());
        let link = LintLink::new(LinkKind::Internal, "/blog/link-text/", "choosing link text");
        post.links.push(link);
        site.posts.push(post);
    }
    let repeated = |lint_toml: &str| {
        let linter = Linter::new(&LintConfig::parse(lint_toml).unwrap(), &clock()).unwrap();
        let report = linter.lint_post(&site.posts[0], &site);
        let rules: Vec<_> = report.findings().iter().map(Finding::rule).collect();
        rules.contains(&RuleId::new("repeated-link-text"))
    };
    assert!(!repeated(""), "two links are under the default of three");
    assert!(repeated("[thresholds]\nsame-link-text-max = 1\n"));
}

#[test]
fn site_rules_run_on_the_whole_site_but_never_before_publishing() {
    let mut site = undated_site();
    // A language's only post is never an orphan, so the site gets a second.
    let second = LintPost::new(
        site.posts[0].locale.clone(),
        Slug::try_new("second").unwrap(),
    );
    site.posts.push(second);
    let linter = Linter::new(&LintConfig::default(), &clock()).unwrap();
    let orphan = |report: &Report| {
        let mut rules = report.findings().iter().map(Finding::rule);
        rules.any(|rule| rule == RuleId::new("orphan-post"))
    };
    assert!(!orphan(&linter.lint_post(&site.posts[0], &site)));
    assert!(orphan(&linter.site_health(&site)));
    assert!(orphan(&linter.lint_site(&site)));
    let quiet = LintConfig::parse("[rules]\norphan-post = \"off\"\n").unwrap();
    let linter = Linter::new(&quiet, &clock()).unwrap();
    assert!(
        !orphan(&linter.site_health(&site)),
        "lint.toml turns it off"
    );
}
