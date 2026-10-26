//! The demo site as render input.

use khalid_core::{AbsoluteUrl, Locale};
use khalid_render::{Alternate, Nav, NavLink, PageKind, PageMeta, RenderSite, Robots, ShareImage};

pub(crate) fn url(text: &str) -> AbsoluteUrl {
    AbsoluteUrl::parse(text).unwrap()
}

pub(crate) fn locale(tag: &str) -> Locale {
    Locale::try_new(tag).unwrap()
}

fn link(label: &str, href: &str) -> NavLink {
    NavLink {
        label: label.to_owned(),
        href: href.to_owned(),
    }
}

/// The demo site in `tag`, with the menus the demo site uses.
pub(crate) fn demo_site(tag: &str) -> RenderSite {
    let prefix = if tag == "en" { "/blog/" } else { "/blog/de/" };
    RenderSite {
        name: "Field Notes on SEO".to_owned(),
        locale: locale(tag),
        home: prefix.to_owned(),
        stylesheet: "/blog/assets/site.0123abcd.css".to_owned(),
        nav: Nav {
            primary: vec![
                link("Posts", prefix),
                link("Glossary", &format!("{prefix}glossary/")),
            ],
            footer: vec![link("About", &format!("{prefix}about/"))],
        },
        main_site: Some(link("Back to example.com", "https://www.example.com/")),
        share_image: Some(ShareImage {
            url: url("https://www.example.com/blog/assets/share.png"),
            alt: "Field Notes on SEO, a blog about search".to_owned(),
            width: 1200,
            height: 630,
        }),
    }
}

/// The head of a page at `path` that exists in English and German.
pub(crate) fn meta(title: &str, path: &str, kind: PageKind) -> PageMeta {
    let german = path.replacen("/blog/", "/blog/de/", 1);
    PageMeta {
        title: title.to_owned(),
        description: "Plain advice on search for small teams.".to_owned(),
        canonical: url(&format!("https://www.example.com{path}")),
        path: path.to_owned(),
        kind,
        robots: Robots::Index,
        image: None,
        alternates: vec![
            Alternate {
                locale: locale("en"),
                href: url(&format!("https://www.example.com{path}")),
                label: "English".to_owned(),
            },
            Alternate {
                locale: locale("de"),
                href: url(&format!("https://www.example.com{german}")),
                label: "Deutsch".to_owned(),
            },
        ],
        x_default: Some(url(&format!("https://www.example.com{path}"))),
        seo_head: None,
    }
}
