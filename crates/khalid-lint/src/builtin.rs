//! The list of rules this crate ships.

use crate::{
    Rule, SiteRule, Thresholds, address, blocking, link_text, site_links, snippet, structure, words,
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
        Box::new(address::BrokenWikilink),
        Box::new(words::KeywordStuffing {
            max_percent: thresholds.keyword_density_percent,
        }),
        Box::new(address::LongSlug {
            max: thresholds.slug_max_chars,
        }),
        Box::new(address::SlugStopwords),
        Box::new(address::SlugDate),
    ]
}

/// Every built-in site rule, in the order their findings are listed.
pub(crate) fn site_rules() -> Vec<Box<dyn SiteRule>> {
    vec![Box::new(site_links::OrphanPost)]
}
