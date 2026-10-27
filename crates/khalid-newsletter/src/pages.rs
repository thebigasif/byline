//! The newsletter's pages: plain HTML with one heading, one sentence and at
//! most one form, so every step works with scripts off (plan decision 12).

use serde::Deserialize;

use crate::{NewsletterSite, NewsletterWords, WordsError};

/// One language's words for the pages. `{site}` stands for the site's name.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PageWords {
    back: String,
    email_label: String,
    sign_up_button: String,
    check_title: String,
    check_text: String,
    wrong_email_title: String,
    wrong_email_text: String,
    confirm_title: String,
    confirm_text: String,
    confirm_button: String,
    confirmed_title: String,
    confirmed_text: String,
    expired_title: String,
    expired_text: String,
    unknown_title: String,
    unknown_text: String,
    leave_title: String,
    leave_text: String,
    leave_button: String,
    left_title: String,
    left_text: String,
    closed_title: String,
    closed_text: String,
    failed_title: String,
    failed_text: String,
}

/// What a page tells the reader.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Page<'a> {
    /// After a sign-up, whatever happened: look for the email.
    Check,
    /// The address typed cannot receive mail; the form again.
    WrongEmail,
    /// A confirmation link was opened: one button confirms, with this
    /// token. Opening a link never changes anything by itself, since mail
    /// programs open links to scan them.
    Confirm(&'a str),
    /// The subscriber is confirmed.
    Confirmed,
    /// The confirmation link is too old; the form again.
    Expired,
    /// The link was never sent, cut short, or used up.
    Unknown,
    /// An unsubscribe link was opened: one button leaves, with this token.
    Leave(&'a str),
    /// The subscriber gets no more email.
    Left,
    /// The site cannot send email, so it takes no sign-ups.
    Closed,
    /// Something failed on the server; nothing was sent.
    Failed,
}

/// Writes the pages of one site, in its language.
#[derive(Debug, Clone)]
pub struct Pages {
    site: NewsletterSite,
    words: NewsletterWords,
}

impl Pages {
    /// The pages of `site`.
    ///
    /// # Errors
    ///
    /// [`WordsError`] when a built-in words file is not valid, which this
    /// crate's tests rule out.
    pub fn new(site: NewsletterSite) -> Result<Self, WordsError> {
        Ok(Self {
            site,
            words: NewsletterWords::load()?,
        })
    }

    /// The mount path the pages answer under, such as `/blog/`.
    pub fn mount(&self) -> &str {
        self.site.links.mount()
    }

    /// `page` as a whole HTML document.
    pub fn html(&self, page: Page<'_>) -> String {
        let Some(w) = self.words.page(&self.site.locale) else {
            return String::new();
        };
        let mount = self.site.links.mount();
        let sign_up = || sign_up_form(w, mount);
        let (title, text, form) = match page {
            Page::Check => (&w.check_title, &w.check_text, String::new()),
            Page::WrongEmail => (&w.wrong_email_title, &w.wrong_email_text, sign_up()),
            Page::Confirm(token) => {
                let form = button_form(
                    &format!("{mount}subscribe/confirm"),
                    token,
                    &w.confirm_button,
                );
                (&w.confirm_title, &w.confirm_text, form)
            }
            Page::Confirmed => (&w.confirmed_title, &w.confirmed_text, String::new()),
            Page::Expired => (&w.expired_title, &w.expired_text, sign_up()),
            Page::Unknown => (&w.unknown_title, &w.unknown_text, sign_up()),
            Page::Leave(token) => {
                let form = button_form(&format!("{mount}unsubscribe"), token, &w.leave_button);
                (&w.leave_title, &w.leave_text, form)
            }
            Page::Left => (&w.left_title, &w.left_text, String::new()),
            Page::Closed => (&w.closed_title, &w.closed_text, String::new()),
            Page::Failed => (&w.failed_title, &w.failed_text, String::new()),
        };
        let site = &self.site.name;
        let (title, text) = (filled(title, site), filled(text, site));
        let lang = escape(self.site.locale.as_str());
        let (back, site, mount) = (filled(&w.back, site), escape(site), escape(mount));
        format!(
            "<!doctype html>\n<html lang=\"{lang}\">\n<head>\n<meta charset=\"utf-8\">\n\
             <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
             <meta name=\"robots\" content=\"noindex\">\n<title>{title} | {site}</title>\n\
             </head>\n<body>\n<main>\n<h1>{title}</h1>\n<p>{text}</p>\n{form}\
             <p><a href=\"{mount}\">{back}</a></p>\n</main>\n</body>\n</html>\n"
        )
    }
}

/// The sign-up form, posting to the site's own sign-up address.
fn sign_up_form(w: &PageWords, mount: &str) -> String {
    format!(
        "<form method=\"post\" action=\"{}subscribe\">\n<p><label for=\"email\">{}</label>\n\
         <input id=\"email\" name=\"email\" type=\"email\" autocomplete=\"email\" required></p>\n\
         <p><button type=\"submit\">{}</button></p>\n</form>\n",
        escape(mount),
        escape(&w.email_label),
        escape(&w.sign_up_button)
    )
}

/// A form of one button that posts `token` to `action`.
fn button_form(action: &str, token: &str, button: &str) -> String {
    format!(
        "<form method=\"post\" action=\"{}\">\n<input type=\"hidden\" name=\"token\" value=\"{}\">\n\
         <p><button type=\"submit\">{}</button></p>\n</form>\n",
        escape(action),
        escape(token),
        escape(button)
    )
}

/// `text` with the site's name in place of `{site}`, safe between tags.
fn filled(text: &str, site: &str) -> String {
    escape(&text.replace("{site}", site))
}

/// `text` safe between tags and inside a quoted attribute.
fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
