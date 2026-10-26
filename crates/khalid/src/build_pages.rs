//! The pages `khalid build` renders: the site turned into the input records of
//! `khalid-render` (ADR 0009), one language at a time.

use khalid_core::{AbsoluteUrl, Config, Locale, UserMessage};
use khalid_render::{Nav, PageBody, PageKind, PageMeta, RenderListing, RenderSite, Robots};

use crate::build_message;

/// The pages of one language, and the site as they show it.
#[derive(Debug)]
pub(crate) struct LanguagePages {
    pub(crate) site: RenderSite,
    pub(crate) pages: Vec<Page>,
}

/// One page and the file it goes to, relative to the output folder.
#[derive(Debug)]
pub(crate) struct Page {
    pub(crate) file: String,
    pub(crate) meta: PageMeta,
    pub(crate) body: PageBody,
}

/// Every page of the site, per language in the order of `site.toml`: the home
/// page and the 404 page.
pub(crate) fn pages(config: &Config) -> Result<Vec<LanguagePages>, UserMessage> {
    config
        .languages()
        .iter()
        .map(|locale| language_pages(config, locale))
        .collect()
}

fn language_pages(config: &Config, locale: &Locale) -> Result<LanguagePages, UserMessage> {
    let home = RenderListing {
        title: config.name().to_owned(),
        intro: None,
        posts: Vec::new(),
        pagination: None,
    };
    let home_path = path_of(config, locale, "");
    let home_meta = meta(
        config,
        config.name().to_owned(),
        home_path,
        PageKind::Website,
    )?;
    let mut missing = home_meta.clone();
    missing.robots = Robots::NoIndex;
    let pages = vec![
        page(locale, config, "", home_meta, PageBody::Home(home)),
        Page {
            file: format!("{}404.html", prefix(config, locale)),
            meta: missing,
            body: PageBody::NotFound,
        },
    ];
    Ok(LanguagePages {
        site: render_site(config, locale),
        pages,
    })
}

/// The head of a page at `path`, indexable, with no description or
/// alternates yet.
fn meta(
    config: &Config,
    title: String,
    path: String,
    kind: PageKind,
) -> Result<PageMeta, UserMessage> {
    let address = format!("{}{path}", config.address().as_str().trim_end_matches('/'));
    let canonical = AbsoluteUrl::parse(&address).map_err(|_| build_message::address(&address))?;
    Ok(PageMeta {
        title,
        description: String::new(),
        canonical,
        path,
        kind,
        robots: Robots::Index,
        image: None,
        alternates: Vec::new(),
        x_default: None,
    })
}

/// A page whose path ends in `rest`, written to `rest/index.html`.
fn page(locale: &Locale, config: &Config, rest: &str, meta: PageMeta, body: PageBody) -> Page {
    Page {
        file: format!("{}{rest}index.html", prefix(config, locale)),
        meta,
        body,
    }
}

/// The site as pages in `locale` show it. The stylesheet arrives with task 1.12.
fn render_site(config: &Config, locale: &Locale) -> RenderSite {
    RenderSite {
        name: config.name().to_owned(),
        locale: locale.clone(),
        home: path_of(config, locale, ""),
        stylesheet: format!("{}assets/site.css", config.mount()),
        nav: Nav::default(),
        main_site: None,
        share_image: None,
    }
}

/// The path of `rest` in `locale`: the mount, the language prefix for any
/// language but the default, then `rest`, the rule `khalid_seo::SeoSite::url_for`
/// follows, so `why-links-matter/` in German is `/blog/de/why-links-matter/`.
fn path_of(config: &Config, locale: &Locale, rest: &str) -> String {
    format!("{}{}{rest}", config.mount(), prefix(config, locale))
}

/// `de/` for a language other than the default, and nothing for the default.
fn prefix(config: &Config, locale: &Locale) -> String {
    if locale == config.default_language() {
        String::new()
    } else {
        format!("{}/", locale.as_str().to_ascii_lowercase())
    }
}
