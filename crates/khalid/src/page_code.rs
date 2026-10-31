//! The site's own code from `[code]`, a post's head code and the consent
//! banner, as each page of one render carries them (G45, KH-SEO-12, ADR
//! "consent is a form and a cookie").

use khalid_core::{Config, HeadCode, Locale};
use khalid_render::Renderer;
use khalid_server::{
    ASK_END, ASK_START, CHOSEN_END, CHOSEN_START, HELD_END, HELD_START, NONCE_SLOT, SITE_BODY_SLOT,
    SITE_HEAD_SLOT,
};

use crate::post_lock::escaped;
use crate::site_page::HeadNonce;

/// The folder under the mount the consent form posts to. Slugs have no
/// `_`, so no post can take this address.
pub(crate) const CONSENT_FOLDER: &str = "_consent";

/// How the code reaches the pages of one render.
#[derive(Debug, Clone)]
pub(crate) struct PageCode {
    place: Place,
    /// Where the consent form posts, such as `/blog/_consent/`.
    action: String,
    /// The site's head and body code as each page carries it, when it does
    /// not wait for consent.
    site: (String, String),
}

/// Where the code goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Place {
    /// Into every page, with the nonce slot or, on a static host, without.
    Open(HeadNonce),
    /// Into the marked places of every page, which the server fills only
    /// for a reader who agreed, with the banner before the body ends.
    Held,
    /// Nowhere: the code waits for consent, which a static host cannot ask.
    Left,
}

impl PageCode {
    /// The code of the site with `config`, for pages whose head code gets
    /// its nonce as `nonce` says.
    pub(crate) fn new(config: &Config, nonce: HeadNonce) -> Self {
        let code = config.code();
        let place = match nonce {
            _ if !code.waits_for_consent() => Place::Open(nonce),
            HeadNonce::PerRequest => Place::Held,
            HeadNonce::Static => Place::Left,
        };
        let written = |html: Option<String>| html.unwrap_or_default();
        let site = match place {
            Place::Open(HeadNonce::PerRequest) => (
                written(code.head.as_ref().map(|head| head.to_html(NONCE_SLOT))),
                written(code.body.as_ref().map(|body| body.to_html(NONCE_SLOT))),
            ),
            Place::Open(HeadNonce::Static) => (
                written(code.head.as_ref().map(ToString::to_string)),
                written(code.body.as_ref().map(ToString::to_string)),
            ),
            Place::Held | Place::Left => (String::new(), String::new()),
        };
        Self {
            place,
            action: format!("{}{CONSENT_FOLDER}/", config.mount()),
            site,
        }
    }

