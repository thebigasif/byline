//! The checks `khalid doctor` runs on the site's own files: the settings, the
//! routes and the content. Each reports its mistake the way every other
//! command does, but doctor goes on to the next check instead of stopping.

use std::path::Path;

use khalid_content::{ContentTree, SiteContent, check_bodies, read_content};
use khalid_core::{Config, Environment};
use khalid_routes::Routes;

use crate::doctor_check::{Check, Finding};
use crate::{config_message, content_message, routes_message};

/// Checks the settings, the routes and the content of `site`, in that order.
/// The content is read only when the settings and the routes load, because
/// reading it needs both.
pub(crate) fn check(site: &Path) -> Vec<Finding> {
    let config = Config::load(site, &Environment::from_process());
    let routes = Routes::load(site);
    let mut found = vec![
        match &config {
            Ok(config) => Finding::ok(
                Check::Settings,
                format!(
                    "site.toml loads the site {} at {}.",
                    config.name(),
                    config.address()
                ),
            ),
            Err(error) => Finding::problem(Check::Settings, config_message::of(error)),
        },
        match &routes {
            Ok(_) => Finding::ok(
                Check::Routes,
                "Every post gets an address, from routes.toml or the defaults.",
            ),
            Err(error) => Finding::problem(Check::Routes, routes_message::of(error)),
        },
    ];
    found.push(match (&config, &routes) {
        (Ok(config), Ok(routes)) => content(site, config, routes),
        _ => Finding::skipped(
            Check::Content,
            "Khalid reads the content with the settings and the routes, so fix those first.",
        ),
    });
    found
}

/// Reads every content file and checks every block in it.
fn content(site: &Path, config: &Config, routes: &Routes) -> Finding {
    let read = ContentTree::walk(site)
        .and_then(|tree| read_content(&tree))
        .and_then(|content| check_bodies(&content, config, routes).map(|()| content));
    match read {
        Ok(content) => Finding::ok(Check::Content, summary(&content)),
        Err(error) => Finding::problem(Check::Content, content_message::of(site, &error)),
    }
}

/// How much content was read, in one sentence.
fn summary(content: &SiteContent) -> String {
    format!(
        "Read {}, {} and {}, with no mistake in any of them.",
        counted(content.posts.len(), "post"),
        counted(content.pages.len(), "page"),
        counted(content.authors.len(), "author"),
    )
}

/// `count` of `noun`, such as `1 post` or `10 posts`.
fn counted(count: usize, noun: &str) -> String {
    if count == 1 {
        format!("1 {noun}")
    } else {
        format!("{count} {noun}s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_is_said_without_a_plural() {
        assert_eq!(counted(1, "post"), "1 post");
        assert_eq!(counted(0, "page"), "0 pages");
        assert_eq!(counted(10, "author"), "10 authors");
    }
}
