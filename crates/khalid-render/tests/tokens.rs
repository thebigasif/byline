//! `theme/tokens.toml`: typed values, light defaults, located mistakes, CSS
//! variables and the WCAG contrast check.

use std::fs;
use std::path::PathBuf;

use khalid_render::{RenderError, Tokens};

#[test]
fn a_site_without_tokens_gets_light_defaults_that_pass_contrast() {
    let site = tempfile::tempdir().unwrap();
    let tokens = Tokens::load(site.path()).unwrap();
    assert_eq!(tokens, Tokens::default());
    assert_eq!(tokens.colors.background.as_str(), "#ffffff");
    assert!(
        tokens.contrast_failures().is_empty(),
        "{:?}",
        tokens.contrast_failures()
    );
}

#[test]
fn tokens_become_css_variables() {
    let tokens = Tokens::from_toml(
        r##"
[colors]
text = "#123"
link = "#0A58CA"

[fonts]
body = 'Inter, "Segoe UI", sans-serif'

[shape]
radius = "4px"
"##,
    )
    .unwrap();
    let css = tokens.to_css();
    assert!(css.starts_with(":root {\n"));
    assert!(
        css.contains("  --color-text: #112233;\n"),
        "#rgb becomes #rrggbb"
    );
    assert!(
        css.contains("  --color-link: #0a58ca;\n"),
        "hex is lowercased"
    );
    assert!(css.contains(r#"  --font-body: Inter, "Segoe UI", sans-serif;"#));
    assert!(css.contains("  --radius: 4px;\n"));
    assert!(
        css.contains("  --color-background: #ffffff;\n"),
        "unset tokens keep defaults"
    );
}

#[test]
fn a_value_that_could_break_out_of_css_is_refused_with_its_line() {
    let text = "[colors]\ntext = \"#000\"\n\n[fonts]\nbody = \"Inter; } body { display: none\"\n";
    let error = Tokens::from_toml(text).unwrap_err();
    assert!(
        matches!(error, RenderError::Tokens { line: Some(5), .. }),
        "{error:?}"
    );
    assert_eq!(
        error.to_string(),
        "theme/tokens.toml has a mistake on line 5"
    );
}

#[test]
fn bad_values_and_unknown_fields_are_refused() {
    for text in [
        "[colors]\ntext = \"black\"",
        "[shape]\nradius = \"4vw\"",
        "[fonts]\nbody = 'Inter, \"Segoe UI'",
        "[colours]\ntext = \"#000\"",
        "[colors]\nbackgound = \"#fff\"",
    ] {
        assert!(Tokens::from_toml(text).is_err(), "{text}");
    }
}

#[test]
fn a_locked_list_of_token_names_is_read_and_a_wrong_name_is_refused_with_its_line() {
    let tokens = Tokens::from_toml(
        "locked = [\"colors.link\", \"fonts.heading\"]\n\n[colors]\nlink = \"#9c3d1c\"\n",
    )
    .unwrap();
    assert_eq!(tokens.colors.link.as_str(), "#9c3d1c");
    assert_eq!(tokens.locked.len(), 2);

    let error = Tokens::from_toml("\nlocked = [\"colors.lnk\"]\n").unwrap_err();
    assert!(
        matches!(error, RenderError::Tokens { line: Some(2), .. }),
        "{error:?}"
    );
}

#[test]
fn faint_text_is_reported_with_its_contrast() {
    let tokens = Tokens::from_toml("[colors]\nmuted = \"#aaaaaa\"\nlink = \"#777777\"").unwrap();
    let failures = tokens.contrast_failures();
    let names: Vec<&str> = failures.iter().map(|failure| failure.token).collect();
    assert_eq!(names, ["colors.muted", "colors.link"]);
    assert!(
        (failures[0].ratio - 2.32).abs() < 0.01,
        "{}",
        failures[0].ratio
    );
}

#[test]
fn tokens_load_from_the_theme_folder() {
    let site = tempfile::tempdir().unwrap();
    let mut path: PathBuf = site.path().to_path_buf();
    path.push("theme");
    fs::create_dir(&path).unwrap();
    path.push("tokens.toml");
    fs::write(&path, "[colors]\naccent = \"#b3261e\"\n").unwrap();
    let tokens = Tokens::load(site.path()).unwrap();
    assert_eq!(tokens.colors.accent.as_str(), "#b3261e");
}
