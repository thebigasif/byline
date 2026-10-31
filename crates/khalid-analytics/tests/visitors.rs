//! The visitor counts: page views per day, per source and per page, against
//! the period before (KH-GRO-4).

use chrono::NaiveDate;
use khalid_analytics::{AiTool, Period, ReferrerGroup, Source, Views, Visitors};

const POST: &str = "/blog/internal-links/";
const OTHER: &str = "/blog/anchor-text/";

#[cfg(test)]
fn date(day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 10, day).unwrap()
}

#[cfg(test)]
fn week() -> Period {
    Period::ending(date(28), 7).unwrap()
}

#[cfg(test)]
fn counts() -> Visitors {
    let views = [
        Views::new(date(16), POST, ReferrerGroup::Search, 4),
        Views::new(date(22), POST, ReferrerGroup::Search, 5),
        Views::new(date(22), OTHER, ReferrerGroup::Ai(AiTool::ChatGpt), 2),
        Views::new(date(23), OTHER, ReferrerGroup::Ai(AiTool::Claude), 1),
        Views::new(date(28), POST, ReferrerGroup::Internal, 3),
        Views::new(date(28), POST, ReferrerGroup::Direct, 1),
        Views::new(date(29), POST, ReferrerGroup::Social, 9),
    ];
    Visitors::from_views(week(), &views)
}

#[test]
fn views_are_counted_against_the_period_before() {
    let counts = counts();
    assert_eq!(counts.period, week());
    assert_eq!((counts.views, counts.before), (12, 4));
    assert_eq!(
        counts.arrivals, 9,
        "views from inside the site are not arrivals"
    );
}

#[test]
fn every_source_is_listed_once_in_order_with_ai_tools_together() {
    let sources: Vec<_> = counts()
        .sources
        .iter()
        .map(|row| (row.source.key(), row.views, row.before))
        .collect();
    assert_eq!(
        sources,
        [
            ("direct", 1, 0),
            ("search", 5, 4),
            ("social", 0, 0),
            ("ai", 3, 0),
            ("other", 0, 0),
            ("internal", 3, 0),
        ]
    );
    assert_eq!(Source::of(ReferrerGroup::Ai(AiTool::Gemini)), Source::Ai);
}

#[test]
fn every_day_of_the_period_has_a_count_even_an_empty_one() {
    let days: Vec<_> = counts()
        .days
        .iter()
        .map(|row| (row.day, row.views))
        .collect();
    assert_eq!(
        days,
        [
            (date(22), 7),
            (date(23), 1),
            (date(24), 0),
            (date(25), 0),
            (date(26), 0),
            (date(27), 0),
            (date(28), 4),
        ]
    );
}

#[test]
fn pages_come_most_viewed_first_and_stop_at_the_top() {
    let counted = counts();
    let pages: Vec<_> = counted
        .pages
        .iter()
        .map(|row| (row.path.as_str(), row.views))
        .collect();
    assert_eq!(pages, [(POST, 9), (OTHER, 3)]);
    let many: Vec<Views> = (0..30)
        .map(|n| {
            Views::new(
                date(28),
                &format!("/blog/post-{n}/"),
                ReferrerGroup::Direct,
                1,
            )
        })
        .collect();
    let counts = Visitors::from_views(week(), &many);
    assert_eq!(counts.pages.len(), Visitors::TOP_PAGES);
}

#[test]
fn a_site_that_counted_nothing_still_has_its_days() {
    let counts = Visitors::from_views(week(), &[]);
    assert_eq!((counts.views, counts.arrivals), (0, 0));
    assert_eq!(counts.days.len(), 7);
    assert_eq!(counts.sources.len(), Source::ALL.len());
    assert_eq!(counts.pages, []);
}
