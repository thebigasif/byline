//! Theme sync end to end on three sites shaped like real ones: each is
//! served from `fixtures/sites/` by a local `wiremock` server, fetched
//! through `Outbound`, and read into a `SiteTheme`. Nothing is fetched
//! from the internet.

mod common;

use common::{sync, url};
use khalid_theme::{Brand, FontHost, FontSource, Rgb, SiteTheme};

#[cfg(test)]
fn hex(text: &str) -> Option<Rgb> {
    Rgb::from_css(text)
}

#[tokio::test]
async fn a_wordpress_block_theme_gives_its_global_styles() {
    let (theme, home) = sync("bakery", "/wp-content/themes/northwind/style.css").await;

    let palette = theme.palette;
    assert_eq!(palette.background, hex("#fbf8f3"), "dark scheme left out");
    assert_eq!(palette.text, hex("#2b2118"));
    assert_eq!(palette.link, hex("#9c3d1c"));
    assert_eq!(palette.accent, hex("#9c3d1c"));
    let fonts = &theme.typography;
    assert_eq!(fonts.body.as_deref(), Some(r#""Lora", serif"#));
    assert_eq!(fonts.heading.as_deref(), Some(r#""Fraunces", serif"#));
    let families: Vec<&str> = fonts.sources.iter().map(FontSource::family).collect();
    assert_eq!(families, ["Fraunces", "Lora"]);
    assert!(
        fonts
            .sources
            .iter()
            .all(|source| source.host() == FontHost::Google)
    );
    let brand = &theme.brand;
    assert_eq!(brand.name.as_deref(), Some("Northwind Bakery"));
    let upload = |file: &str| Some(url(&format!("{home}/wp-content/uploads/{file}")));
    assert_eq!(brand.logo, upload("2026/01/northwind-wordmark.png"));
    assert_eq!(brand.share_image, upload("2026/09/shopfront-1200x630.jpg"));
}

#[tokio::test]
async fn utility_css_with_hashed_classes_is_read_by_frequency() {
    let (theme, home) = sync("saas", "/_next/static/css/4f1c2a9e.css").await;

    let palette = theme.palette;
    assert_eq!(palette.background, hex("#ffffff"));
    assert_eq!(
        palette.text,
        hex("#111827"),
        "the faint grey is used more but fails"
    );
    assert_eq!(palette.accent, hex("#4f46e5"), "inline styles count too");
    assert_eq!(palette.link, None, "no rule styles plain links");
    let fonts = &theme.typography;
    assert_eq!(
        fonts.body.as_deref(),
        Some(r#""Inter var", ui-sans-serif, system-ui, sans-serif"#)
    );
    assert_eq!(fonts.heading, None);
    assert_eq!(fonts.sources.len(), 1);
    assert_eq!(fonts.sources[0].host(), FontHost::SelfHosted);
    let brand = &theme.brand;
    assert_eq!(brand.name.as_deref(), Some("Acme Analytics"));
    let logo = format!("{home}/_next/static/media/acme-logo.8d2f.svg");
    assert_eq!(brand.logo, Some(url(&logo)));
    assert_eq!(
        brand.share_image,
        Some(url("https://acme.example/og/home.png"))
    );
}

#[test]
fn pasted_variables_alone_give_the_colours_and_fonts_and_no_brand() {
    let pasted = ":root {\n  --color-background: #fdfcfa;\n  --color-text: #1e2a3a;\n  \
                  --color-link: #b4232a;\n  --font-body: \"Source Serif 4\", Georgia, serif;\n}\n";

    let theme = SiteTheme::from_css(pasted);

    let palette = theme.palette;
    assert_eq!(palette.background, hex("#fdfcfa"));
    assert_eq!(palette.text, hex("#1e2a3a"));
    assert_eq!(palette.link, hex("#b4232a"));
    assert_eq!(
        theme.typography.body.as_deref(),
        Some(r#""Source Serif 4", Georgia, serif"#)
    );
    assert_eq!(theme.brand, Brand::default());
}

#[tokio::test]
async fn a_bootstrap_site_gives_its_variables_and_an_adobe_font_is_not_rehosted() {
    let (theme, home) = sync("agency", "/assets/css/main.min.css").await;

    let palette = theme.palette;
    assert_eq!(palette.background, hex("#ffffff"));
    assert_eq!(palette.text, hex("#212529"), "the dark variant is left out");
    assert_eq!(palette.link, hex("#c2255c"));
    assert_eq!(palette.accent, hex("#d6336c"));
    let fonts = &theme.typography;
    assert_eq!(
        fonts.body.as_deref(),
        Some(r#"proxima-nova, "Helvetica Neue", Arial, sans-serif"#)
    );
    assert!(
        fonts
            .sources
            .iter()
            .all(|source| source.family() != "proxima-nova"),
        "an Adobe kit names no family we may serve"
    );
    let brand = &theme.brand;
    assert_eq!(brand.name.as_deref(), Some("Studio Kite"));
    assert_eq!(
        brand.logo,
        Some(url(&format!("{home}/assets/img/studio-kite-logo.svg")))
    );
    assert_eq!(brand.icons.len(), 2);
}
