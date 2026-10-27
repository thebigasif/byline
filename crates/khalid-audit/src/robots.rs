//! What a page's robots tags and headers ask of search engines.

/// What the `robots` meta tags or `X-Robots-Tag` headers of a page ask of
/// search engines.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Robots {
    noindex: bool,
    nofollow: bool,
}

impl Robots {
    /// Whether a robots tag says `noindex` or `none`.
    pub fn noindex(self) -> bool {
        self.noindex
    }

    /// Whether a robots tag says `nofollow` or `none`.
    pub fn nofollow(self) -> bool {
        self.nofollow
    }

    /// The same, with the words in `content` added, as a tag or a header
    /// spells them: `noindex, follow`.
    #[must_use]
    pub fn with(self, content: &str) -> Self {
        let words = || content.split(',').map(str::trim);
        let has = |word: &str| words().any(|said| said.eq_ignore_ascii_case(word));
        Self {
            noindex: self.noindex || has("noindex") || has("none"),
            nofollow: self.nofollow || has("nofollow") || has("none"),
        }
    }
}
