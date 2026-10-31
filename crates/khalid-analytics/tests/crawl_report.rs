//! The crawl reports: health per crawler (GEO C3) and errors bots got
//! (C5). A bot not on its list never counts as that bot.

use std::net::IpAddr;

use chrono::{DateTime, TimeZone, Utc};
use khalid_analytics::{
    AiTool, Answers, BotCheck, CrawledPage, Crawler, ReferrerGroup, ShortAddress, Visit, Visitor,
    VisitorKey, bot_errors, crawl_health,
};

#[cfg(test)]
fn at(day: u32, hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, day, hour, 0, 0).unwrap()
}

#[cfg(test)]
fn named(name: &str) -> Crawler {
    Crawler::named(name).unwrap()
}

#[cfg(test)]
fn visit(at: DateTime<Utc>, path: &str, status: u16, visitor: Visitor) -> Visit {
    let address: IpAddr = "66.249.66.1".parse().unwrap();
    Visit {
        at,
        path: path.to_owned(),
        status,
        address: ShortAddress::of(address),
        code: VisitorKey::from_bytes([2; 32]).code(address, None),
        visitor,
    }
}

#[cfg(test)]
fn bot(name: &str, check: BotCheck) -> Visitor {
    Visitor::Bot(named(name), check)
}

#[cfg(test)]
fn crawled(path: &str, name: &str, first: DateTime<Utc>, last: DateTime<Utc>) -> CrawledPage {
    CrawledPage {
        path: path.to_owned(),
        crawler: named(name),
        first_at: first,
        last_at: last,
    }
}

#[test]
fn health_counts_answers_per_crawler_and_keeps_pretenders_apart() {
    let visits = [
        visit(
            at(3, 9),
            "/blog/geo/",
            200,
            bot("Googlebot", BotCheck::Listed),
        ),
        visit(
            at(3, 10),
            "/blog/gone/",
            404,
            bot("Googlebot", BotCheck::Listed),
        ),
        visit(
            at(3, 11),
            "/blog/geo/",
            500,
            bot("Googlebot", BotCheck::Listed),
        ),
        visit(
            at(3, 12),
            "/wp-login.php",
            404,
            bot("Googlebot", BotCheck::NotListed),
        ),
        visit(
            at(3, 13),
            "/blog/geo/",
            200,
            bot("ClaudeBot", BotCheck::NoList),
        ),
        visit(
            at(3, 14),
            "/blog/geo/",
            200,
            bot("AhrefsBot", BotCheck::NoList),
        ),
        visit(
            at(3, 15),
            "/blog/geo/",
            200,
            Visitor::Person(ReferrerGroup::Ai(AiTool::ChatGpt)),
        ),
    ];
    // Bingbot fetched a page before the log's oldest row.
    let kept = [crawled("/blog/geo/", "Bingbot", at(1, 0), at(1, 8))];
    let health = crawl_health(&visits, &kept);
    let names: Vec<&str> = health.iter().map(|one| one.crawler.name()).collect();
    assert_eq!(names, ["Bingbot", "Googlebot", "ClaudeBot"]);
    let google = &health[1];
    let answers = Answers {
        ok: 1,
        moved: 0,
        missing: 1,
        failed: 1,
    };
    assert_eq!(google.listed, answers);
    assert_eq!(google.unchecked, Answers::default());
    assert_eq!((google.pretending, google.pages), (1, 2));
    assert_eq!(google.last_at, Some(at(3, 11)));
    assert_eq!(health[0].last_at, Some(at(1, 8)));
    assert_eq!(health[0].pages, 0);
    assert_eq!(health[2].unchecked.ok, 1);
}

#[test]
fn bot_errors_group_by_address_and_status_most_hits_first() {
    let visits = [
        visit(
            at(3, 9),
            "/blog/broken/",
            500,
            bot("Googlebot", BotCheck::Listed),
        ),
        visit(
            at(3, 9),
            "/blog/gone/",
            404,
            bot("Googlebot", BotCheck::Listed),
        ),
        visit(
            at(3, 10),
            "/blog/gone/",
            404,
            bot("GPTBot", BotCheck::Listed),
        ),
        visit(
            at(3, 11),
            "/blog/gone/",
            404,
            bot("ClaudeBot", BotCheck::NoList),
        ),
        visit(
            at(3, 12),
            "/wp-login.php",
            404,
            bot("Googlebot", BotCheck::NotListed),
        ),
        visit(
            at(3, 12),
            "/blog/gone/",
            404,
            bot("AhrefsBot", BotCheck::NoList),
        ),
        visit(
            at(3, 12),
            "/blog/geo/",
            200,
            bot("Googlebot", BotCheck::Listed),
        ),
        visit(
            at(3, 13),
            "/blog/gone/",
            404,
            Visitor::Person(ReferrerGroup::Ai(AiTool::ChatGpt)),
        ),
    ];
    let errors = bot_errors(&visits);
    let seen: Vec<(&str, u16, u64, Vec<&str>)> = errors
        .iter()
        .map(|error| {
            let names = error.crawlers.iter().map(|bot| bot.name()).collect();
            (error.path.as_str(), error.status, error.hits, names)
        })
        .collect();
    assert_eq!(
        seen,
        [
            (
                "/blog/gone/",
                404,
                3,
                vec!["ClaudeBot", "GPTBot", "Googlebot"]
            ),
            ("/blog/broken/", 500, 1, vec!["Googlebot"]),
        ]
    );
    assert_eq!(errors[0].last_at, at(3, 11));
}
