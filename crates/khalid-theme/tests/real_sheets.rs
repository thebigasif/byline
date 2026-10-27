//! Stylesheets saved from real sites, with their permissive licences named
//! in `fixtures/real/README.md`, each giving the colours and fonts a person
//! sees on that site in a light browser.

use khalid_theme::{Rgb, SiteTheme, ThemeTokens};

#[cfg(test)]
fn sheet(name: &str) -> SiteTheme {
    let path = format!(
        "{}/fixtures/real/{name}/style.css",
        env!("CARGO_MANIFEST_DIR")
    );
    SiteTheme::from_css(&std::fs::read_to_string(path).unwrap())
}

#[cfg(test)]
fn hex(text: &str) -> Option<Rgb> {
    Rgb::from_css(text)
}

#[test]
fn the_nodejs_docs_give_their_light_variables_and_leave_the_dark_mode_class_out() {
    let theme = sheet("nodejs-api");

    let palette = theme.palette;
    assert_eq!(palette.background, hex("#ffffff"));
    assert_eq!(palette.text, hex("#333333"));
    assert_eq!(palette.link, hex("#3e7a38"));
    assert_eq!(
        theme.typography.body.as_deref(),
        Some(
            r#"Lato, "Lucida Grande", "Lucida Sans Unicode", "Lucida Sans", Verdana, Tahoma, sans-serif"#
        )
    );
    assert_eq!(theme.typography.held, []);
}

#[test]
fn a_coverage_report_that_sets_only_its_text_colour_keeps_the_browsers_white() {
    let theme = sheet("istanbul");

    let palette = theme.palette;
    assert_eq!(palette.background, None, "status colours are not the page");
    assert_eq!(palette.text, hex("#333333"));
    assert_eq!(palette.link, hex("#0074d9"));
    assert_eq!(
        theme.typography.body.as_deref(),
        Some("Helvetica Neue, Helvetica, Arial")
    );
    let tokens = ThemeTokens::from_theme(&theme);
    assert_eq!(tokens.colors.background.to_hex(), "#ffffff");
}

#[test]
fn the_bzip2_manual_gives_its_link_and_font_and_keeps_the_browsers_white_and_black() {
    let theme = sheet("bzip2");

    let palette = theme.palette;
    assert_eq!(palette.link, hex("#336699"));
    assert_eq!(theme.typography.body.as_deref(), Some("sans-serif"));
    let tokens = ThemeTokens::from_theme(&theme);
    assert_eq!(tokens.colors.background.to_hex(), "#ffffff");
    assert_eq!(
        tokens.colors.text.to_hex(),
        "#1f2328",
        "the default, near black"
    );
}