    /// What goes after a page's SEO head: the post's own head `code`, when
    /// it has some, then the site's head code or its slot. Empty when
    /// neither is there.
    pub(crate) fn head(&self, code: Option<&HeadCode>) -> String {
        let post = code.map_or_else(String::new, |code| match self.place {
            Place::Open(HeadNonce::PerRequest) => code.to_html(NONCE_SLOT),
            Place::Open(HeadNonce::Static) => code.to_string(),
            Place::Held => format!("{HELD_START}{}{HELD_END}", code.to_html(NONCE_SLOT)),
            Place::Left => String::new(),
        });
        let site = match self.place {
            Place::Held => SITE_HEAD_SLOT,
            Place::Open(_) | Place::Left => &self.site.0,
        };
        [post.as_str(), site]
            .into_iter()
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// `html`, a page drawn at `path` in `locale`, with the site's body
    /// code, or its slot and the banner, before its body ends.
    pub(crate) fn with_body_end(
        &self,
        html: String,
        (renderer, locale): (&Renderer, &Locale),
        path: &str,
    ) -> String {
        let end = match self.place {
            Place::Held => self.banner(renderer, locale, path),
            Place::Open(_) => self.site.1.clone(),
            Place::Left => String::new(),
        };
        match html.rfind("</body>") {
            Some(at) if !end.is_empty() => {
                let (before, after) = html.split_at(at);
                format!("{before}{end}\n{after}")
            }
            _ => html,
        }
    }

    /// The body code's slot, the banner and, hidden until a reader has
    /// chosen, the smaller form to change the choice.
    fn banner(&self, renderer: &Renderer, locale: &Locale, path: &str) -> String {
        let words = |key: &str| escaped(renderer.text(locale, key).unwrap_or_default());
        let form = |class: &str, note: &str| {
            format!(
                "<aside class=\"{class}\" aria-label=\"{label}\"><form method=\"post\" action=\"{action}\">\
                 <p>{note}</p><input type=\"hidden\" name=\"back\" value=\"{back}\">\
                 <button type=\"submit\" name=\"choice\" value=\"yes\">{allow}</button>\
                 <button type=\"submit\" name=\"choice\" value=\"no\">{refuse}</button></form></aside>",
                label = words("consent_label"),
                action = escaped(&self.action),
                back = escaped(path),
                allow = words("consent_allow"),
                refuse = words("consent_refuse"),
            )
        };
        format!(
            "{SITE_BODY_SLOT}\n{ASK_START}{}{ASK_END}\n{CHOSEN_START}{}{CHOSEN_END}",
            form("consent", &words("consent_note")),
            form("consent consent-chosen", &words("consent_change")),
        )
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use khalid_core::Environment;

    use super::*;

    /// The demo's settings with `rest` after them.
    fn config(rest: &str) -> Config {
        let text = format!(
            "name = \"Field Notes on SEO\"\naddress = \"https://www.example.com\"\nmount = \"/blog/\"\n\
             default_language = \"en\"\nlanguages = [\"en\", \"de\"]\n{rest}"
        );
        Config::from_toml(Path::new("site.toml"), &text, &Environment::empty()).unwrap()
    }

    const CODE: &str = "[code]\nhead = \"<script src='https://tags.example.com/t.js'></script>\"\nbody = \"<script>chat()</script>\"\n";

    const NOT_NEEDED: &str = "consent = \"not-needed\"\n";

    const PAGE: &str = "<html><head></head><body><p>Text.</p></body></html>";

    fn post_code() -> HeadCode {
        HeadCode::try_new("<script>post()</script>").unwrap()
    }

    fn ending(code: &PageCode, locale: &str) -> String {
        let renderer = Renderer::new().unwrap();
        let locale: Locale = locale.parse().unwrap();
        code.with_body_end(PAGE.to_owned(), (&renderer, &locale), "/blog/a-post/")
    }

    #[test]
    fn a_site_without_code_draws_pages_as_before() {
        let code = PageCode::new(&config(""), HeadNonce::PerRequest);
        assert_eq!(code.head(None), "");
        let post = format!("<script nonce=\"{NONCE_SLOT}\">post()</script>");
        assert_eq!(code.head(Some(&post_code())), post);
        assert_eq!(ending(&code, "en"), PAGE);
    }

    #[test]
    fn code_that_waits_is_marked_and_the_banner_ends_the_body() {
        let code = PageCode::new(&config(CODE), HeadNonce::PerRequest);
        let post = format!("<script nonce=\"{NONCE_SLOT}\">post()</script>");
        assert_eq!(
            code.head(Some(&post_code())),
            format!("{HELD_START}{post}{HELD_END}\n{SITE_HEAD_SLOT}")
        );
        let html = ending(&code, "en");
        assert!(!html.contains("chat()"), "{html}");
        let (body, end) = html.split_once(SITE_BODY_SLOT).unwrap();
        assert!(body.ends_with("<p>Text.</p>"), "{html}");
        assert!(end.ends_with("</body></html>"), "{html}");
        let (ask, chosen) = end.split_once(ASK_END).unwrap();
        for part in [
            "<form method=\"post\" action=\"/blog/_consent/\">",
            "name=\"back\" value=\"/blog/a-post/\"",
            "It runs only if you allow it.",
            "value=\"yes\">Allow</button>",
            "value=\"no\">Do not allow</button>",
        ] {
            assert!(ask.contains(part), "{part} in {ask}");
        }
        assert!(chosen.contains(CHOSEN_START), "{chosen}");
        assert!(chosen.contains("You can change your choice"), "{chosen}");
        assert!(ending(&code, "de").contains("Erlauben</button>"));
    }

    #[test]
    fn code_that_needs_no_consent_goes_into_every_page() {
        let settings = config(&format!("{CODE}{NOT_NEEDED}"));
        let code = PageCode::new(&settings, HeadNonce::PerRequest);
        let tag = format!(
            "<script src=\"https://tags.example.com/t.js\" nonce=\"{NONCE_SLOT}\"></script>"
        );
        assert_eq!(code.head(None), tag);
        let chat = format!("<script nonce=\"{NONCE_SLOT}\">chat()</script>");
        let page = format!("<html><head></head><body><p>Text.</p>{chat}\n</body></html>");
        assert_eq!(ending(&code, "en"), page);
        let built = PageCode::new(&settings, HeadNonce::Static);
        assert_eq!(
            built.head(Some(&post_code())),
            "<script>post()</script>\n<script src=\"https://tags.example.com/t.js\"></script>"
        );
    }

    #[test]
    fn a_build_leaves_out_code_that_waits() {
        let code = PageCode::new(&config(CODE), HeadNonce::Static);
        assert_eq!(code.head(Some(&post_code())), "");
        assert_eq!(ending(&code, "en"), PAGE);
    }
}
