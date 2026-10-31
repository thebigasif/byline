//! `khalid new --template`: a starter for a comparison, alternatives,
//! best-of or review post (phase GEO, P4).
//!
//! Each starter gives the page the shape the linter's page-shape rules
//! check: headings, and a table where choices are weighed. Its guidance,
//! answer first, sits in comments at the end of the front matter, because
//! the body has no comment that stays off the page. ADR
//! `0000-page-kinds-come-from-the-title`.

use clap::ValueEnum;
use khalid_core::Locale;
use khalid_lint::PageKind;

/// A kind of post `khalid new` can start from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum Template {
    /// Two named things side by side, such as "Ahrefs vs Semrush"
    Comparison,
    /// The choices beside one thing, such as "7 Ahrefs alternatives"
    Alternatives,
    /// A ranked list, such as "The 10 best SEO tools"
    BestOf,
    /// A verdict on one thing, such as "Ahrefs review"
    Review,
}

impl Template {
    /// The name written after `--template`.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Comparison => "comparison",
            Self::Alternatives => "alternatives",
            Self::BestOf => "best-of",
            Self::Review => "review",
        }
    }

    /// A title of this kind, to show in a message.
    pub(crate) fn example(self) -> &'static str {
        match self {
            Self::Comparison => "Ahrefs vs Semrush",
            Self::Alternatives => "7 Ahrefs alternatives",
            Self::BestOf => "The 10 best SEO tools",
            Self::Review => "Ahrefs review",
        }
    }
}

/// The starter for `template` from `starters/content/en/`.
fn starter(template: Template) -> &'static str {
    match template {
        Template::Comparison => include_str!("../starters/content/en/comparison.md"),
        Template::Alternatives => include_str!("../starters/content/en/alternatives.md"),
        Template::BestOf => include_str!("../starters/content/en/best-of.md"),
        Template::Review => include_str!("../starters/content/en/review.md"),
    }
}

/// The end of a new post titled `title` in `locale`, started from
/// `template`: the guidance as front matter comments, the front matter's
/// closing line, and the body. `None` when the title does not read as that
/// kind of post, so the linter would never check its shape.
///
/// Every language gets the English starter, for the writer to translate.
pub(crate) fn body(template: Template, title: &str, locale: &Locale) -> Option<String> {
    let kind = PageKind::of_title(locale.language(), title)?;
    let text = starter(template);
    match (template, kind) {
        // The starters are plain markdown files, so the two names compared
        // go in by replacing their marks rather than through `format!`.
        (Template::Comparison, PageKind::Comparison { first, second }) => {
            Some(text.replace("{first}", &first).replace("{second}", &second))
        }
        (Template::Alternatives, PageKind::Alternatives { .. })
        | (Template::BestOf, PageKind::BestOf { .. })
        | (Template::Review, PageKind::Review) => Some(text.to_owned()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use khalid_content::{parse_post, render};
    use khalid_core::SlugMode;

    use super::*;

    fn english() -> Locale {
        Locale::try_new("en").unwrap()
    }

    #[test]
    fn a_comparison_names_both_sides_in_its_table_and_headings() {
        let body = body(Template::Comparison, "Ahrefs vs Semrush", &english()).unwrap();
        assert!(body.contains("| | Ahrefs | Semrush |"), "{body}");
        assert!(body.contains("## Where Semrush wins"), "{body}");
        assert!(!body.contains("{first}"), "{body}");
    }

    #[test]
    fn a_title_of_another_kind_gets_no_starter() {
        assert_eq!(
            body(Template::Comparison, "Ahrefs review", &english()),
            None
        );
        assert_eq!(body(Template::Review, "Why links matter", &english()), None);
    }

    #[test]
    fn other_languages_get_the_english_starter() {
        let french = Locale::try_new("fr").unwrap();
        let body_fr = body(Template::Comparison, "Ghost vs WordPress", &french).unwrap();
        assert!(body_fr.contains("## Where Ghost wins"), "{body_fr}");
    }

    #[test]
    fn no_guidance_shows_on_the_page() {
        let cases = [
            (Template::Comparison, "Ahrefs vs Semrush"),
            (Template::Alternatives, "Ahrefs alternatives"),
            (Template::BestOf, "The best SEO tools"),
            (Template::Review, "Ahrefs review"),
        ];
        for (template, title) in cases {
            let tail = body(template, title, &english()).unwrap();
            let text = format!("---\ntitle: {title}\n{tail}");
            let post = parse_post(Path::new("post.md"), &text).unwrap();
            assert_eq!(post.front.title.as_deref(), Some(title));
            let page = render(&post.body, &english(), SlugMode::Ascii).unwrap();
            assert!(!page.html.contains("never show"), "{}", page.html);
            assert!(!page.html.contains(":::"), "{}", page.html);
        }
    }
}
