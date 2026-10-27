//! The search page of each language: the plain `GET` form when a running
//! server answers it, the lock card when the site is plain files (plan
//! section 7, KH-WEB-2, KH-OPS-1).

use khalid_core::{Config, Mode, UserMessage};
use khalid_render::{PageBody, PageKind, RenderSearch, Renderer, SearchMode};
use khalid_seo::{PageKind as SeoKind, SeoSite, SiteSearch, UrlPath};

use crate::build_pages::{self, LanguagePages, Page};
use crate::{build_message, build_seo, serve_message};

/// The end of the search page's address, after the mount and the language.
pub(crate) const REST: &str = "search/";

/// The strings file's name for the search page, which titles it.
const TITLE: &str = "search_link";

/// Whether the site can answer a search: only a running server can.
pub(crate) fn mode(config: &Config) -> SearchMode {
    if config.mode() == Mode::Server {
        SearchMode::Live
    } else {
        SearchMode::Locked
    }
}

/// Adds the search page to each of `languages`, titled in its language by
/// `renderer`'s strings. When the site answers searches, every page of the
/// language links to it from its header.
///
/// # Errors
///
/// Returns what, why and fix when the page's address cannot be made from
/// the site's settings.
pub(crate) fn add(
    config: &Config,
    renderer: &Renderer,
    languages: &mut [LanguagePages],
) -> Result<(), UserMessage> {
    let mode = mode(config);
    for language in languages.iter_mut() {
        let locale = language.site.locale.clone();
        let title = renderer
            .text(&locale, TITLE)
            .ok_or_else(serve_message::internal)?
            .to_owned();
        let path = build_pages::path_of(config, &locale, REST);
        let meta = build_pages::meta(
            config,
            title.clone(),
            String::new(),
            path.clone(),
            PageKind::Website,
        )?;
        let seo = build_seo::page(SeoKind::Search, &locale, REST, &title)?;
        let body = PageBody::Search(RenderSearch {
            title,
            mode,
            action: path.clone(),
            query: None,
            results: Vec::new(),
        });
        language.pages.push(Page {
            file: format!("{}{REST}index.html", build_pages::prefix(config, &locale)),
            meta,
            body,
            seo,
            source: None,
            template: None,
            head_code: None,
        });
    }
    link(config, languages);
    Ok(())
}

/// Links the search page from the header of every page of `languages`
/// when the site answers searches, as [`add`] does. The home pages
/// `khalid serve` renders first take the link this way, so they match the
/// full render's.
pub(crate) fn link(config: &Config, languages: &mut [LanguagePages]) {
    if mode(config) != SearchMode::Live {
        return;
    }
    for language in languages {
        let path = build_pages::path_of(config, &language.site.locale, REST);
        language.site.search = Some(path);
    }
}

/// Tells search engines where the site answers searches, in the `WebSite`
/// node, so they may show a search box for it. Only a running server does.
///
/// # Errors
///
/// Returns what, why and fix when the search page's path is not valid.
pub(crate) fn announce(config: &Config, site: &mut SeoSite) -> Result<(), UserMessage> {
    if mode(config) == SearchMode::Live {
        let path =
            UrlPath::try_new(&format!("/{REST}")).map_err(|error| build_message::seo(&error))?;
        site.search = SiteSearch::On(path);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use khalid_core::Environment;
    use khalid_routes::Routes;

    use super::*;
    use crate::site_pages::SitePages;

    fn demo() -> PathBuf {
        let mut path = Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf();
        path.push("../../fixtures/demo-site");
        path
    }

    /// The HTML of the page at `address` among the pages of the site in
    /// `folder`, as `khalid serve` renders them.
    fn served(folder: &Path, address: &str) -> String {
        let config = Config::load(folder, &Environment::empty()).unwrap();
        let routes = Routes::load(folder).unwrap();
        let pages = SitePages::new(folder, config, routes, None).unwrap();
        let rendered = pages.render_all().unwrap().pages;
        let page = rendered.iter().find(|page| page.path().as_str() == address);
        String::from_utf8_lossy(&page.unwrap().body().unwrap()).into_owned()
    }

    fn copy_dir(from: &Path, into: &Path) {
        fs::create_dir_all(into).unwrap();
        for entry in fs::read_dir(from).unwrap() {
            let entry = entry.unwrap();
            let mut target = into.to_path_buf();
            target.push(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy_dir(&entry.path(), &target);
            } else {
                fs::copy(entry.path(), &target).unwrap();
            }
        }
    }

    #[test]
    fn a_server_has_a_search_form_in_each_language_and_links_it_from_every_page() {
        let english = served(&demo(), "/blog/search/");
        assert!(english.contains("<h1>Search</h1>"), "{english}");
        assert!(english.contains(
            r#"<form class="search" role="search" action="/blog/search/" method="get">"#
        ));
        assert!(
            english.contains(r#"<meta name="robots" content="noindex"#),
            "{english}"
        );
        let german = served(&demo(), "/blog/de/search/");
        assert!(german.contains("<h1>Suche</h1>"), "{german}");
        assert!(german.contains(r#"action="/blog/de/search/""#));
        let home = served(&demo(), "/blog/");
        assert!(home.contains(r#"<a class="search-link" href="/blog/search/">Search</a>"#));
        assert!(
            home.contains(
                r#""urlTemplate":"https://www.example.com/blog/search/?q={search_term_string}""#
            ),
            "{home}"
        );
    }

    #[test]
    fn a_static_site_shows_the_lock_card_and_links_no_search() {
        let site = tempfile::tempdir().unwrap();
        copy_dir(&demo(), site.path());
        let mut settings = site.path().to_path_buf();
        settings.push("site.toml");
        let text = fs::read_to_string(&settings).unwrap();
        fs::write(
            &settings,
            text.replace("mode = \"server\"", "mode = \"static\""),
        )
        .unwrap();

        let page = served(site.path(), "/blog/search/");
        assert!(page.contains("Search needs a running server"), "{page}");
        assert!(page.contains("<code>khalid serve</code>"));
        assert!(!page.contains("<form"));
        let home = served(site.path(), "/blog/");
        assert!(!home.contains("search-link"));
        assert!(!home.contains("SearchAction"));
    }
}
