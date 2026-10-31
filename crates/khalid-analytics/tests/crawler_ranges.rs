//! Telling a real crawler from one that only copies its name, by the
//! address lists Google, Bing, `OpenAI` and Perplexity publish (GEO C1, D2).

use std::net::IpAddr;

use khalid_analytics::{BotCheck, Crawler, CrawlerRanges};

/// The shape every one of these companies publishes, cut down.
const GOOGLEBOT: &str = r#"{"creationTime":"2026-10-30T14:46:04.000000","prefixes":[
{"ipv6Prefix":"2001:4860:4801:10::/64"},{"ipv4Prefix":"66.249.64.0/27"}]}"#;

#[cfg(test)]
fn ip(text: &str) -> IpAddr {
    text.parse().unwrap()
}

#[cfg(test)]
fn bot(name: &str) -> Crawler {
    Crawler::named(name).unwrap()
}

#[cfg(test)]
fn with_googlebot() -> CrawlerRanges {
    let mut ranges = CrawlerRanges::default();
    let list = CrawlerRanges::LISTS
        .iter()
        .find(|list| list.crawlers.contains(&"Googlebot"))
        .unwrap();
    assert_eq!(ranges.add(list, GOOGLEBOT.as_bytes()).unwrap(), 2);
    ranges
}

#[test]
fn an_address_inside_a_listed_range_is_the_real_bot() {
    let ranges = with_googlebot();
    let googlebot = bot("Googlebot");
    assert_eq!(
        ranges.check(googlebot, ip("66.249.64.31")),
        BotCheck::Listed
    );
    assert_eq!(
        ranges.check(googlebot, ip("2001:4860:4801:10::abcd")),
        BotCheck::Listed
    );
    assert_eq!(
        ranges.check(googlebot, ip("::ffff:66.249.64.1")),
        BotCheck::Listed
    );
}

#[test]
fn an_address_outside_every_range_only_claims_the_name() {
    let ranges = with_googlebot();
    let googlebot = bot("Googlebot");
    assert_eq!(
        ranges.check(googlebot, ip("66.249.64.32")),
        BotCheck::NotListed
    );
    assert_eq!(
        ranges.check(googlebot, ip("203.0.113.7")),
        BotCheck::NotListed
    );
    assert_eq!(
        ranges.check(googlebot, ip("2001:4860:4801:11::1")),
        BotCheck::NotListed
    );
}

#[test]
fn one_google_list_covers_its_other_common_crawlers() {
    let ranges = with_googlebot();
    let inspection = bot("Google-InspectionTool");
    assert_eq!(
        ranges.check(inspection, ip("66.249.64.2")),
        BotCheck::Listed
    );
}

#[test]
fn a_bot_with_a_list_not_yet_read_or_no_list_at_all_says_so() {
    let ranges = with_googlebot();
    assert_eq!(
        ranges.check(bot("GPTBot"), ip("66.249.64.2")),
        BotCheck::NoList
    );
    assert_eq!(
        ranges.check(bot("ClaudeBot"), ip("66.249.64.2")),
        BotCheck::NoList
    );
    assert_eq!(
        ranges.check(bot("Other bot"), ip("66.249.64.2")),
        BotCheck::NoList
    );
}

#[test]
fn every_list_names_only_bots_the_log_knows_and_is_fetched_over_https() {
    for list in CrawlerRanges::LISTS {
        assert!(list.url.starts_with("https://"), "{}", list.url);
        for name in list.crawlers {
            assert!(Crawler::named(name).is_some(), "{name}");
        }
    }
}

#[test]
fn a_list_that_is_not_the_published_shape_is_refused_and_changes_nothing() {
    let mut ranges = with_googlebot();
    let list = &CrawlerRanges::LISTS[0];
    for body in [
        "<html>moved</html>",
        r#"{"prefixes":[{"ipv4Prefix":"66.249.64.0/33"}]}"#,
        r#"{"prefixes":[{"ipv4Prefix":"not an address/24"}]}"#,
        r#"{"prefixes":[{"ipv6Prefix":"2001:db8::/129"}]}"#,
        r#"{"prefixes":[]}"#,
    ] {
        assert!(ranges.add(list, body.as_bytes()).is_err(), "{body}");
    }
    assert_eq!(
        ranges.check(bot("Googlebot"), ip("66.249.64.31")),
        BotCheck::Listed
    );
}

#[test]
fn a_fresh_list_replaces_the_old_one() {
    let mut ranges = with_googlebot();
    let list = &CrawlerRanges::LISTS[0];
    let moved = r#"{"prefixes":[{"ipv4Prefix":"192.0.2.0/24"}]}"#;
    assert_eq!(ranges.add(list, moved.as_bytes()).unwrap(), 1);
    let googlebot = bot("Googlebot");
    assert_eq!(
        ranges.check(googlebot, ip("66.249.64.31")),
        BotCheck::NotListed
    );
    assert_eq!(ranges.check(googlebot, ip("192.0.2.9")), BotCheck::Listed);
}
