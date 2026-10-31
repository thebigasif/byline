//! The AI visitors report: readers each AI assistant sent, against the
//! period before (GEO M1).

use chrono::NaiveDate;
use khalid_analytics::{AiTool, AiVisitors, Period, ReferrerGroup, Views};

const POST: &str = "/blog/internal-links/";
const OTHER: &str = "/blog/anchor-text/";

#[cfg(test)]
fn date(day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 10, day).unwrap()
}

#[cfg(test)]
fn ai(tool: AiTool) -> ReferrerGroup {
    ReferrerGroup::Ai(tool)
}

#[cfg(test)]
fn week() -> Period {
    Period::ending(date(28), 7).unwrap()
}

#[test]
fn a_period_counts_back_from_its_last_day_and_has_one_before_it() {
    let week = week();
    assert_eq!((week.first(), week.last()), (date(22), date(28)));
    let before = week.before();
    assert_eq!((before.first(), before.last()), (date(15), date(21)));
    assert_eq!(week.days(), 7);
    assert!(Period::ending(date(28), 0).is_none());
    assert!(Period::ending(date(28), Period::MOST_DAYS + 1).is_none());
}

#[test]
fn each_tool_gets_its_views_its_views_before_and_its_top_pages() {
    let views = [
        Views::new(date(16), POST, ai(AiTool::ChatGpt), 4),
        Views::new(date(20), POST, ai(AiTool::Claude), 3),
        Views::new(date(22), POST, ai(AiTool::ChatGpt), 5),
        Views::new(date(23), OTHER, ai(AiTool::ChatGpt), 7),
        Views::new(date(28), POST, ai(AiTool::Perplexity), 2),
        Views::new(date(28), POST, ReferrerGroup::Search, 20),
        Views::new(date(28), POST, ReferrerGroup::Internal, 50),
        Views::new(date(29), POST, ai(AiTool::Gemini), 9),
    ];
    let report = AiVisitors::from_views(week(), &views);
    assert_eq!(report.period, week());
    assert_eq!(report.arrivals, 34, "internal views are not arrivals");
    assert_eq!(report.ai_views, 14);
    let tools: Vec<_> = report
        .tools
        .iter()
        .map(|row| (row.tool, row.views, row.before))
        .collect();
    assert_eq!(
        tools,
        [
            (AiTool::ChatGpt, 12, 4),
            (AiTool::Perplexity, 2, 0),
            (AiTool::Claude, 0, 3),
        ]
    );
    let pages: Vec<_> = report.tools[0]
        .pages
        .iter()
        .map(|page| (page.path.as_str(), page.views))
        .collect();
    assert_eq!(pages, [(OTHER, 7), (POST, 5)]);
    assert_eq!(report.tools[2].pages, []);
}

#[test]
fn a_tool_lists_at_most_its_top_pages() {
    let views: Vec<_> = (0..12)
        .map(|n| Views::new(date(24), &format!("/blog/{n}/"), ai(AiTool::Claude), n + 1))
        .collect();
    let report = AiVisitors::from_views(week(), &views);
    let pages = &report.tools[0].pages;
    assert_eq!(pages.len(), AiVisitors::TOP_PAGES);
    assert_eq!(pages[0].path, "/blog/11/");
}

#[test]
fn no_ai_views_make_an_empty_report() {
    let views = [Views::new(date(24), POST, ReferrerGroup::Direct, 3)];
    let report = AiVisitors::from_views(week(), &views);
    assert_eq!((report.arrivals, report.ai_views), (3, 0));
    assert_eq!(report.tools, []);
}
