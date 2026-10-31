//! `[code]` in `site.toml`: the site's own head and body code, and whether
//! it waits for a reader's consent.

use std::path::Path;

use khalid_core::{BodyCode, BodyCodeError, Config, Consent, Environment, HeadCodeError};

const SITE: &str = r#"name = "Field Notes on SEO"
address = "https://www.example.com"
default_language = "en"
languages = ["en"]
"#;

const CODE: &str = r#"
[code]
head = """
<script async src="https://www.googletagmanager.com/gtag/js?id=G-1"></script>
<script>gtag('config', 'G-1');</script>"""
body = "<script src='https://chat.example.com/widget.js' defer></script>"
"#;

#[cfg(test)]
fn load(text: &str) -> Result<Config, String> {
    Config::from_toml(Path::new("site.toml"), text, &Environment::empty())
        .map_err(|error| error.to_string())
}

#[test]
fn a_site_without_the_table_has_no_code() {
    let config = load(SITE).unwrap();
    let code = config.code();
    assert!(code.is_empty());
    assert_eq!(code.consent, Consent::Ask);
    assert!(!code.waits_for_consent());
}

#[test]
fn reads_head_and_body_code_that_waits_for_consent_by_default() {
    let config = load(&format!("{SITE}{CODE}")).unwrap();
    let code = config.code();
    assert!(!code.is_empty());
    assert_eq!(code.consent, Consent::Ask);
    assert!(code.waits_for_consent());
    let head = code.head.as_ref().unwrap();
    assert_eq!(
        head.to_html("n"),
        "<script async src=\"https://www.googletagmanager.com/gtag/js?id=G-1\" nonce=\"n\"></script>\n<script nonce=\"n\">gtag('config', 'G-1');</script>"
    );
    let body = code.body.as_ref().unwrap();
    assert_eq!(
        body.to_html("n"),
        "<script src=\"https://chat.example.com/widget.js\" defer nonce=\"n\"></script>"
    );
}

#[test]
fn code_may_run_without_asking() {
    let text = format!("{SITE}{CODE}consent = \"not-needed\"\n");
    let config = load(&text).unwrap();
    assert_eq!(config.code().consent, Consent::NotNeeded);
    assert!(!config.code().waits_for_consent());
}

#[test]
fn a_table_of_comments_only_is_empty_and_asks_nothing() {
    let config = load(&format!("{SITE}[code]\nhead = \"<!-- later -->\"\n")).unwrap();
    assert!(config.code().is_empty());
    assert!(!config.code().waits_for_consent());
}

#[test]
fn body_code_holds_scripts_only() {
    for (html, name) in [
        ("<style>p{color:red}</style>", "style"),
        ("<meta name=a content=b>", "meta"),
        ("<div>hi</div>", "div"),
    ] {
        assert_eq!(
            BodyCode::try_new(html),
            Err(BodyCodeError::Element {
                name: name.to_owned()
            }),
            "{html}"
        );
    }
    let open = BodyCode::try_new("<script>go()").unwrap_err();
    assert_eq!(
        open,
        BodyCodeError::Code(HeadCodeError::Unclosed { name: "script" })
    );
    assert_eq!(
        open.to_string(),
        "body code has a script that is never closed"
    );
    let error = load(&format!("{SITE}[code]\nbody = \"<div>hi</div>\"\n")).unwrap_err();
    assert!(
        error.contains("field \"body\"") && error.contains("body code may hold only script"),
        "{error}"
    );
}

#[test]
fn a_static_site_cannot_ask_for_consent() {
    let text = format!("{SITE}mode = \"static\"\n{CODE}");
    let error = load(&text).unwrap_err();
    assert!(
        error.contains("field \"consent\"") && error.contains("consent = \"not-needed\""),
        "{error}"
    );
    let text = format!("{text}consent = \"not-needed\"\n");
    assert_eq!(load(&text).unwrap().code().consent, Consent::NotNeeded);
}

#[test]
fn refuses_an_unknown_consent() {
    let error = load(&format!("{SITE}[code]\nconsent = \"maybe\"\n")).unwrap_err();
    assert!(error.contains("field \"consent\""), "{error}");
}
