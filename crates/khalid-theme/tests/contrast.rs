//! The WCAG contrast pass: a failing colour gets the nearest shade that
//! passes.

use khalid_theme::{ColorToken, ContrastFix, Rgb, ThemeTokens};

#[cfg(test)]
fn hex(text: &str) -> Rgb {
    Rgb::from_css(text).unwrap()
}

/// Asserts every checked token meets its target on both grounds.
#[cfg(test)]
fn assert_all_pass(tokens: &ThemeTokens, site: &str) {
    let colors = tokens.colors;
    let checked = [
        (ColorToken::Text, colors.text),
        (ColorToken::Muted, colors.muted),
        (ColorToken::Link, colors.link),
        (ColorToken::Accent, colors.accent),
    ];
    for (token, color) in checked {
        for ground in [colors.background, colors.surface] {
            let ratio = color.contrast(ground);
            assert!(
                ratio >= token.target(),
                "{site}: {} {} on {} is {ratio:.2}",
                token.name(),
                color.to_hex(),
                ground.to_hex()
            );
        }
    }
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
    assert!(fix.meets_target());
    assert_all_pass(&tokens, "defaults");
    let link = tokens.colors.link;
    assert!(
        fix.after_ratio() < 4.7,
        "the nearest shade, not a much darker one: {}",
        link.to_hex()
    );
    let [red, green, blue] = link.channels();
    assert!(blue > red && blue > green, "still blue: {}", link.to_hex());
}

#[test]
fn light_text_on_a_dark_site_is_made_lighter() {
    let mut tokens = ThemeTokens::default();
    tokens.colors.background = hex("#111111");
    tokens.colors.surface = hex("#1b1b1b");
    tokens.colors.text = hex("#666666");
    tokens.colors.muted = hex("#999999");
    tokens.colors.link = hex("#8ab4f8");
    tokens.colors.accent = hex("#8ab4f8");

    let fixes = tokens.fix_contrast();

    let changed: Vec<ColorToken> = fixes.iter().map(ContrastFix::token).collect();
    assert_eq!(changed, [ColorToken::Text]);
    assert!(tokens.colors.text.luminance() > hex("#666666").luminance());
    assert_all_pass(&tokens, "dark");
}

#[test]
fn the_accent_needs_three_to_one_like_any_mark_that_is_not_text() {
    let mut tokens = ThemeTokens::default();
    tokens.colors.accent = hex("#fde68a");

    let fixes = tokens.fix_contrast();

    assert_eq!(fixes.len(), 1);
    assert_eq!(fixes[0].token(), ColorToken::Accent);
    let accent = tokens.colors.accent;
    assert!(accent.contrast(tokens.colors.surface) >= 3.0);
    assert!(fixes[0].after_ratio() < 3.2, "{}", accent.to_hex());
}

#[test]
fn a_second_pass_changes_nothing() {
    let mut tokens = ThemeTokens::default();
    tokens.colors.text = hex("#9ca3af");
    tokens.colors.link = hex("#fbbf24");
    assert_eq!(tokens.fix_contrast().len(), 2);
    let once = tokens.clone();

    assert_eq!(tokens.fix_contrast(), []);
    assert_eq!(tokens, once);
}

#[test]
fn when_no_shade_can_pass_it_gets_the_one_with_the_most_contrast() {
    let mut tokens = ThemeTokens::default();
    tokens.colors.surface = hex("#222222");
    tokens.colors.text = hex("#777777");

    let fixes = tokens.fix_contrast();

    let fix = fixes
        .iter()
        .find(|fix| fix.token() == ColorToken::Text)
        .unwrap();
    assert!(!fix.meets_target());
    assert!(fix.after_ratio() >= fix.before_ratio(), "{fix:?}");
    assert!(
        fix.after_ratio() > 3.9,
        "the best is about 4 to 1, level with white and #222222: {fix:?}"
    );
}
