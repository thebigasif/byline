//! The analytics desk: today from the site's clock, and the AI visitors
//! report from the counted views (GEO M1).

use std::sync::Arc;

use chrono::{NaiveDate, TimeZone, Utc};
use khalid_analytics::{AiTool, Period, ReferrerGroup, Views};
use khalid_api::analytics_desk::{AnalyticsDesk, ViewsInMemory};
use khalid_core::FixedClock;

#[cfg(test)]
fn date(day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 10, day).unwrap()
}

#[tokio::test]
async fn the_desk_reports_from_its_views_on_the_clock_s_day() {
    let late = Utc.with_ymd_and_hms(2026, 10, 28, 23, 59, 0).unwrap();
    let views = ViewsInMemory::new(vec![
        Views::new(date(20), "/a/", ReferrerGroup::Ai(AiTool::Claude), 2),
        Views::new(date(27), "/a/", ReferrerGroup::Ai(AiTool::Claude), 3),
    ]);
    let desk = AnalyticsDesk::new(Arc::new(views), Arc::new(FixedClock::new(late)));
    assert_eq!(desk.today(), date(28));
    let week = Period::ending(desk.today(), 7).unwrap();
    let report = desk.ai_visitors(week).await.unwrap();
    let claude: Vec<_> = report.tools.iter().map(|t| (t.views, t.before)).collect();
    assert_eq!(claude, [(3, 2)]);
}

#[tokio::test]
async fn a_site_that_counts_nothing_reports_nothing() {
    let desk = AnalyticsDesk::none();
    let week = Period::ending(desk.today(), 7).unwrap();
    let report = desk.ai_visitors(week).await.unwrap();
    assert_eq!((report.arrivals, report.ai_views), (0, 0));
    assert_eq!(report.tools, []);
}
