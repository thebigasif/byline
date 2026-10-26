//! Title and description templates with variables, such as `{title} {sep} {site}`.

use std::fmt;
use std::str::FromStr;

use crate::{SeoError, SeoPage, SeoSite};

/// A variable a template may use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Variable {
    Title,
    Site,
    Tagline,
    Sep,
    Excerpt,
    Category,
    Author,
}

impl Variable {
    const ALL: [Self; 7] = [
        Self::Title,
        Self::Site,
        Self::Tagline,
        Self::Sep,
        Self::Excerpt,
        Self::Category,
        Self::Author,
    ];

    fn name(self) -> &'static str {
        match self {
            Self::Title => "title",
            Self::Site => "site",
            Self::Tagline => "tagline",
            Self::Sep => "sep",
            Self::Excerpt => "excerpt",
            Self::Category => "category",
            Self::Author => "author",
        }
    }

    fn value<'a>(self, site: &'a SeoSite, page: &'a SeoPage) -> &'a str {
        match self {
            Self::Title => &page.title,
            Self::Site => &site.name,
            Self::Tagline => &site.tagline,
            Self::Sep => &site.separator,
            Self::Excerpt => &page.excerpt,
            Self::Category => page.category.as_deref().unwrap_or_default(),
            Self::Author => page.author.as_deref().unwrap_or_default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Piece {
    Text(String),
    Var(Variable),
}

/// A title or description template: text with variables in braces.
///
/// The variables are `{title}`, `{site}`, `{tagline}`, `{sep}`, `{excerpt}`,
/// `{category}` and `{author}`. A variable with no value renders as nothing,
/// and the result loses doubled spaces and any separator left at either end.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetaTemplate {
    pieces: Vec<Piece>,
    source: String,
}

impl MetaTemplate {
    /// Parses a template.
    ///
    /// # Errors
    ///
    /// [`SeoError::TemplateVariable`] for an unknown name in braces, and
    /// [`SeoError::TemplateBrace`] for a brace without its partner.
    pub fn parse(text: &str) -> Result<Self, SeoError> {
        let brace = || SeoError::TemplateBrace {
            template: text.to_owned(),
        };
        let mut pieces = Vec::new();
        let mut rest = text;
        while let Some(open) = rest.find(['{', '}']) {
            let (before, after) = rest.split_at(open);
            let after = after.strip_prefix('{').ok_or_else(brace)?;
            let (name, tail) = after.split_once('}').ok_or_else(brace)?;
            if name.contains('{') {
                return Err(brace());
            }
            let variable = Variable::ALL
                .into_iter()
                .find(|variable| variable.name() == name)
                .ok_or_else(|| SeoError::TemplateVariable {
                    template: text.to_owned(),
                    name: name.to_owned(),
                })?;
            if !before.is_empty() {
                pieces.push(Piece::Text(before.to_owned()));
            }
            pieces.push(Piece::Var(variable));
            rest = tail;
        }
        if !rest.is_empty() {
            pieces.push(Piece::Text(rest.to_owned()));
        }
        Ok(Self {
            pieces,
            source: text.to_owned(),
        })
    }

    /// The template as written.
    pub fn as_str(&self) -> &str {
        &self.source
    }

    /// Fills the variables for `page` on `site` and tidies the result: one
    /// space between words, and no separator at either end.
    pub fn render(&self, site: &SeoSite, page: &SeoPage) -> String {
        let mut raw = String::new();
        for piece in &self.pieces {
            match piece {
                Piece::Text(text) => raw.push_str(text),
                Piece::Var(variable) => raw.push_str(variable.value(site, page)),
            }
        }
        let words: Vec<&str> = raw.split_whitespace().collect();
        let separator = site.separator.trim();
        let start = words.iter().position(|word| *word != separator);
        let end = words.iter().rposition(|word| *word != separator);
        match (start, end) {
            (Some(start), Some(end)) => words.get(start..=end).unwrap_or_default().join(" "),
            _ => String::new(),
        }
    }
}

impl FromStr for MetaTemplate {
    type Err = SeoError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::parse(text)
    }
}

impl fmt::Display for MetaTemplate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.source)
    }
}
