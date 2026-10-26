//! robots.txt with per-bot rules (task 1.13, KH-SEO-7).
//!
//! Every check goes through `robotstxt`, a port of Google's own parser and
//! matcher, so the tests ask what Googlebot would do, not what the text says.

#![cfg(test)]

mod common;

use common::{demo_site, internal_links_post, internal_links_post_de, path};
use khalid_seo::{BotAccess, RobotsPath, RobotsTxt, SeoError, SeoSite, UserAgent};
use robotstxt::DefaultMatcher;

fn agent(token: &str) -> UserAgent {
    UserAgent::try_new(token).unwrap()
}

/// The demo site with a private preview folder, AI training crawlers
/// blocked and Claude's search crawler let in.
fn demo_policy_site() -> SeoSite {
    let mut site = demo_site();
    site.crawlers.disallow = vec![RobotsPath::try_new("/preview/").unwrap()];
    site.crawlers
        .bots
        .insert(agent("GPTBot"), BotAccess::Blocked);
    site.crawlers
        .bots
        .insert(agent("CCBot"), BotAccess::Blocked);
    site.crawlers
        .bots
        .insert(agent("Claude-SearchBot"), BotAccess::Allowed);
    site
}

fn allowed(robots: &str, bot: &str, address: &str) -> bool {
    DefaultMatcher::default().one_agent_allowed_by_robots(robots, bot, address)
}

#[test]
fn the_demo_robots_txt() {
    let robots = RobotsTxt::for_site(&demo_policy_site()).unwrap();
    insta::assert_snapshot!(robots.to_string());
}

#[test]
fn every_bot_may_read_posts_unless_blocked() {
    let robots = RobotsTxt::for_site(&demo_policy_site())
        .unwrap()
        .to_string();
    let site = demo_policy_site();
    for post in [internal_links_post(), internal_links_post_de()] {
        let address = site.url_for(&post.language, &post.path).unwrap();
        let address = address.as_str();
        assert!(allowed(&robots, "Googlebot", address), "{address}");
        assert!(allowed(&robots, "Claude-SearchBot", address), "{address}");
        assert!(!allowed(&robots, "GPTBot", address), "{address}");
        assert!(!allowed(&robots, "CCBot", address), "{address}");
    }
}

#[test]
fn a_path_blocked_for_all_is_blocked_for_a_named_bot_too() {
    let robots = RobotsTxt::for_site(&demo_policy_site())
        .unwrap()
        .to_string();
    let preview = "https://www.example.com/blog/preview/draft-on-links/";
    assert!(!allowed(&robots, "Googlebot", preview));
    assert!(!allowed(&robots, "Claude-SearchBot", preview));
}

#[test]
fn a_blog_in_a_folder_says_nothing_about_the_rest_of_the_site() {
    let robots = RobotsTxt::for_site(&demo_policy_site())
        .unwrap()
        .to_string();
    assert!(allowed(
        &robots,
        "GPTBot",
        "https://www.example.com/pricing/"
    ));
}

#[test]
fn the_default_lets_everyone_in_and_names_the_sitemap() {
    let mut site = demo_site();
    site.mount = path("/");
    let robots = RobotsTxt::for_site(&site).unwrap().to_string();
    assert!(allowed(
        &robots,
        "Googlebot",
        "https://www.example.com/anything/"
    ));
    assert!(
        robots.contains("Sitemap: https://www.example.com/sitemap.xml"),
        "{robots}"
    );
}

#[test]
fn bot_names_and_paths_are_checked() {
    assert!(matches!(
        UserAgent::try_new("Bad Bot"),
        Err(SeoError::UserAgent { .. })
    ));
    assert!(matches!(
        UserAgent::try_new(""),
        Err(SeoError::UserAgent { .. })
    ));
    assert!(matches!(
        RobotsPath::try_new("preview/"),
        Err(SeoError::RobotsPath { .. })
    ));
    assert!(matches!(
        RobotsPath::try_new("/a b/"),
        Err(SeoError::RobotsPath { .. })
    ));
    assert!(RobotsPath::try_new("/*.pdf$").is_ok());
}
