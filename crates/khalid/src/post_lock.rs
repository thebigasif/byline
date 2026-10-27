//! Posts a reader may not read yet: those with a password, and those for
//! members. Their public page shows the title, the description and a
//! password form or a note, and never the body, in every output that
//! reads it: the page, its table of contents, the feeds and the schema
//! (task 5.11, ADR "a password page is a plain form and a signed cookie").

use khalid_content::{PostFront, SiteContent};
use khalid_core::{Config, Locale, PasswordHash, PostAccess, RobotsDirectives};
use khalid_render::{BodyHtml, PageBody, ReadingTime, Renderer, Robots};

use crate::build_pages::{LanguagePages, Page};

/// The folder under the mount the password form posts to. Slugs have no
/// `_`, so no post can take this address.
pub(crate) const UNLOCK_FOLDER: &str = "_unlock";

/// What keeps a post's body from a reader.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Lock {
    /// The reader must give the password this hash was made from.
    Password(PasswordHash),
    /// Only signed-in members may read it. Readers cannot sign in yet, so
    /// for now nobody reads it on the public site.
    Members,
}

impl Lock {
    /// The lock `front` asks for, if any. A password comes first: a
    /// members' post with a password opens with the password.
    pub(crate) fn of(front: &PostFront) -> Option<Self> {
        if let Some(hash) = &front.password_hash {
            return Some(Self::Password(hash.clone()));
        }
        match front.access {
            PostAccess::Public => None,
            // Members, and any tier added later, stay locked until
            // readers can sign in.
            _ => Some(Self::Members),
        }
    }
}

/// Locks the page of every locked post of `content` in `languages`, with
/// the words `renderer` has for each language. Without a renderer, as on a
/// headless site, the page body is left empty. Call it before
/// `build_seo::heads`, so feeds and the schema see only the description.
pub(crate) fn lock(
    config: &Config,
    content: &SiteContent,
    renderer: Option<&Renderer>,
    languages: &mut [LanguagePages],
) {
    for language in languages.iter_mut() {
        let locale = language.site.locale.clone();
        for page in &mut language.pages {
            let front = content
                .posts
                .iter()
                .find(|doc| page.source.as_ref() == Some(&doc.path))
                .map(|doc| &doc.front);
            if let Some(lock) = front.and_then(Lock::of) {
                let words = Words {
                    renderer,
                    locale: &locale,
                };
                lock_page(config, page, &lock, &words);
            }
        }
    }
}

/// The words of one language for the locked page.
struct Words<'a> {
    renderer: Option<&'a Renderer>,
    locale: &'a Locale,
}

impl Words<'_> {
    /// The text `key`, escaped, or nothing without a renderer.
    fn get(&self, key: &str) -> String {
        let text = self.renderer.and_then(|r| r.text(self.locale, key));
        escaped(text.unwrap_or_default())
    }
}

/// Puts the description and the form or note in place of `page`'s body.
fn lock_page(config: &Config, page: &mut Page, lock: &Lock, words: &Words<'_>) {
    let PageBody::Post(post) = &mut page.body else {
        return;
    };
    let description = escaped(&page.meta.description);
    let summary = if description.is_empty() {
        String::new()
    } else {
        format!("<p>{description}</p>")
    };
    let rest = match lock {
        Lock::Password(_) => form(config, &page.meta.path, words),
        Lock::Members => format!("<p>{}</p>\n", words.get("locked_members_note")),
    };
    let html = format!("<div class=\"locked\">\n{summary}\n{rest}</div>");
    post.body = BodyHtml::from_sanitised(html);
    post.reading_time = ReadingTime::of(&post.body, words.locale);
    post.contents = None;
    post.comments = None;
    page.seo.content_html = summary;
    page.seo.word_count = None;
    if matches!(lock, Lock::Password(_)) {
        page.meta.robots = Robots::NoIndex;
        page.seo.robots = "noindex, nofollow".parse::<RobotsDirectives>().ok();
    }
}

/// The password form: a plain HTML form that works without scripts. The
/// alert is hidden until the server answers a wrong password.
fn form(config: &Config, path: &str, words: &Words<'_>) -> String {
    let action = format!("{}{UNLOCK_FOLDER}/", config.mount());
    format!(
        "<p>{note}</p>\n\
         <form class=\"unlock\" method=\"post\" action=\"{action}\">\n\
         <input type=\"hidden\" name=\"path\" value=\"{path}\">\n\
         <p class=\"unlock-error\" role=\"alert\" hidden>{wrong}</p>\n\
         <label for=\"unlock-password\">{label}</label>\n\
         <input id=\"unlock-password\" name=\"password\" type=\"password\" required autocomplete=\"current-password\">\n\
         <button type=\"submit\">{button}</button>\n\
         </form>\n",
        note = words.get("locked_password_note"),
        action = escaped(&action),
        path = escaped(path),
        wrong = words.get("locked_password_wrong"),
        label = words.get("locked_password_label"),
        button = words.get("locked_password_button"),
    )
}

/// `text` safe inside HTML text and double-quoted attributes.
fn escaped(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
