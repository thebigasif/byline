//! The body and heading fonts, and where each font file comes from.

use khalid_theme::{FontHost, FontSource, Typography};

#[cfg(test)]
fn source(family: &str, url: &str, host: FontHost) -> FontSource {
    FontSource::new(family, url, host)
}

#[test]
fn finds_body_and_heading_families_from_rules_and_properties() {
    let fonts = Typography::from_css(
        &[":root { --font-sans: 'Inter', system-ui, sans-serif; }\n\
           body { font-family: var(--font-sans) }\n\
           h1, h2, h3 { font: 700 2rem/1.2 'Playfair Display', Georgia, serif }"],
        &[],
    );

    assert_eq!(
        fonts.body.as_deref(),
        Some(r#""Inter", system-ui, sans-serif"#)
    );
    assert_eq!(
        fonts.heading.as_deref(),
        Some(r#""Playfair Display", Georgia, serif"#)
    );
}

#[test]
fn named_properties_are_used_when_no_rule_says() {
    let fonts = Typography::from_css(
        &[
            ":root { --bs-body-font-family: Lato, sans-serif; --font-heading: Merriweather, serif;\
           --font-mono: Menlo, monospace }",
        ],
        &[],
    );

    assert_eq!(fonts.body.as_deref(), Some("Lato, sans-serif"));
    assert_eq!(fonts.heading.as_deref(), Some("Merriweather, serif"));
}

#[test]
fn lists_where_each_font_comes_from() {
    let fonts = Typography::from_css(
        &[
            "@import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;700&family=Source+Serif+4&display=swap');\n\
             @font-face { font-family: \"Acme Sans\"; src: url(/fonts/acme.woff2) format(\"woff2\"), url(/fonts/acme.woff); }",
            "@font-face { font-family: proxima-nova; src: url(\"https://use.typekit.net/af/1a2b3c/00000000/l?subset_id=2\") }",
        ],
        &["https://fonts.googleapis.com/css?family=Roboto+Slab:400,700|Open+Sans"],
    );

    let google = "https://fonts.googleapis.com/css2?family=Inter:wght@400;700&family=Source+Serif+4&display=swap";
    let roboto = "https://fonts.googleapis.com/css?family=Roboto+Slab:400,700|Open+Sans";
    assert_eq!(
        fonts.sources,
        [
            source("Inter", google, FontHost::Google),
            source("Source Serif 4", google, FontHost::Google),
            source("Acme Sans", "/fonts/acme.woff2", FontHost::SelfHosted),
            source(
                "proxima-nova",
                "https://use.typekit.net/af/1a2b3c/00000000/l?subset_id=2",
                FontHost::Adobe
            ),
            source("Roboto Slab", roboto, FontHost::Google),
            source("Open Sans", roboto, FontHost::Google),
        ]
    );
    assert!(FontHost::Google.can_rehost() && FontHost::SelfHosted.can_rehost());
    assert!(!FontHost::Adobe.can_rehost() && !FontHost::Other.can_rehost());
}

#[test]
fn a_site_that_names_no_font_finds_nothing() {
    let fonts = Typography::from_css(&["p { margin: 0 }"], &["https://cdn.example.net/x.css"]);

    assert_eq!(fonts, Typography::default());
}
