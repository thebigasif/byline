//! Pages AI bots read that no AI assistant ever sent a reader to.

use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use khalid_analytics::{AiTool, CrawlerKind, Crawls, ReferrerGroup, Views, uncited};

#[cfg(test)]
fn date(day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, day).unwrap()
}

#[cfg(test)]
fn at(day: u32, hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, day, hour, 0, 0).unwrap()
}

#[cfg(test)]
fn read(day: u32, path: &str, crawler: &str, kind: CrawlerKind, status: u16) -> Crawls {
    Crawls {
        day: date(day),
        path: path.to_owned(),
        crawler: crawler.to_owned(),
        kind,
        hits: 2,
        last_seen: at(day, 10),
        last_status: status,
    }
}

#[cfg(test)]
fn ai(day: u32, path: &str, crawler: &str) -> Crawls {
    read(day, path, crawler, CrawlerKind::Ai, 200)
}

#[test]
fn a_page_ai_bots_read_with_no_ai_reader_is_listed() {
    let crawls = [
        ai(3, "/blog/internal-links/", "GPTBot"),
        ai(5, "/blog/internal-links/", "ClaudeBot"),
        ai(4, "/blog/internal-links/", "GPTBot"),
    ];
    let views = [Views::new(
        date(4),
        "/blog/internal-links/",
        ReferrerGroup::Search,
        9,
    )];
    let pages = uncited(&crawls, &views);
    assert_eq!(pages.len(), 1);
    let page = &pages[0];
    assert_eq!(page.path, "/blog/internal-links/");
    assert_eq!(page.reads, 6);
    assert_eq!(page.bots, ["ClaudeBot", "GPTBot"]);
    assert_eq!(page.last_read, at(5, 10));
}

#[test]
fn one_reader_from_an_ai_assistant_counts_as_cited() {
    let crawls = [
        ai(3, "/blog/cited/", "PerplexityBot"),
        ai(3, "/blog/quiet/", "PerplexityBot"),
    ];
    let views = [Views::new(
        date(9),
        "/blog/cited/",
        ReferrerGroup::Ai(AiTool::ChatGpt),
        1,
    )];
    let paths: Vec<_> = uncited(&crawls, &views)
        .into_iter()
        .map(|page| page.path)
        .collect();
    assert_eq!(paths, ["/blog/quiet/"]);
}

#[test]
fn other_bots_failed_reads_and_files_are_not_reads() {
    let crawls = [
        read(3, "/blog/searched/", "Googlebot", CrawlerKind::Search, 200),
        read(3, "/blog/tooled/", "AhrefsBot", CrawlerKind::Seo, 200),
        read(3, "/blog/gone/", "GPTBot", CrawlerKind::Ai, 404),
        read(3, "/blog/broken/", "GPTBot", CrawlerKind::Ai, 500),
        read(3, "/blog/redirected/", "GPTBot", CrawlerKind::Ai, 301),
        ai(3, "/robots.txt", "GPTBot"),
        ai(3, "/llms.txt", "ClaudeBot"),
        ai(3, "/media/cover/800.webp", "GPTBot"),
        read(3, "/blog/unchanged/", "GPTBot", CrawlerKind::Ai, 304),
        ai(3, "/about", "ClaudeBot"),
    ];
    let paths: Vec<_> = uncited(&crawls, &[])
        .into_iter()
        .map(|page| page.path)
        .collect();
    assert_eq!(paths, ["/about", "/blog/unchanged/"]);
}

#[test]
fn the_most_read_page_comes_first() {
    let mut busy = ai(3, "/blog/busy/", "GPTBot");
    busy.hits = 40;
    let crawls = [
        ai(3, "/blog/b/", "GPTBot"),
        busy,
        ai(3, "/blog/a/", "GPTBot"),
    ];
    let paths: Vec<_> = uncited(&crawls, &[])
        .into_iter()
        .map(|page| page.path)
        .collect();
    assert_eq!(paths, ["/blog/busy/", "/blog/a/", "/blog/b/"]);
}

#[test]
fn no_reads_means_nothing_to_list() {
    let views = [Views::new(date(3), "/blog/a/", ReferrerGroup::Direct, 3)];
    assert_eq!(uncited(&[], &views), []);
}
