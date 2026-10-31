//! `[visitor_log]` in `site.toml`: the visitor log stays off until the site
//! owner switches it on, and keeps its rows 7 to 14 days (GEO C1, D5).

use std::path::Path;

use khalid_core::{Config, Environment, VisitorLogSettings};

const SITE: &str = r#"name = "Field Notes on SEO"
address = "https://www.example.com"
default_language = "en"
languages = ["en"]
"#;

#[cfg(test)]
fn load(text: &str) -> Result<Config, String> {
    Config::from_toml(Path::new("site.toml"), text, &Environment::default())
        .map_err(|error| error.to_string())
}

#[test]
fn a_site_without_the_table_keeps_no_log() {
    let log = load(SITE).unwrap().visitor_log().clone();
    assert_eq!(log, VisitorLogSettings::default());
    assert!(!log.on);
    assert_eq!(log.keep_days, 14);
}

#[test]
fn reads_every_key() {
    let text = format!("{SITE}\n[visitor_log]\non = true\nkeep_days = 7\n");
    let config = load(&text).unwrap();
    assert!(config.visitor_log().on);
    assert_eq!(config.visitor_log().keep_days, 7);
}

#[test]
fn refuses_days_outside_seven_to_fourteen() {
    for days in [0, 6, 15, 90] {
        let text = format!("{SITE}\n[visitor_log]\non = true\nkeep_days = {days}\n");
        assert_eq!(
            load(&text).unwrap_err(),
            "site.toml, line 8, field \"keep_days\": the visitor log keeps its rows 7 to 14 days, so use a number from 7 to 14",
            "{days} days"
        );
    }
}

#[test]
fn refuses_the_log_on_a_static_site() {
    let text = format!("mode = \"static\"\n{SITE}\n[visitor_log]\non = true\n");
    assert_eq!(
        load(&text).unwrap_err(),
        "site.toml, line 8, field \"on\": a static host sends the pages without Khalid, so there are no visits to log; set on = false, or mode = \"server\""
    );
}

#[test]
fn refuses_an_unknown_key() {
    let text = format!("{SITE}\n[visitor_log]\nkeep_addresses = true\n");
    let error = load(&text).unwrap_err();
    assert!(error.contains("keep_addresses"), "{error}");
}
