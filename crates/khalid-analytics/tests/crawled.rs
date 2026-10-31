//! Pages a checked bot fetched: one per page and bot, with the first and
//! last fetch, from listed bots answered with a page (GEO C2).

use std::net::IpAddr;

use chrono::{DateTime, TimeZone, Utc};
use khalid_analytics::{
    BotCheck, CrawledPage, Crawler, ReferrerGroup, ShortAddress, Visit, Visitor, VisitorKey,
};

#[cfg(test)]
fn at(hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 2, hour, 0, 0).unwrap()
}

#[cfg(test)]
fn visit(hour: u32, path: &str, status: u16, visitor: Visitor) -> Visit {
    let address: IpAddr = "66.249.66.1".parse().unwrap();
    Visit {
        at: at(hour),
        path: path.to_owned(),
        status,
        address: ShortAddress::of(address),
        code: VisitorKey::from_bytes([1; 32]).code(address, None),
        visitor,
    }
}

#[cfg(test)]
fn bot(name: &str, check: BotCheck) -> Visitor {
    Visitor::Bot(Crawler::named(name).unwrap(), check)
}

#[test]
fn a_listed_bot_answered_with_a_page_is_kept_with_its_first_and_last_fetch() {
    let google = bot("Googlebot", BotCheck::Listed);
    let visits = [
        visit(9, "/blog/geo/", 200, google),
        visit(7, "/blog/geo/", 304, google),
        visit(8, "/blog/geo/", 200, bot("Bingbot", BotCheck::Listed)),
        visit(11, "/blog/geo/", 200, google),
    ];
    let pages = CrawledPage::from_visits(&visits);
    let seen: Vec<(&str, &str, u32, u32)> = pages
        .iter()
        .map(|page| {
            let hour = |at: DateTime<Utc>| at.format("%H").to_string().parse().unwrap();
            (
                page.path.as_str(),
                page.crawler.name(),
                hour(page.first_at),
                hour(page.last_at),
            )
        })
        .collect();
    assert_eq!(
        seen,
        [
            ("/blog/geo/", "Bingbot", 8, 8),
            ("/blog/geo/", "Googlebot", 7, 11)
        ]
    );
}

#[test]
fn people_unchecked_bots_and_failed_answers_are_not_crawled_pages() {
    let visits = [
        visit(9, "/blog/geo/", 200, Visitor::Person(ReferrerGroup::Ai)),
        visit(9, "/blog/geo/", 200, bot("Googlebot", BotCheck::NotListed)),
        visit(9, "/blog/geo/", 200, bot("ClaudeBot", BotCheck::NoList)),
        visit(9, "/blog/gone/", 404, bot("Googlebot", BotCheck::Listed)),
        visit(9, "/blog/moved/", 301, bot("Googlebot", BotCheck::Listed)),
        visit(9, "/blog/broken/", 500, bot("Googlebot", BotCheck::Listed)),
    ];
    assert_eq!(CrawledPage::from_visits(&visits), []);
}
