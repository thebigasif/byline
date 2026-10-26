//! hreflang: the links between the language versions of one page.

use std::collections::BTreeMap;

use khalid_core::{AbsoluteUrl, Indexing, Locale};

use crate::{SeoError, SeoPage, SeoSite};

/// The name a post's language versions share, from the `translation` key in
/// their front matter, such as `internal-links`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TranslationKey(String);

impl TranslationKey {
    /// Checks a key and wraps it, without the spaces around it.
    ///
    /// # Errors
    ///
    /// [`SeoError::TranslationKey`] when the key is empty or only spaces.
    pub fn try_new(text: &str) -> Result<Self, SeoError> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Err(SeoError::TranslationKey {
                key: text.to_owned(),
            });
        }
        Ok(Self(trimmed.to_owned()))
    }

    /// The key as written.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Which readers an alternate version is for.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Hreflang {
    /// Readers of this language.
    Language(Locale),
    /// Readers whose language has no version of its own: `x-default`.
    Default,
}

impl Hreflang {
    /// The `hreflang` attribute value.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Language(language) => language.as_str(),
            Self::Default => "x-default",
        }
    }
}

/// One version of a page, for one `<link rel="alternate" hreflang>` tag.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Alternate {
    /// Which readers it is for.
    pub hreflang: Hreflang,
    /// Its address.
    pub url: AbsoluteUrl,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Group {
    Key(TranslationKey),
    Alone(usize),
}

/// Fills `alternates` on every page from the pages' translation keys.
///
/// Every version in a group gets the same list: each language in the order
/// the site declares them, then `x-default`, which is the default language's
/// version or, without one, the first language present. A page with no key
/// is a group of one. A page that is `noindex` or names another page as its
/// canonical is left out, since search engines ignore hreflang there. A site
/// with one language gets no alternates at all.
///
/// # Errors
///
/// [`SeoError::TranslationLanguage`] when two pages in one group share a
/// language, and [`SeoError::Url`] when an address cannot be built.
pub fn link_translations(site: &SeoSite, pages: &mut [SeoPage]) -> Result<(), SeoError> {
    let mut groups: BTreeMap<Group, Vec<usize>> = BTreeMap::new();
    for (index, page) in pages.iter_mut().enumerate() {
        page.alternates.clear();
        let eligible = site.languages.len() > 1
            && page.canonical.is_none()
            && site.indexing.directives_for(page).indexing == Indexing::Index;
        if eligible {
            let group = page
                .translation
                .clone()
                .map_or(Group::Alone(index), Group::Key);
            groups.entry(group).or_default().push(index);
        }
    }
    for (group, members) in groups {
        let alternates = group_alternates(site, pages, &group, &members)?;
        for index in members {
            if let Some(page) = pages.get_mut(index) {
                page.alternates.clone_from(&alternates);
            }
        }
    }
    Ok(())
}

fn group_alternates(
    site: &SeoSite,
    pages: &[SeoPage],
    group: &Group,
    members: &[usize],
) -> Result<Vec<Alternate>, SeoError> {
    let rank = |language: &Locale| {
        site.languages
            .iter()
            .position(|declared| declared == language)
            .unwrap_or(site.languages.len())
    };
    let mut versions: Vec<&SeoPage> = members.iter().filter_map(|&i| pages.get(i)).collect();
    versions
        .sort_by(|a, b| (rank(&a.language), &a.language).cmp(&(rank(&b.language), &b.language)));
    let duplicate = versions.windows(2).find_map(|pair| match pair {
        [a, b] if a.language == b.language => Some(a.language.as_str().to_owned()),
        _ => None,
    });
    if let (Group::Key(key), Some(language)) = (group, duplicate) {
        return Err(SeoError::TranslationLanguage {
            key: key.as_str().to_owned(),
            language,
        });
    }
    let mut alternates = Vec::with_capacity(versions.len() + 1);
    for page in &versions {
        alternates.push(Alternate {
            hreflang: Hreflang::Language(page.language.clone()),
            url: site.url_for(&page.language, &page.path)?,
        });
    }
    let fallback = versions
        .iter()
        .find(|page| page.language == site.default_language)
        .or(versions.first());
    if let Some(page) = fallback {
        alternates.push(Alternate {
            hreflang: Hreflang::Default,
            url: site.url_for(&page.language, &page.path)?,
        });
    }
    Ok(alternates)
}
