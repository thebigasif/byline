//! How long a preview link works: 7 days unless `site.toml` sets
//! `preview_link_days` or `preview_link_hours`, from 1 hour to 30 days
//! (CTO answers 2026-10-27 section 4).

use std::path::Path;
use std::time::Duration;

use khalid_core::{Config, Environment};

const SITE: &str = r#"name = "Field Notes on SEO"
address = "https://www.example.com"
default_language = "en"
languages = ["en"]
"#;

/// Reads the site with `extra` lines added.
#[cfg(test)]
fn with(extra: &str) -> Result<Config, String> {
    let text = format!("{SITE}{extra}");
    Config::from_toml(Path::new("site.toml"), &text, &Environment::empty())
        .map_err(|error| error.to_string())
}

#[cfg(test)]
fn hours(count: u64) -> Duration {
    Duration::from_secs(count * 3600)
}

#[test]
fn a_preview_link_works_for_7_days_by_default() {
    assert_eq!(with("").unwrap().preview_link_lifetime(), hours(7 * 24));
}

#[test]
fn a_site_sets_the_lifetime_in_days_or_hours() {
    let days = with("preview_link_days = 30\n").unwrap();
    assert_eq!(days.preview_link_lifetime(), hours(30 * 24));
    let one = with("preview_link_hours = 1\n").unwrap();
    assert_eq!(one.preview_link_lifetime(), hours(1));
}

#[test]
fn a_lifetime_outside_1_hour_to_30_days_is_refused() {
    let expected = "site.toml, line 5, field \"preview_link_days\": a preview link works for 1 hour to 30 days, so use 1 to 30 days or 1 to 720 hours";
    assert_eq!(with("preview_link_days = 31\n").unwrap_err(), expected);
    assert_eq!(
        with("preview_link_days = 0\n").unwrap_err(),
        expected,
        "zero days"
    );
    let hours = with("preview_link_hours = 721\n").unwrap_err();
    assert!(hours.contains("field \"preview_link_hours\""), "{hours}");
}

#[test]
fn days_and_hours_together_are_refused() {
    let both = with("preview_link_days = 2\npreview_link_hours = 5\n").unwrap_err();
    assert_eq!(
        both,
        "site.toml, line 6, field \"preview_link_hours\": set preview_link_days or preview_link_hours, not both"
    );
}
