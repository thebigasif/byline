//! The list of rules this crate ships.

use chrono::NaiveDate;

use crate::focus::{FocusNotIn, Place};
use crate::{
    Rule, SiteRule, Thresholds, address, blocking, collections, link_hygiene, link_targets,
    link_text, reading, site_content, site_links, snippet, structure, topics, words,
};

/// Every built-in rule, measured against `thresholds`, in the order their
/// findings are listed: the blocks first, then the warnings.
pub(crate) fn rules(thresholds: Thresholds) -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(blocking::NoTitle),
        Box::new(blocking::ManyHeadlines),
        Box::new(blocking::NoDate),
        Box::new(blocking::NoAuthor),
        Box::new(blocking::NoInternalLink),
        Box::new(blocking::InvalidStructuredData),
        Box::new(snippet::LongTitle {
            max: thresholds.title_max_chars,
        }),
        Box::new(snippet::DuplicateTitle),
        Box::new(snippet::NoDescription),
        Box::new(snippet::LongDescription {
            max: thresholds.description_max_chars,
        }),
        Box::new(snippet::DuplicateDescription),
        Box::new(structure::HeadingOrder),
        Box::new(structure::MissingAlt),
        Box::new(structure::ThinPost {
            min_words: thresholds.thin_post_words,
        }),
        Box::new(link_text::ReusedLinkText),
        Box::new(link_text::RepeatedLinkText {
            max: thresholds.same_link_text_max,
        }),
        Box::new(link_text::VagueLinkText),
        Box::new(link_text::SameTargetTwice),
        Box::new(link_text::EmptyLinkText),
        Box::new(structure::NoOutboundLink),
        Box::new(link_hygiene::NofollowInternalLink),
        Box::new(link_hygiene::InsecureInternalLink),
        Box::new(link_hygiene::TooManyLinks {
            max: thresholds.max_links,
        }),
        Box::new(link_targets::RedirectingLink),
        Box::new(link_targets::CrossSiteLink),
        Box::new(structure::FormWithoutDestination),
        Box::new(reading::HardToRead {
            max: thresholds.hard_reading_index,
        }),
        Box::new(FocusNotIn(Place::Title)),
        Box::new(FocusNotIn(Place::Description)),
        Box::new(FocusNotIn(Place::FirstParagraph)),
        Box::new(FocusNotIn(Place::Subheading)),
        Box::new(FocusNotIn(Place::Address)),
        Box::new(FocusNotIn(Place::ImageAlt)),
        Box::new(collections::OverlappingCollections),
        Box::new(address::BrokenWikilink),
        Box::new(words::KeywordStuffing {
            max_percent: thresholds.keyword_density_percent,
        }),
        Box::new(address::LongSlug {
            max: thresholds.slug_max_chars,
        }),
        Box::new(address::SlugStopwords),
        Box::new(address::SlugDate),
        Box::new(address::UnicodeSlug),
        Box::new(topics::NearDuplicate {
            max_percent: thresholds.near_duplicate_percent,
        }),
        Box::new(topics::GlossaryTermNotLinked),
        Box::new(topics::ClusterWithoutPillar),
    ]
}

/// Every built-in site rule, in the order their findings are listed. `today`
/// is the date the age rules measure from.
pub(crate) fn site_rules(thresholds: Thresholds, today: NaiveDate) -> Vec<Box<dyn SiteRule>> {
    vec![
        Box::new(site_links::OrphanPost),
        Box::new(site_links::OneInboundLink),
        Box::new(site_links::TooDeep {
            max: thresholds.max_clicks,
        }),
        Box::new(site_content::MissingTranslation),
        Box::new(site_content::StalePost {
            months: thresholds.stale_months,
            today,
        }),
    ]
}
