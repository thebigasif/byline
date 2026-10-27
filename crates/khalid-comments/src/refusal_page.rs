//! The page a reader sees when a comment cannot be sent, in their language.

use std::collections::BTreeMap;
use std::fmt::Write;

use khalid_core::Locale;
use serde::Deserialize;

use crate::{CommentForm, FieldProblem};

/// The language whose words fill in for a language with no file.
const FALLBACK_LANGUAGE: &str = "en";

/// The built-in words, by language.
const FILES: [(&str, &str); 2] = [
    ("en", include_str!("../strings/en.toml")),
    ("de", include_str!("../strings/de.toml")),
];

/// Why a comment was not sent, as the reader is told.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Refusal {
    /// Fields to fix, in the order the form shows them.
    Fields(Vec<FieldProblem>),
    /// The comment answered is no longer shown.
    ParentGone,
    /// The address sent too many comments; it may try again in `minutes`.
    TooMany {
        /// Whole minutes until a comment is allowed again, at least one.
        minutes: u64,
    },
    /// The page the form was sent from takes no comments.
    NoPost,
    /// Something on the server failed; the reader may try again later.
    Unavailable,
}

/// One language's words, one sentence or label each.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Words {
    title: String,
    name_missing: String,
    name_too_long: String,
    email_invalid: String,
    text_missing: String,
    text_too_long: String,
    parent_gone: String,
    too_many: String,
    no_post: String,
    unavailable: String,
    name: String,
    email: String,
    comment: String,
    honeypot: String,
    send: String,
    back: String,
}

/// The words in every built-in language, ready to write refusal pages.
#[derive(Debug, Clone)]
pub struct RefusalPages {
    words: BTreeMap<&'static str, Words>,
}

/// A built-in words file is not valid, which this crate's tests rule out.
#[derive(Debug, thiserror::Error)]
#[error("the comment words for {language:?} are not valid")]
pub struct LoadWordsError {
    language: &'static str,
    #[source]
    source: toml::de::Error,
}

impl RefusalPages {
    /// Reads every built-in words file.
    ///
    /// # Errors
    ///
    /// [`LoadWordsError`] when a file is not valid.
    pub fn load() -> Result<Self, LoadWordsError> {
        let mut words = BTreeMap::new();
        for (language, text) in FILES {
            let read =
                toml::from_str(text).map_err(|source| LoadWordsError { language, source })?;
            words.insert(language, read);
        }
        Ok(Self { words })
    }

    /// A whole HTML page in `locale`, linking the site's `stylesheet`,
    /// saying why the comment in `form` was not sent. Unless the page
    /// takes no comments, it shows the form again, filled in as sent and
    /// posting to `action`, so nothing typed is lost and no script is
    /// needed.
    pub fn page(
        &self,
        locale: &Locale,
        refusal: &Refusal,
        form: &CommentForm,
        (action, stylesheet): (&str, &str),
    ) -> String {
        let words = self
            .words
            .get(locale.language())
            .or_else(|| self.words.get(FALLBACK_LANGUAGE));
        let Some(words) = words else {
            return String::new();
        };
        let mut html = format!(
            "<!doctype html>\n<html lang=\"{lang}\">\n<head>\n<meta charset=\"utf-8\">\n\
             <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
             <meta name=\"robots\" content=\"noindex\">\n<title>{title}</title>\n\
             <link rel=\"stylesheet\" href=\"{stylesheet}\">\n</head>\n\
             <body>\n<main class=\"comment-refused\">\n<h1>{title}</h1>\n<ul>\n",
            lang = escape(locale.as_str()),
            title = escape(&words.title),
            stylesheet = escape(stylesheet),
        );
        for sentence in sentences(words, refusal) {
            let _ = writeln!(html, "<li>{}</li>", escape(&sentence));
        }
        html.push_str("</ul>\n");
        if *refusal != Refusal::NoPost {
            let parent_gone = match refusal {
                Refusal::ParentGone => true,
                Refusal::Fields(problems) => problems.contains(&FieldProblem::ParentInvalid),
                _ => false,
            };
            let parent = form.parent.as_deref().filter(|_| !parent_gone);
            html.push_str(&form_again(words, form, parent, action));
            let _ = writeln!(
                html,
                "<p><a href=\"{}\">{}</a></p>",
                escape(&form.page),
                escape(&words.back)
            );
        }
        html.push_str("</main>\n</body>\n</html>\n");
        html
    }
}

