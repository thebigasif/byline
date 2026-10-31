//! Which requests come from bots, and which bot each claims to be.

use khalid_analytics::{Crawler, CrawlerKind};

fn named(user_agent: &str) -> Option<(&'static str, CrawlerKind)> {
    Crawler::recognise(Some(user_agent)).map(|bot| (bot.name(), bot.kind()))
}

#[test]
fn search_engine_crawlers_are_named() {
    let googlebot = "Mozilla/5.0 (Linux; Android 6.0.1; Nexus 5X Build/MMB29P) \
        AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.6478.126 Mobile Safari/537.36 \
        (compatible; Googlebot/2.1; +http://www.google.com/bot.html)";
    assert_eq!(named(googlebot), Some(("Googlebot", CrawlerKind::Search)));
    let bingbot = "Mozilla/5.0 (compatible; bingbot/2.0; +http://www.bing.com/bingbot.htm)";
    assert_eq!(named(bingbot), Some(("Bingbot", CrawlerKind::Search)));
}

#[test]
fn ai_crawlers_are_named() {
    for (user_agent, name) in [
        (
            "Mozilla/5.0 AppleWebKit/537.36 (KHTML, like Gecko; compatible; GPTBot/1.2; +https://openai.com/gptbot)",
            "GPTBot",
        ),
        (
            "Mozilla/5.0 AppleWebKit/537.36 (KHTML, like Gecko); compatible; ChatGPT-User/1.0; +https://openai.com/bot",
            "ChatGPT-User",
        ),
        (
            "Mozilla/5.0 AppleWebKit/537.36 (KHTML, like Gecko; compatible; ClaudeBot/1.0; +claudebot@anthropic.com)",
            "ClaudeBot",
        ),
        (
            "Mozilla/5.0 AppleWebKit/537.36 (KHTML, like Gecko; compatible; PerplexityBot/1.0; +https://perplexity.ai/perplexitybot)",
            "PerplexityBot",
        ),
        ("CCBot/2.0 (https://commoncrawl.org/faq/)", "CCBot"),
    ] {
        assert_eq!(
            named(user_agent),
            Some((name, CrawlerKind::Ai)),
            "{user_agent}"
        );
    }
}

#[test]
fn seo_tools_and_link_previews_are_named() {
    let ahrefs = "Mozilla/5.0 (compatible; AhrefsBot/7.0; +http://ahrefs.com/robot/)";
    assert_eq!(named(ahrefs), Some(("AhrefsBot", CrawlerKind::Seo)));
    let facebook = "facebookexternalhit/1.1 (+http://www.facebook.com/externalhit_uatext.php)";
    assert_eq!(named(facebook), Some(("Facebook", CrawlerKind::Social)));
}

#[test]
fn scripts_and_unnamed_bots_are_other_bots() {
    for user_agent in [
        "curl/8.5.0",
        "python-requests/2.32.3",
        "Mozilla/5.0 (compatible; SomeNewCrawler/0.1)",
        "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 HeadlessChrome/126.0 Safari/537.36",
    ] {
        assert_eq!(
            named(user_agent),
            Some(("Other bot", CrawlerKind::Other)),
            "{user_agent}"
        );
    }
    let nameless = Crawler::recognise(None).map(|bot| (bot.name(), bot.kind()));
    assert_eq!(nameless, Some(("No user agent", CrawlerKind::Other)));
    assert_eq!(named("  "), Some(("No user agent", CrawlerKind::Other)));
}

#[test]
fn browsers_are_readers() {
    for user_agent in [
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36",
        "Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Mobile/15E148 Safari/604.1",
        "Mozilla/5.0 (X11; Linux x86_64; rv:131.0) Gecko/20100101 Firefox/131.0",
        "Lynx/2.9.0dev.12 libwww-FM/2.14 SSL-MM/1.4.1 GNUTLS/3.8.3",
    ] {
        assert_eq!(named(user_agent), None, "{user_agent}");
    }
}

#[test]
fn each_kind_reads_back_from_its_stored_name() {
    for kind in CrawlerKind::ALL {
        assert_eq!(CrawlerKind::from_stored(kind.as_str()), Some(kind));
    }
}
