//! `theme/tokens.toml` in the shape the renderer loads, with its rules.

use std::fs;

use khalid_theme::{FontStack, Length, ThemeTokens, TokenName};

#[cfg(test)]
fn defaults_file() -> String {
    let path = format!(
        "{}/fixtures/tokens/defaults.toml",
        env!("CARGO_MANIFEST_DIR")
    );
    fs::read_to_string(path).unwrap()
}

#[test]
fn the_defaults_are_the_renderers_byte_for_byte_and_read_back() {
    let tokens = ThemeTokens::default();

    assert_eq!(tokens.to_toml(), defaults_file());
    assert_eq!(ThemeTokens::from_toml(&defaults_file()).unwrap(), tokens);
}

#[test]
fn a_file_with_an_unknown_field_or_a_bad_value_is_refused_with_its_line() {
    let unknown = defaults_file().replace("[shape]", "[shape]\nshadow = \"0 1px 2px\"");
    let bad_colour = defaults_file().replace("text = \"#1f2328\"", "text = \"red\"");
    let alpha = defaults_file().replace("link = \"#0b57d0\"", "link = \"#0b57d0ff\"");
    let bad_font =
        defaults_file().replace("mono = \"ui-monospace", "mono = \"ui-monospace; } body { x");
    let bad_length = defaults_file().replace("\"68ch\"", "\"68vw\"");

    let cases = [
        (unknown, 21),
        (bad_colour, 9),
        (alpha, 11),
        (bad_font, 18),
        (bad_length, 22),
    ];
    for (text, line) in cases {
        let error = ThemeTokens::from_toml(&text).unwrap_err();
        assert_eq!(error.line(), Some(line), "{error}");
    }
}

#[test]
fn a_locked_list_is_read_written_back_above_the_tables_and_checked_by_name() {
    let text = defaults_file().replace(
        "[colors]",
        "locked = [\"colors.link\", \"fonts.heading\"]\n\n[colors]",
    );

    let tokens = ThemeTokens::from_toml(&text).unwrap();

    let names: Vec<&str> = tokens.locked.iter().map(|name| name.as_str()).collect();
    assert_eq!(names, ["colors.link", "fonts.heading"]);
    assert_eq!(TokenName::ALL.len(), 12);
    assert_eq!(tokens.to_toml(), text);
    let wrong = defaults_file().replace("[colors]", "locked = [\"colors.lnk\"]\n\n[colors]");
    let error = ThemeTokens::from_toml(&wrong).unwrap_err();
    assert_eq!(error.line(), Some(6), "{error}");
}

#[test]
fn font_lists_and_lengths_follow_the_renderers_rules() {
    for good in [r#"Inter, "Segoe UI", sans-serif"#, "Söhne, system-ui"] {
        assert!(FontStack::try_from(good.to_owned()).is_ok(), "{good}");
    }
    for bad in [
        "",
        "Inter; color: red",
        r#""Inter, serif"#,
        "Fira\\Code",
        &"A".repeat(301),
    ] {
        assert!(FontStack::try_from(bad.to_owned()).is_err(), "{bad}");
    }
    for good in ["0.375rem", "4px", "68ch", "1em", ".5rem"] {
        assert!(Length::try_from(good.to_owned()).is_ok(), "{good}");
    }
    for bad in ["rem", "4", "1.2.3em", "50%", "calc(1px)", "4 px"] {
        assert!(Length::try_from(bad.to_owned()).is_err(), "{bad}");
    }
}
