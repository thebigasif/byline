//! CSS custom properties: the ones a site sets for the whole page, with
//! dark variants left out.

use khalid_theme::CustomProperties;

#[test]
fn reads_page_wide_properties_from_every_stylesheet() {
    let properties = CustomProperties::from_css(&[
        "/* tokens */ :root { --blue-600: #0b57d0; --color-link: var(--blue-600); }\n\
         .card { --card-pad: 1rem; }",
        "@media (min-width: 40em) { html { --font-sans: Inter, sans-serif !important; } }",
    ]);

    assert_eq!(properties.get("--color-link"), Some("var(--blue-600)"));
    assert_eq!(properties.get("--font-sans"), Some("Inter, sans-serif"));
    assert_eq!(properties.get("--card-pad"), None, "not set for the page");
    let names: Vec<&str> = properties.iter().map(|(name, _)| name).collect();
    assert_eq!(
        names,
        ["--blue-600", "--color-link", "--font-sans"],
        "sorted"
    );
}

#[test]
fn a_later_rule_wins_and_dark_variants_are_left_out() {
    let properties = CustomProperties::from_css(&[":root { --bg: #fafafa; --text: #222 }\n\
         :root { --bg: #ffffff }\n\
         :root[data-theme=\"dark\"], .dark { --text: #eee }\n\
         @media (prefers-color-scheme : DARK) { :root { --bg: #111; } }"]);

    assert_eq!(properties.get("--bg"), Some("#ffffff"));
    assert_eq!(properties.get("--text"), Some("#222"));
}

#[test]
fn property_names_keep_their_case() {
    let properties = CustomProperties::from_css(&["body { --Brand: teal; COLOR: red }"]);

    assert_eq!(properties.get("--Brand"), Some("teal"));
    assert_eq!(properties.get("--brand"), None);
}
