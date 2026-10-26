//! [`MetaTemplates`]: the title and description templates per page type.

use std::collections::BTreeMap;

use crate::PageKind;
use crate::meta_template::{MetaTemplate, Piece, Variable};

/// The title and description templates for one page type.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct TemplatePair {
    /// The `<title>` template.
    pub title: MetaTemplate,
    /// The meta description template.
    pub description: MetaTemplate,
}

impl TemplatePair {
    /// A pair from a title and a description template.
    pub fn new(title: MetaTemplate, description: MetaTemplate) -> Self {
        Self { title, description }
    }
}

/// Title and description templates per page type.
///
/// Every page type has a default: the home page shows `{site} {sep}
/// {tagline}` and describes itself with `{tagline}`; every other page shows
/// `{title} {sep} {site}` and describes itself with `{excerpt}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetaTemplates {
    by_kind: BTreeMap<PageKind, TemplatePair>,
    fallback: TemplatePair,
}

impl MetaTemplates {
    /// Sets the templates for one page type.
    pub fn set(&mut self, kind: PageKind, pair: TemplatePair) {
        self.by_kind.insert(kind, pair);
    }

    /// The templates for one page type.
    pub fn for_kind(&self, kind: PageKind) -> &TemplatePair {
        self.by_kind.get(&kind).unwrap_or(&self.fallback)
    }
}

impl Default for MetaTemplates {
    fn default() -> Self {
        let pair = |first, last, description| {
            TemplatePair::new(
                MetaTemplate::from_pieces(vec![
                    Piece::Var(first),
                    Piece::Text(" ".to_owned()),
                    Piece::Var(Variable::Sep),
                    Piece::Text(" ".to_owned()),
                    Piece::Var(last),
                ]),
                MetaTemplate::from_pieces(vec![Piece::Var(description)]),
            )
        };
        Self {
            by_kind: BTreeMap::from([(
                PageKind::Home,
                pair(Variable::Site, Variable::Tagline, Variable::Tagline),
            )]),
            fallback: pair(Variable::Title, Variable::Site, Variable::Excerpt),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::MetaTemplates;
    use crate::PageKind;

    #[test]
    fn the_default_templates_print_as_their_source() {
        let templates = MetaTemplates::default();
        let home = templates.for_kind(PageKind::Home);
        assert_eq!(home.title.as_str(), "{site} {sep} {tagline}");
        assert_eq!(home.description.as_str(), "{tagline}");
        let post = templates.for_kind(PageKind::Post);
        assert_eq!(post.title.as_str(), "{title} {sep} {site}");
        assert_eq!(post.description.as_str(), "{excerpt}");
    }
}
