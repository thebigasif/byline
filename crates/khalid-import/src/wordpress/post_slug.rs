//! A Khalid slug from a `WordPress` slug, kept whenever Khalid's rules allow.

use std::borrow::Cow;

use khalid_core::{Locale, ParseSlugError, Slug, SlugMode};
use percent_encoding::percent_decode_str;

/// A post's slug and how it relates to the slug it had.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PostSlug {
    /// The slug the post has in Khalid.
    pub(crate) slug: Slug,
    /// Whether that is the slug it had.
    pub(crate) origin: SlugOrigin,
}

/// How a post's slug relates to the one it had in `WordPress`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SlugOrigin {
    /// The same slug, so the address keeps its last part.
    Kept,
    /// A new slug, because the old one breaks the site's slug rules, such as
    /// `über-uns` on a site with ASCII slugs. The old address must redirect.
    Changed {
        /// The old slug, decoded.
        was: String,
    },
    /// A new slug for a post that had none, such as an unsaved draft.
    Made,
}

/// The slug for an item whose `WordPress` slug is `name`, with `title` and
/// `id` to fall back on, on a site written in `locale` with slugs in `mode`.
///
/// The error is the one `post-<id>` would give as a slug, which never
/// happens; it is returned rather than assumed away.
pub(crate) fn post_slug(
    name: &str,
    title: &str,
    id: u64,
    locale: &Locale,
    mode: SlugMode,
) -> Result<PostSlug, ParseSlugError> {
    // WordPress writes letters outside ASCII percent-encoded, as in
    // `%c3%bcber-uns`.
    let decoded = percent_decode_str(name)
        .decode_utf8()
        .map_or_else(|_| name.to_owned(), Cow::into_owned);
    if !decoded.is_empty() {
        let same = match mode {
            SlugMode::Unicode => Slug::try_new_unicode(&decoded),
            SlugMode::Ascii => Slug::try_new(&decoded),
        };
        if let Ok(slug) = same {
            return Ok(PostSlug {
                slug,
                origin: SlugOrigin::Kept,
            });
        }
        if let Ok(slug) = Slug::from_title(&decoded, locale, mode) {
            let origin = SlugOrigin::Changed { was: decoded };
            return Ok(PostSlug { slug, origin });
        }
    }
    let slug =
        Slug::from_title(title, locale, mode).or_else(|_| Slug::try_new(&format!("post-{id}")))?;
    let origin = if decoded.is_empty() {
        SlugOrigin::Made
    } else {
        SlugOrigin::Changed { was: decoded }
    };
    Ok(PostSlug { slug, origin })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slug(name: &str, title: &str, language: &str, mode: SlugMode) -> (String, SlugOrigin) {
        let locale = Locale::try_new(language).unwrap();
        let PostSlug { slug, origin } = post_slug(name, title, 7, &locale, mode).unwrap();
        (slug.as_str().to_owned(), origin)
    }

    #[test]
    fn a_slug_that_fits_the_rules_is_kept() {
        assert_eq!(
            slug("keep-a-starter-alive", "Anything", "en", SlugMode::Ascii),
            ("keep-a-starter-alive".to_owned(), SlugOrigin::Kept)
        );
        assert_eq!(
            slug("%c3%bcber-uns", "Über uns", "de", SlugMode::Unicode),
            ("über-uns".to_owned(), SlugOrigin::Kept)
        );
    }

    #[test]
    fn a_slug_that_breaks_the_rules_is_made_again_and_remembered() {
        let changed = |was: &str| SlugOrigin::Changed {
            was: was.to_owned(),
        };
        assert_eq!(
            slug("%c3%bcber-uns", "Über uns", "de", SlugMode::Ascii),
            ("ueber-uns".to_owned(), changed("über-uns"))
        );
        assert_eq!(
            slug("Rye_Bread", "Rye bread", "en", SlugMode::Ascii),
            ("rye-bread".to_owned(), changed("Rye_Bread"))
        );
    }

    #[test]
    fn a_post_with_no_slug_gets_one_from_its_title_or_id() {
        assert_eq!(
            slug("", "Rye: a first loaf", "en", SlugMode::Ascii),
            ("rye-a-first-loaf".to_owned(), SlugOrigin::Made)
        );
        assert_eq!(
            slug("", "", "en", SlugMode::Ascii),
            ("post-7".to_owned(), SlugOrigin::Made)
        );
        assert_eq!(
            slug("", "!!!", "en", SlugMode::Ascii),
            ("post-7".to_owned(), SlugOrigin::Made)
        );
    }
}
