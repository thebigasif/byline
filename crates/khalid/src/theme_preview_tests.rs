//! Tests for the theme preview, with the HTML checks every rendered page
//! must pass (the renderer's own tests hold the same list).

use std::collections::BTreeSet;

use scraper::{ElementRef, Html, Selector};

use super::*;
use khalid_theme::{FontStack, Rgb};

fn under(root: &Path, relative: &str) -> PathBuf {
    let mut path = root.to_owned();
    for segment in relative.split('/') {
        path.push(segment);
    }
    path
}

/// Tokens a sync of a bakery's site finds, with a font name in quotes.
fn bakery() -> ThemeTokens {
    let mut tokens = ThemeTokens::default();
    tokens.colors.background = Rgb::new(0xfb, 0xf8, 0xf3);
    tokens.colors.text = Rgb::new(0x2b, 0x21, 0x18);
    tokens.colors.link = Rgb::new(0x9c, 0x3d, 0x1c);
    tokens.fonts.body = FontStack::try_from(r#""Lora", serif"#.to_owned()).unwrap();
    tokens
}

fn preview(name: Option<&str>) -> Preview {
    let url = AbsoluteUrl::parse("https://northwind.example/").unwrap();
    let locale = Locale::try_new("en").unwrap();
    let source = Source {
        name,
        url: &url,
        locale: &locale,
    };
    Preview::render(&Renderer::new().unwrap(), &bakery(), source).unwrap()
}

#[test]
fn a_synced_file_loads_with_the_renderers_rules() {
    let site = tempfile::tempdir().unwrap();
    bakery().write(site.path()).unwrap();

    let loaded = Tokens::load(site.path()).unwrap();

    assert_eq!(loaded.colors.background.as_str(), "#fbf8f3");
    assert_eq!(loaded.colors.link.as_str(), "#9c3d1c");
    assert_eq!(loaded.fonts.body.as_str(), r#""Lora", serif"#);
}

#[test]
fn the_preview_passes_the_html_checks() {
    assert_valid_page(&preview(Some("Northwind Bakery")).html);
}

#[test]
fn the_preview_names_the_site_links_back_and_is_never_indexed() {
    let html = preview(Some("Northwind Bakery")).html;

    for expected in [
        "How Northwind Bakery will look",
        r#"href="site.css""#,
        r#"href="https://northwind.example/""#,
        "noindex",
    ] {
        assert!(html.contains(expected), "{expected} in {html}");
    }
    let unnamed = preview(None).html;
    assert!(
        unnamed.contains("How northwind.example will look"),
        "{unnamed}"
    );
}

#[test]
fn the_stylesheet_carries_the_synced_tokens() {
    let css = String::from_utf8(preview(None).css).unwrap();

    assert!(css.contains("--color-background: #fbf8f3;"), "{css}");
    assert!(css.contains("--color-link: #9c3d1c;"), "{css}");
    assert!(css.contains(r#"--font-body: "Lora", serif;"#), "{css}");
}

#[test]
fn the_preview_is_written_under_data_and_replaces_the_last_one() {
    let site = tempfile::tempdir().unwrap();
    preview(None).write(site.path()).unwrap();
    let preview = preview(Some("Northwind Bakery"));

    let page = preview.write(site.path()).unwrap();

    assert!(page.ends_with("data/theme-preview/index.html"), "{page:?}");
    assert_eq!(fs::read_to_string(&page).unwrap(), preview.html);
    let folder = under(site.path(), "data/theme-preview");
    assert_eq!(fs::read(under(&folder, "site.css")).unwrap(), preview.css);
    let names: BTreeSet<_> = fs::read_dir(&folder)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(names.len(), 2, "no temporary file left: {names:?}");
}

#[cfg(unix)]
#[test]
fn a_data_folder_that_leads_outside_the_site_is_refused() {
    let site = tempfile::tempdir().unwrap();
    let elsewhere = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(elsewhere.path(), under(site.path(), "data")).unwrap();

    let error = preview(None).write(site.path()).unwrap_err();

    assert!(matches!(error, PreviewError::Path(_)), "{error:?}");
    assert_eq!(fs::read_dir(elsewhere.path()).unwrap().count(), 0);
    let message = error.message();
    assert!(message.what().contains("data/theme-preview"), "{message}");
    assert!(message.fix().contains("plain folder"), "{message}");
}

#[test]
fn a_write_that_fails_names_the_file_and_what_to_do() {
    let message = PreviewError::Write {
        path: PathBuf::from("site/data/theme-preview/index.html"),
        source: io::Error::from(io::ErrorKind::PermissionDenied),
    }
    .message();

    assert!(message.what().contains("index.html"), "{message}");
    assert!(message.fix().contains("may write"), "{message}");
}

fn select<'a>(html: &'a Html, selector: &str) -> Vec<ElementRef<'a>> {
    html.select(&Selector::parse(selector).unwrap()).collect()
}

/// What a machine can check of FRONTEND.md and RULES.md BL-WEB-1, BL-WEB-2
/// and BL-WEB-5 without a browser.
fn assert_valid_page(page: &str) {
    let html = Html::parse_document(page);
    assert!(html.errors.is_empty(), "parse errors: {:?}", html.errors);
    assert!(page.starts_with("<!doctype html>"), "a doctype first");
    let root = select(&html, "html")[0].value();
    assert!(root.attr("lang").is_some_and(|v| !v.is_empty()), "lang");
    assert!(matches!(root.attr("dir"), Some("ltr" | "rtl")), "dir");
    let levels: Vec<u8> = select(&html, "h1, h2, h3, h4, h5, h6")
        .iter()
        .map(|heading| heading.value().name().as_bytes()[1] - b'0')
        .collect();
    assert!(levels.windows(2).all(|w| w[1] <= w[0] + 1), "{levels:?}");
    for (selector, count) in [
        ("h1", 1),
        ("main#main", 1),
        ("a.skip-link[href='#main']", 1),
        ("body > header", 1),
        ("body > footer", 1),
        ("nav:not([aria-label])", 0),
        ("script", 0),
        ("a:not([href]), a[href=''], a[href='#']", 0),
    ] {
        assert_eq!(select(&html, selector).len(), count, "{selector}");
    }
    let ids: Vec<_> = select(&html, "[id]")
        .iter()
        .filter_map(|e| e.value().id())
        .collect();
    let unique: BTreeSet<_> = ids.iter().collect();
    assert_eq!(unique.len(), ids.len(), "ids are unique");
    assert!(!page.contains(" style="), "no inline style");
}