/// The sentences that say what `refusal` means, in `words`' language.
fn sentences(words: &Words, refusal: &Refusal) -> Vec<String> {
    match refusal {
        Refusal::Fields(problems) => problems
            .iter()
            .map(|problem| match problem {
                FieldProblem::NameMissing => words.name_missing.clone(),
                FieldProblem::NameTooLong => words.name_too_long.clone(),
                FieldProblem::EmailInvalid => words.email_invalid.clone(),
                FieldProblem::TextMissing => words.text_missing.clone(),
                FieldProblem::TextTooLong => words.text_too_long.clone(),
                // The parent field is never shown, so only a changed page
                // sends a bad one, and the comment can go on its own.
                _ => words.parent_gone.clone(),
            })
            .collect(),
        Refusal::ParentGone => vec![words.parent_gone.clone()],
        Refusal::TooMany { minutes } => {
            vec![words.too_many.replace("{minutes}", &minutes.to_string())]
        }
        Refusal::NoPost => vec![words.no_post.clone()],
        Refusal::Unavailable => vec![words.unavailable.clone()],
    }
}

/// The comment form, filled in with what `form` held, answering `parent`
/// when it still can.
fn form_again(words: &Words, form: &CommentForm, parent: Option<&str>, action: &str) -> String {
    let parent = parent.map_or_else(String::new, |id| {
        format!(
            "<input type=\"hidden\" name=\"parent\" value=\"{}\">\n",
            escape(id)
        )
    });
    format!(
        "<form class=\"comment-form\" method=\"post\" action=\"{action}\">\n\
         <input type=\"hidden\" name=\"page\" value=\"{page}\">\n{parent}\
         <p><label for=\"comment-name\">{name_label}</label>\n\
         <input id=\"comment-name\" name=\"name\" type=\"text\" required maxlength=\"80\" autocomplete=\"name\" value=\"{name}\"></p>\n\
         <p><label for=\"comment-email\">{email_label}</label>\n\
         <input id=\"comment-email\" name=\"email\" type=\"email\" autocomplete=\"email\" value=\"{email}\"></p>\n\
         <p class=\"comment-trap\" aria-hidden=\"true\"><label for=\"comment-website\">{trap}</label>\n\
         <input id=\"comment-website\" name=\"website\" type=\"text\" tabindex=\"-1\" autocomplete=\"off\"></p>\n\
         <p><label for=\"comment-text\">{text_label}</label>\n\
         <textarea id=\"comment-text\" name=\"comment\" required maxlength=\"5000\" rows=\"6\">{text}</textarea></p>\n\
         <p><button type=\"submit\">{send}</button></p>\n</form>\n",
        action = escape(action),
        page = escape(&form.page),
        name_label = escape(&words.name),
        name = escape(&form.name),
        email_label = escape(&words.email),
        email = escape(&form.email),
        trap = escape(&words.honeypot),
        text_label = escape(&words.comment),
        text = escape(&form.comment),
        send = escape(&words.send),
    )
}

/// `text` safe between HTML tags and inside a quoted attribute.
fn escape(text: &str) -> String {
    let mut safe = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => safe.push_str("&amp;"),
            '<' => safe.push_str("&lt;"),
            '>' => safe.push_str("&gt;"),
            '"' => safe.push_str("&quot;"),
            '\'' => safe.push_str("&#39;"),
            c => safe.push(c),
        }
    }
    safe
}
