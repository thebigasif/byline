//! The body and heading fonts, and where each font file comes from.

use khalid_theme::{FontHost, FontSource, Typography};

#[cfg(test)]
fn source(family: &str, url: &str, host: FontHost) -> FontSource {
    FontSource::new(family, url, host)
}

#[test]
fn only_text_fonts_from_an_unknown_server_are_held_and_inline_fonts_never_are() {
    let css = [
        "@font-face { font-family: InterVariable; src: url(https://static.gnome.org/Inter.woff2) }",
        "@font-face { font-family: Pythonicon; src: url(https://cdn.example.org/icons.woff) }",
        "@font-face { font-family: Brand; src: url(data:font/woff2;base64,d09GMgAB) }",
        "body { font-family: InterVariable, sans-serif }",
        "h1 { font-family: Brand, serif }",
    ]
    .join("\n");

    let fonts = Typography::from_css(&[&css], &[]);

    let held: Vec<(&str, &str)> = fonts
        .held
        .iter()
        .map(|font| (font.family(), font.server()))
        .collect();
    assert_eq!(held, [("InterVariable", "static.gnome.org")]);
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

#[tokio::test]
async fn rehostable_fonts_are_downloaded_and_returned_as_files() {
    use khalid_theme::download as fonts_download;
    use std::net::{IpAddr, Ipv4Addr};

    let server = wiremock::MockServer::start().await;
    let port = server.address().port();
    let loopback = vec![IpAddr::V4(Ipv4Addr::LOCALHOST)];
    let hosts = [
        ("fonts.googleapis.test".to_owned(), loopback.clone()),
        ("fonts.gstatic.test".to_owned(), loopback),
    ];
    let resolver = khalid_outbound::Resolver::for_tests(
        hosts.into(),
        khalid_outbound::Reach::PublicAndLoopback,
    );
    let outbound =
        khalid_outbound::Outbound::with_resolver(khalid_outbound::Limits::default(), resolver)
            .unwrap();

    let css = format!(
        "/* latin */\n@font-face {{ font-family: 'Inter'; font-style: normal; font-weight: 400; src: url(http://fonts.gstatic.test:{port}/inter-latin.woff2) format('woff2'); unicode-range: U+0000-00FF; }}\n"
    );
    wiremock::Mock::given(wiremock::matchers::path("/css2"))
        .respond_with(wiremock::ResponseTemplate::new(200).set_body_string(css))
        .mount(&server)
        .await;
    wiremock::Mock::given(wiremock::matchers::path("/inter-latin.woff2"))
        .respond_with(wiremock::ResponseTemplate::new(200).set_body_bytes(b"font bytes".as_slice()))
        .mount(&server)
        .await;

    let source = source(
        "Inter",
        &format!("http://fonts.googleapis.test:{port}/css2?family=Inter"),
        FontHost::Google,
    );
    let into = tempfile::tempdir().unwrap();
    let files = fonts_download(&outbound, std::slice::from_ref(&source), into.path())
        .await
        .unwrap();
    assert_eq!(files.len(), 1, "{files:?}");
    assert_eq!(files[0].family, "Inter");
    assert_eq!(files[0].path, "inter-latin.woff2");
    assert_eq!(
        files[0].range.as_deref(),
        Some("U+0000-00FF"),
        "the subset the file covers is kept"
    );
    let saved = std::fs::read(within(into.path(), "inter-latin.woff2")).unwrap();
    assert_eq!(saved, b"font bytes");
}

#[tokio::test]
async fn a_font_that_cannot_be_read_is_skipped_not_fatal() {
    use khalid_theme::download as fonts_download;
    use std::net::{IpAddr, Ipv4Addr};

    let server = wiremock::MockServer::start().await;
    let port = server.address().port();
    let loopback = vec![IpAddr::V4(Ipv4Addr::LOCALHOST)];
    let hosts = [("fonts.googleapis.test".to_owned(), loopback)];
    let resolver = khalid_outbound::Resolver::for_tests(
        hosts.into(),
        khalid_outbound::Reach::PublicAndLoopback,
    );
    let outbound =
        khalid_outbound::Outbound::with_resolver(khalid_outbound::Limits::default(), resolver)
            .unwrap();

    wiremock::Mock::given(wiremock::matchers::path("/css2"))
        .respond_with(wiremock::ResponseTemplate::new(500))
        .mount(&server)
        .await;

    let source = source(
        "Inter",
        &format!("http://fonts.googleapis.test:{port}/css2?family=Inter"),
        FontHost::Google,
    );
    let into = tempfile::tempdir().unwrap();
    let files = fonts_download(&outbound, std::slice::from_ref(&source), into.path())
        .await
        .unwrap();
    assert!(
        files.is_empty(),
        "a failed download leaves the site working"
    );
}

/// `name` inside `dir`.
#[cfg(test)]
fn within(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
    let mut path = dir.to_path_buf();
    path.push(name);
    path
}

#[tokio::test]
async fn a_font_over_the_cap_is_skipped_in_plain_words() {
    use khalid_theme::{MAX_FONT_BYTES, download_and_report as download};
    use std::net::{IpAddr, Ipv4Addr};

    let server = wiremock::MockServer::start().await;
    let port = server.address().port();
    let loopback = vec![IpAddr::V4(Ipv4Addr::LOCALHOST)];
    let hosts = [
        ("fonts.googleapis.com".to_owned(), loopback.clone()),
        ("fonts.gstatic.com".to_owned(), loopback),
    ];
    let resolver = khalid_outbound::Resolver::for_tests(
        hosts.into(),
        khalid_outbound::Reach::PublicAndLoopback,
    );
    let outbound =
        khalid_outbound::Outbound::with_resolver(khalid_outbound::Limits::default(), resolver)
            .unwrap();

    let css = format!(
        "/* latin */\n@font-face {{ font-family: 'Inter'; font-style: normal; font-weight: 400; src: url(http://fonts.gstatic.com:{port}/huge.woff2) format('woff2'); }}\n"
    );
    wiremock::Mock::given(wiremock::matchers::path("/css2"))
        .respond_with(wiremock::ResponseTemplate::new(200).set_body_string(css))
        .mount(&server)
        .await;
    // One byte over the cap: the file is never read further.
    let huge = vec![0u8; MAX_FONT_BYTES + 1];
    wiremock::Mock::given(wiremock::matchers::path("/huge.woff2"))
        .respond_with(wiremock::ResponseTemplate::new(200).set_body_bytes(huge))
        .mount(&server)
        .await;

    let source = source(
        "Inter",
        &format!("http://fonts.googleapis.com:{port}/css2?family=Inter"),
        FontHost::Google,
    );
    let into = tempfile::tempdir().unwrap();
    let (saved, skipped) = download(&outbound, std::slice::from_ref(&source), into.path())
        .await
        .unwrap();
    assert!(saved.is_empty(), "an oversized font is never served");
    assert_eq!(skipped.len(), 1, "{skipped:?}");
    assert!(
        skipped[0].contains("over the 4096 KB a font may be"),
        "the reason is plain words: {}",
        skipped[0]
    );
    assert!(
        !within(into.path(), "inter-latin.woff2").exists(),
        "nothing was written"
    );
}

#[tokio::test]
async fn a_stylesheet_that_answers_an_error_is_skipped_in_plain_words() {
    use khalid_theme::download_and_report as download;
    use std::net::{IpAddr, Ipv4Addr};

    let server = wiremock::MockServer::start().await;
    let port = server.address().port();
    let loopback = vec![IpAddr::V4(Ipv4Addr::LOCALHOST)];
    let hosts = [("fonts.googleapis.com".to_owned(), loopback)];
    let resolver = khalid_outbound::Resolver::for_tests(
        hosts.into(),
        khalid_outbound::Reach::PublicAndLoopback,
    );
    let outbound =
        khalid_outbound::Outbound::with_resolver(khalid_outbound::Limits::default(), resolver)
            .unwrap();

    wiremock::Mock::given(wiremock::matchers::path("/css2"))
        .respond_with(wiremock::ResponseTemplate::new(500))
        .mount(&server)
        .await;

    let source = source(
        "Inter",
        &format!("http://fonts.googleapis.com:{port}/css2?family=Inter"),
        FontHost::Google,
    );
    let into = tempfile::tempdir().unwrap();
    let (saved, skipped) = download(&outbound, std::slice::from_ref(&source), into.path())
        .await
        .unwrap();
    assert!(
        saved.is_empty(),
        "a failed stylesheet serves nothing: {saved:?}"
    );
    assert_eq!(skipped.len(), 1, "{skipped:?}");
    assert!(
        skipped[0].contains("answered 500"),
        "the reason names the status: {}",
        skipped[0]
    );
}
