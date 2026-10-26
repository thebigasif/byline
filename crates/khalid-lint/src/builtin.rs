//! The list of rules this crate ships.

use crate::{Rule, Thresholds, blocking, link_text};

/// Every built-in rule, measured against `thresholds`, in the order their
/// findings are listed: the blocks first, then the warnings.
pub(crate) fn rules(thresholds: Thresholds) -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(blocking::NoTitle),
        Box::new(blocking::ManyHeadlines),
        Box::new(blocking::NoDate),
        Box::new(blocking::NoAuthor),
        Box::new(blocking::NoInternalLink),
        Box::new(link_text::ReusedLinkText),
        Box::new(link_text::RepeatedLinkText {
            max: thresholds.same_link_text_max,
        }),
        Box::new(link_text::VagueLinkText),
        Box::new(link_text::SameTargetTwice),
        Box::new(link_text::EmptyLinkText),
    ]
}
