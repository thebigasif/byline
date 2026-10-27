//! Syncing again: values named in `locked` keep their hand edits, the
//! rest follow the site, and the diff says what changed in plain words.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use common::{sync, sync_edited};
use khalid_core::AbsoluteUrl;
use khalid_theme::{Rgb, ThemeTokens, TokenChange, TokenName};

const STYLESHEET: &str = "/wp-content/themes/northwind/style.css";

#[cfg(test)]
fn hex(text: &str) -> Rgb {
    Rgb::from_css(text).unwrap()
}

#[cfg(test)]
fn tokens_file(site: &Path) -> PathBuf {
    let mut path = site.to_path_buf();
    path.push("theme/tokens.toml");
    path
}

#[tokio::test]
async fn a_locked_hand_edit_is_kept_and_the_rest_follows_the_changed_site() {
    let site = tempfile::tempdir().unwrap();
    let (first, _) = sync("bakery", STYLESHEET).await;
    ThemeTokens::from_theme(&first).write(site.path()).unwrap();
    let path = tokens_file(site.path());
    let edited = fs::read_to_string(&path)
        .unwrap()
        .replace("link = \"#9c3d1c\"", "link = \"#7a2e12\"")
        .replace("[colors]", "locked = [\"colors.link\"]\n\n[colors]");
    fs::write(&path, edited).unwrap();
    let (changed, _) = sync_edited("bakery", STYLESHEET, |css| {
        css.replace("#9c3d1c", "#1d4ed8")
            .replace("#2b2118", "#1b1b1f")
            .replace("\"Lora\"", "\"Merriweather\"")
    })
    .await;
    let from_site = ThemeTokens::from_theme(&changed);

    let mut tokens = ThemeTokens::read(site.path()).unwrap().unwrap();
    let diff = tokens.resync(&from_site);
    tokens.write(site.path()).unwrap();

    let mut expected = from_site;
    expected.colors.link = hex("#7a2e12");
    expected.locked = vec![TokenName::Link];
    assert_eq!(ThemeTokens::read(site.path()).unwrap(), Some(expected));
    let said = diff.to_string();
    for line in [
        "colors.text changed from #2b2118 to #1b1b1f.",
        "colors.link stays #7a2e12, because it is locked. The site now uses #1d4ed8.",
        "fonts.body changed from \"Lora\", serif to \"Merriweather\", serif.",
    ] {
        assert!(said.lines().any(|l| l == line), "{line} in\n{said}");
    }
    assert!(!said.contains("colors.background"), "{said}");
}

#[test]
fn an_unlocked_hand_edit_follows_the_site_and_the_diff_says_so() {
    let mut tokens = ThemeTokens::default();
    tokens.colors.muted = hex("#444444");

    let diff = tokens.resync(&ThemeTokens::default());

    assert_eq!(tokens, ThemeTokens::default());
    let changes = diff.changes();
    assert!(
        matches!(
            changes,
            [TokenChange::Updated { name: TokenName::Muted, before, after }]
                if before == "#444444" && after == "#57606a"
        ),
        "{changes:?}"
    );
}

#[test]
fn a_site_that_did_not_change_gives_an_empty_diff_even_with_locked_values() {
    let mut tokens = ThemeTokens::default();
    tokens.locked = vec![TokenName::Heading, TokenName::Radius];

    let diff = tokens.resync(&ThemeTokens::default());

    assert!(diff.is_empty(), "{diff:?}");
    assert_eq!(diff.to_string(), "Nothing changed since the last sync.\n");
    assert_eq!(tokens.locked, [TokenName::Heading, TokenName::Radius]);
}

#[test]
fn a_logo_found_for_the_first_time_is_added_and_the_diff_says_so() {
    let mut site = ThemeTokens::default();
    site.brand.logo = Some(AbsoluteUrl::parse("https://a.example/logo.svg").unwrap());
    let mut tokens = ThemeTokens::default();

    let diff = tokens.resync(&site);

    assert_eq!(tokens, site);
    assert_eq!(
        diff.to_string(),
        "brand.logo changed from none to https://a.example/logo.svg.\n"
    );
}

#[test]
fn every_token_can_be_locked() {
    let mut text = ThemeTokens::default().to_toml();
    for (from, to) in [
        ("#ffffff", "#fbf8f3"),
        ("#f6f7f9", "#f3efea"),
        ("#1f2328", "#2b2118"),
        ("#57606a", "#746c65"),
        ("link = \"#0b57d0", "link = \"#9c3d1c"),
        ("accent = \"#0b57d0", "accent = \"#e4572e"),
        ("#d0d7de", "#d6d1cc"),
        ("body = 'system-ui", "body = 'Lora, system-ui"),
        ("heading = 'system-ui", "heading = 'Fraunces, system-ui"),
        ("mono = \"ui-monospace", "mono = \"Menlo, ui-monospace"),
        ("0.375rem", "2px"),
        ("68ch", "40rem"),
    ] {
        text = text.replace(from, to);
    }
    text.push_str("\n[brand]\nlogo = \"https://a.example/logo.svg\"\n");
    text.push_str("icon = \"https://a.example/icon.png\"\n");
    let site = ThemeTokens::from_toml(&text).unwrap();
    let mut tokens = ThemeTokens::default();
    tokens.locked = TokenName::ALL.to_vec();

    let diff = tokens.resync(&site);

    let kept: Vec<TokenName> = diff
        .changes()
        .iter()
        .filter_map(|change| match change {
            TokenChange::Kept { name, .. } => Some(*name),
            _ => None,
        })
        .collect();
    assert_eq!(kept, TokenName::ALL);
    let mut unchanged = ThemeTokens::default();
    unchanged.locked = TokenName::ALL.to_vec();
    assert_eq!(tokens, unchanged);
}
