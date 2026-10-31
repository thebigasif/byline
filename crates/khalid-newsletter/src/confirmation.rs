//! The confirmation email: what it says in each language, and the links in
//! it.

use std::collections::BTreeMap;

use khalid_core::{AbsoluteUrl, Locale};
use serde::Deserialize;

use crate::pages::PageWords;
use crate::{EmailAddress, Mail, PostalAddress, SubscriberToken};

/// The language whose words fill in for a language with no file.
const FALLBACK_LANGUAGE: &str = "en";

/// The built-in words files, by language (RULES.md BL-TXT-3).
const FILES: [(&str, &str); 2] = [
    ("en", include_str!("../strings/en.toml")),
    ("de", include_str!("../strings/de.toml")),
];

/// One language's words: the confirmation email, where `{site}`,
/// `{confirm}` and `{remove}` stand for the site's name and the two links,
/// and the pages.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Words {
    confirm_subject: String,
    confirm_text: String,
    digest_subject: String,
    digest_text: String,
    footer_text: String,
    page: PageWords,
}

/// The words of the emails and pages in every built-in language.
#[derive(Debug, Clone)]
pub struct NewsletterWords {
    words: BTreeMap<&'static str, Words>,
}

impl NewsletterWords {
    /// Reads every built-in words file.
    ///
    /// # Errors
    ///
    /// [`WordsError`] when a file is not valid, which this crate's tests
    /// rule out.
    pub fn load() -> Result<Self, WordsError> {
        let mut words = BTreeMap::new();
        for (language, text) in FILES {
            let read = toml::from_str(text).map_err(|source| WordsError {
                language,
                source: Box::new(source),
            })?;
            words.insert(language, read);
        }
        Ok(Self { words })
    }

    /// The confirmation email to `to` from the site called `site`, in
    /// `locale` or in English when the language has no file. It holds the
    /// link that confirms and the link that removes the address at once.
    pub fn confirmation(
        &self,
        locale: &Locale,
        site: &str,
        to: EmailAddress,
        links: ConfirmationLinks<'_>,
    ) -> Mail {
        let words = self.of(locale);
        let fill = |text: &str| {
            text.replace("{confirm}", &links.confirm())
                .replace("{remove}", &links.remove())
                .replace("{site}", site)
        };
        let (subject, text) = words.map_or_else(Default::default, |words| {
            (fill(&words.confirm_subject), fill(&words.confirm_text))
        });
        Mail::new(to, subject, text)
    }

    /// The subject and the first line of the email of new posts from the
    /// site called `site`, in `locale`.
    pub fn digest(&self, locale: &Locale, site: &str) -> (String, String) {
        self.of(locale).map_or_else(Default::default, |words| {
            (
                words.digest_subject.replace("{site}", site),
                words.digest_text.replace("{site}", site),
            )
        })
    }

    /// The end of every newsletter email from the site called `site`, in
    /// `locale`: why the reader gets it, the `remove` link that
    /// unsubscribes them in one click, and the site's `postal` address.
    pub fn footer(
        &self,
        locale: &Locale,
        site: &str,
        remove: &str,
        postal: &PostalAddress,
    ) -> String {
        self.of(locale).map_or_else(String::new, |words| {
            words
                .footer_text
                .replace("{remove}", remove)
                .replace("{postal}", postal.as_str())
                .replace("{site}", site)
        })
    }

    /// The words of the pages in `locale`, or in English when the language
    /// has no file.
    pub(crate) fn page(&self, locale: &Locale) -> Option<&PageWords> {
        self.of(locale).map(|words| &words.page)
    }

    fn of(&self, locale: &Locale) -> Option<&Words> {
        self.words
            .get(locale.language())
            .or_else(|| self.words.get(FALLBACK_LANGUAGE))
    }
}

/// Where the site's newsletter pages answer: the site's address with its
/// mount path, such as `https://www.example.com/blog/`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SiteLinks {
    base: String,
    mount: String,
}

impl SiteLinks {
    /// The pages of the site at `address` mounted under `mount`, which
    /// starts and ends with `/` as `Config` checks.
    pub fn new(address: &AbsoluteUrl, mount: &str) -> Self {
        let mut base = address.as_str().trim_end_matches('/').to_owned();
        base.push_str(mount);
        Self {
            base,
            mount: mount.to_owned(),
        }
    }

    /// The mount path alone, such as `/blog/`, for links between the pages.
    pub fn mount(&self) -> &str {
        &self.mount
    }

    /// The page a confirmation link opens, with `token`.
    pub fn confirm(&self, token: &SubscriberToken) -> String {
        format!(
            "{}subscribe/confirm?token={}",
            self.base,
            token.expose_text()
        )
    }

    /// The page an unsubscribe link opens, with `token`.
    pub fn unsubscribe(&self, token: &SubscriberToken) -> String {
        format!("{}unsubscribe?token={}", self.base, token.expose_text())
    }

    /// The full address the sign-up form posts to, for a form on another
    /// site.
    pub fn subscribe(&self) -> String {
        format!("{}subscribe", self.base)
    }
}

/// The two links in a confirmation email.
#[derive(Debug, Clone, Copy)]
pub struct ConfirmationLinks<'a> {
    /// The site's pages.
    pub site: &'a SiteLinks,
    /// The token that confirms.
    pub confirm: &'a SubscriberToken,
    /// The unsubscribe token that removes the address instead.
    pub remove: &'a SubscriberToken,
}

impl ConfirmationLinks<'_> {
    fn confirm(&self) -> String {
        self.site.confirm(self.confirm)
    }

    fn remove(&self) -> String {
        self.site.unsubscribe(self.remove)
    }
}

/// A built-in words file is not valid.
#[derive(Debug, thiserror::Error)]
#[error("the {language} words of the newsletter emails cannot be read")]
pub struct WordsError {
    language: &'static str,
    #[source]
    source: Box<toml::de::Error>,
}
