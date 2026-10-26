//! Tokens built from a synced theme, and written atomically inside the
//! site folder.

use std::fs;
use std::path::{Path, PathBuf};

use khalid_core::SitePathError;
use khalid_theme::{Palette, Rgb, SiteTheme, ThemeTokens, TokensError, Typography};

#[cfg(test)]
fn hex(text: &str) -> Rgb {
    Rgb::from_css(text).unwrap()
}

/// `root` with a trusted `relative` path pushed on, for building expectations.
#[cfg(test)]
fn under(root: &Path, relative: &str) -> PathBuf {
    let mut path = root.to_path_buf();
    path.push(relative);
    path
}

#[cfg(test)]
fn theme(palette: Palette, body: Option<&str>, heading: Option<&str>) -> SiteTheme {
    let mut typography = Typography::default();
    typography.body = body.map(str::to_owned);
    typography.heading = heading.map(str::to_owned);
    let mut theme = SiteTheme::default();
    theme.palette = palette;
    theme.typography = typography;
    theme
}

#[test]
fn a_site_that_gave_nothing_gets_the_defaults() {
    assert_eq!(
        ThemeTokens::from_theme(&SiteTheme::default()),
        ThemeTokens::default()
    );
}

#[test]
fn found_colours_and_fonts_replace_the_defaults_and_fill_the_rest() {
    let mut palette = Palette::default();
    palette.background = Some(hex("#fbf8f3"));
    palette.text = Some(hex("#2b2118"));
    palette.link = Some(hex("#9c3d1c"));

    let text = ThemeTokens::from_theme(&theme(palette, Some(r#""Lora", serif"#), None)).to_toml();

    for line in [
        "background = \"#fbf8f3\"",
        "text = \"#2b2118\"",
        "link = \"#9c3d1c\"",
        "accent = \"#9c3d1c\"",
        "surface = \"#f3efea\"",
        "border = \"#d6d1cc\"",
        "muted = \"#746c65\"",
        "body = '\"Lora\", serif'",
        "heading = '\"Lora\", serif'",
    ] {
        assert!(text.lines().any(|l| l == line), "{line} in\n{text}");
    }
}

#[test]
fn font_names_that_could_break_out_of_css_are_dropped_and_a_fallback_is_added() {
    let found = theme(
        Palette::default(),
        Some(r#"Inter, "x;}body{display:none", Söhne"#),
        Some(r#""Fraunces""#),
    );

    let text = ThemeTokens::from_theme(&found).to_toml();

    assert!(
        text.contains("body = \"Inter, Söhne, sans-serif\"\n"),
        "{text}"
    );
    assert!(
        text.contains("heading = '\"Fraunces\", sans-serif'\n"),
        "{text}"
    );
}

#[test]
fn writing_creates_the_theme_folder_replaces_the_file_and_leaves_no_temporary_file() {
    let site = tempfile::tempdir().unwrap();
    fs::create_dir(under(site.path(), "theme")).unwrap();
    fs::write(under(site.path(), "theme/tokens.toml"), "old").unwrap();
    let tokens = ThemeTokens::default();

    let written = tokens.write(site.path()).unwrap();

    assert_eq!(fs::read_to_string(&written).unwrap(), tokens.to_toml());
    let names: Vec<_> = fs::read_dir(under(site.path(), "theme"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(names, ["tokens.toml"]);
    let fresh = tempfile::tempdir().unwrap();
    tokens.write(fresh.path()).unwrap();
    assert!(under(fresh.path(), "theme/tokens.toml").is_file());
}

#[test]
fn reading_gives_nothing_for_a_new_site_and_the_written_tokens_after_a_write() {
    let site = tempfile::tempdir().unwrap();
    assert_eq!(ThemeTokens::read(site.path()).unwrap(), None);

    let mut palette = Palette::default();
    palette.accent = Some(hex("#e4572e"));
    let tokens = ThemeTokens::from_theme(&theme(palette, None, None));
    tokens.write(site.path()).unwrap();
    assert_eq!(ThemeTokens::read(site.path()).unwrap(), Some(tokens));

    fs::write(
        under(site.path(), "theme/tokens.toml"),
        "[colors]\ntext = 1\n",
    )
    .unwrap();
    let error = ThemeTokens::read(site.path()).unwrap_err();
    assert_eq!(error.line(), Some(2), "{error}");
}

#[cfg(unix)]
#[test]
fn a_theme_folder_that_links_outside_the_site_is_refused() {
    let site = tempfile::tempdir().unwrap();
    let elsewhere = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(elsewhere.path(), under(site.path(), "theme")).unwrap();

    let error = ThemeTokens::default().write(site.path()).unwrap_err();

    let escapes = matches!(
        &error,
        TokensError::Path {
            source: SitePathError::EscapesRoot { .. }
        }
    );
    assert!(escapes, "{error:?}");
    assert_eq!(fs::read_dir(elsewhere.path()).unwrap().count(), 0);
}
