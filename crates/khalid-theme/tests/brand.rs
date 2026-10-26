//! The site's name, logo, share image and icons, read from its home page.

use khalid_core::AbsoluteUrl;
use khalid_theme::Brand;

#[cfg(test)]
fn url(text: &str) -> AbsoluteUrl {
    AbsoluteUrl::parse(text).unwrap()
}

#[test]
fn reads_name_logo_share_image_and_icons() {
    let html = r#"<!doctype html><html><head>
<title>Northwind Bakery | Sourdough in Leeds</title>
<meta property="og:site_name" content="Northwind Bakery">
<meta property="og:image" content="https://cdn.northwind.example/share/home.jpg">
<link rel="icon" href="/favicon.svg" type="image/svg+xml">
<link rel="apple-touch-icon" sizes="180x180" href="/apple-touch-icon.png">
<link rel="stylesheet" href="/site.css">
</head><body>
<header><a href="/"><img src="/img/wordmark.svg" alt="Northwind Bakery logo"></a>
<nav><img src="/img/cart.svg" alt=""></nav></header>
<main><img src="/img/loaf.jpg" alt="A loaf"></main>
</body></html>"#;

    let brand = Brand::from_html(html, &url("https://northwind.example/"));

    assert_eq!(brand.name.as_deref(), Some("Northwind Bakery"));
    assert_eq!(
        brand.logo,
        Some(url("https://northwind.example/img/wordmark.svg"))
    );
    assert_eq!(
        brand.share_image,
        Some(url("https://cdn.northwind.example/share/home.jpg"))
    );
    assert_eq!(
        brand.icons,
        [
            url("https://northwind.example/favicon.svg"),
            url("https://northwind.example/apple-touch-icon.png"),
        ]
    );
}

#[test]
fn falls_back_to_the_title_the_first_header_image_and_twitter_card() {
    let html = r#"<head><base href="https://static.acme.example/v2/">
<title>Acme Analytics – Dashboards for teams</title>
<meta name="twitter:image" content="cards/home.png">
<link rel="shortcut icon" href="favicon.ico"></head>
<body><header class="site-header"><img src="brand/mark.png" alt="Acme"></header></body>"#;

    let brand = Brand::from_html(html, &url("https://acme.example/"));

    assert_eq!(brand.name.as_deref(), Some("Acme Analytics"));
    assert_eq!(
        brand.logo,
        Some(url("https://static.acme.example/v2/brand/mark.png"))
    );
    assert_eq!(
        brand.share_image,
        Some(url("https://static.acme.example/v2/cards/home.png"))
    );
    assert_eq!(
        brand.icons,
        [url("https://static.acme.example/v2/favicon.ico")]
    );
}

#[test]
fn an_image_marked_as_a_logo_counts_outside_the_header() {
    let html = r#"<div class="top"><img src="/x.png" class="Logo-Image" alt=""></div><img src="/hero.jpg">"#;

    let brand = Brand::from_html(html, &url("https://shop.example/"));

    assert_eq!(brand.logo, Some(url("https://shop.example/x.png")));
}

#[test]
fn a_page_that_says_nothing_gives_an_empty_brand_and_unsafe_links_are_dropped() {
    let html = r#"<title>  </title><link rel="icon" href="javascript:alert(1)"><header><img src="data:image/png;base64,AA=="></header>"#;

    let brand = Brand::from_html(html, &url("https://empty.example/"));

    assert_eq!(brand, Brand::default());
}
