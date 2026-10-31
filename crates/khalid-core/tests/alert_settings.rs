//! `[alerts]` in `site.toml`: who gets alerts by email, and when a burst
//! of 404s or an unchanged post is raised (KH-GRO-6).

use std::path::Path;

use khalid_core::{AlertSettings, Config, Environment};

const SITE: &str = r#"name = "Field Notes on SEO"
address = "https://www.example.com"
default_language = "en"
languages = ["en"]
"#;

const MAIL: &str = r#"
[mail]
from = "news@example.com"

[mail.resend]
"#;

#[cfg(test)]
fn load(text: &str) -> Result<Config, String> {
    // A value made up for these tests, never a real key.
    let env = Environment::from_pairs(&[(Environment::RESEND_API_KEY, "made-for-this-test")]);
    Config::from_toml(Path::new("site.toml"), text, &env).map_err(|error| error.to_string())
}

#[test]
fn a_site_without_the_table_gets_the_defaults_and_no_email() {
    let alerts = load(SITE).unwrap().alerts().clone();
    assert_eq!(alerts, AlertSettings::default());
    assert_eq!(alerts.email, None);
    assert_eq!(alerts.not_found_spike, 50);
    assert_eq!(alerts.stale_after_days, 365);
}

#[test]
fn reads_every_key() {
    let text = format!(
        "{SITE}\n[alerts]\nemail = \"owner@example.com\"\nnot_found_spike = 10\nstale_after_days = 0\n{MAIL}"
    );
    let config = load(&text).unwrap();
    let alerts = config.alerts();
    assert_eq!(alerts.email.as_deref(), Some("owner@example.com"));
    assert_eq!(alerts.not_found_spike, 10);
    assert_eq!(alerts.stale_after_days, 0);
}

#[test]
fn refuses_an_email_without_mail_to_send_it() {
    let text = format!("{SITE}\n[alerts]\nemail = \"owner@example.com\"\n");
    assert_eq!(
        load(&text).unwrap_err(),
        "site.toml, line 7, field \"email\": alerts are emailed through [mail], so add a [mail] table or remove email"
    );
}

#[test]
fn refuses_an_email_that_is_not_an_address() {
    let text = format!("{SITE}\n[alerts]\nemail = \"owner\"\n{MAIL}");
    assert_eq!(
        load(&text).unwrap_err(),
        "site.toml, line 7, field \"email\": email is the address alerts go to, such as owner@example.com"
    );
}

#[test]
fn refuses_a_spike_of_nothing_and_an_unknown_key() {
    let zero = format!("{SITE}\n[alerts]\nnot_found_spike = 0\n");
    assert_eq!(
        load(&zero).unwrap_err(),
        "site.toml, line 7, field \"not_found_spike\": a spike is at least one new 404 in an hour, so use 1 or more"
    );
    let unknown = format!("{SITE}\n[alerts]\nevery = 5\n");
    assert!(load(&unknown).unwrap_err().contains("every"), "unknown key");
}
