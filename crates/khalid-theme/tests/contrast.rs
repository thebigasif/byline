//! The WCAG contrast pass: a failing colour gets the nearest shade that
//! passes, and the user is told in plain words.

use khalid_theme::{ColorToken, Rgb, ThemeTokens};

#[cfg(test)]
fn hex(text: &str) -> Rgb {
    Rgb::from_css(text).unwrap()
}

#[test]
fn the_defaults_pass_and_nothing_changes() {
    let mut tokens = ThemeTokens::default();

    assert_eq!(tokens.fix_contrast(), []);
    assert_eq!(tokens, ThemeTokens::default());
}

#[test]
fn a_faint_link_gets_the_nearest_shade_that_passes_on_background_and_surface() {
    let mut tokens = ThemeTokens::default();
    tokens.colors.link = hex("#60a5fa");

    let fixes = tokens.fix_contrast();

    assert_eq!(fixes.len(), 1);
    let fix = &fixes[0];
    assert_eq!(fix.token(), ColorToken::Link);
    assert_eq!(fix.before(), hex("#60a5fa"));
    assert_eq!(fix.after(), tokens.colors.link);
    let link = tokens.colors.link;
    for ground in [tokens.colors.background, tokens.colors.surface] {
        assert!(link.contrast(ground) >= 4.5, "{} on {}", link.to_hex(), ground.to_hex());
    }
    assert!(
        link.contrast(tokens.colors.surface) < 4.7,
        "the nearest shade, not a much darker one: {}",
        link.to_hex()
    );
    let [red, green, blue] = link.channels();
    assert!(blue > red && blue > green, "still blue: {}", link.to_hex());
}

#[test]
fn light_text_on_a_dark_background_is_made_lighter() {
    let mut tokens = ThemeTokens::default();
    tokens.colors.background = hex("#111111");
    tokens.colors.surface = hex("#1b1b1b");
    tokens.colors.text = hex("#666666");
    tokens.colors.muted = hex("#999999");
    tokens.colors.link = hex("#8ab4f8");
    tokens.colors.accent = hex("#8ab4f8");

    let fixes = tokens.fix_contrast();

    let fixed: Vec<ColorToken> = fixes.iter().map(|fix| fix.token()).collect();
    assert_eq!(fixed, [ColorToken::Text, ColorToken::Muted]);
    assert!(tokens.colors.text.luminance() > hex("#666666").luminance());
    assert!(tokens.colors.text.contrast(tokens.colors.surface) >= 4.5);
}

#[test]
fn the_accent_needs_three_to_one_like_any_mark_that_is_not_text() {
    let mut tokens = ThemeTokens::default();
    tokens.colors.accent = hex("#fde68a");

    let fixes = tokens.fix_contrast();

    assert_eq!(fixes.len(), 1);
    assert_eq!(fixes[0].token(), ColorToken::Accent);
    let accent = tokens.colors.accent;
    assert!(accent.contrast(tokens.colors.background) >= 3.0);
    assert!(accent.contrast(tokens.colors.background) < 3.3, "{}", accent.to_hex());
}

#[test]
fn each_fix_is_told_in_plain_words() {
    let mut tokens = ThemeTokens::default();
    tokens.colors.muted = hex("#a0a0a0");

    let fixes = tokens.fix_contrast();
    let told = fixes[0].to_string();

    let after = tokens.colors.muted.to_hex();
    assert_eq!(
        told,
        format!(
            "Quiet text such as dates was #a0a0a0, at 2.6 to 1 on the page, \
             too faint to read easily. It is now {after}, at 4.5 to 1, the closest \
             shade that meets WCAG AA."
        )
    );
}
