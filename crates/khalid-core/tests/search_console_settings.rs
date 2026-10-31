//! `[search_console]` in `site.toml`: the opt-in Search Console connection
//! (KH-GRO-5), its property and who gets the weekly email, with the two
//! Google secrets read from the environment.

use std::path::Path;

use khalid_core::{Config, Environment, SearchConsoleProperty};

const SITE: &str = r#"name = "Field Notes on SEO"
address = "https://www.example.com"
default_language = "en"
languages = ["en"]
"#;

const CONNECTED: &str = r#"
[search_console]
client_id = "1234-abcd.apps.googleusercontent.com"
"#;

const MAIL: &str = r#"
[mail]
from = "news@example.com"

[mail.resend]
"#;

/// A value made up for these tests, never a real secret.
const MADE_UP: &str = "made-for-this-test-value";

#[cfg(test)]
fn load(text: &str, env: &Environment) -> Result<Config, String> {
    Config::from_toml(Path::new("site.toml"), text, env).map_err(|error| error.to_string())
}

#[test]
fn a_site_without_the_table_has_no_connection() {
    let config = load(SITE, &Environment::empty()).unwrap();
    assert_eq!(config.search_console(), None);
}

#[test]
fn the_property_is_the_site_address_unless_set() {
    let config = load(&format!("{SITE}{CONNECTED}"), &Environment::empty()).unwrap();
    let connection = config.search_console().unwrap();
    assert_eq!(connection.client_id, "1234-abcd.apps.googleusercontent.com");
    assert_eq!(connection.property.as_str(), "https://www.example.com/");
    assert_eq!(connection.report_to, None);

    let domain = format!("{SITE}{CONNECTED}property = \"sc-domain:example.com\"\n");
    let config = load(&domain, &Environment::empty()).unwrap();
    let property = &config.search_console().unwrap().property;
    assert_eq!(
        property,
        &SearchConsoleProperty::Domain("example.com".into())
    );
    assert_eq!(property.as_str(), "sc-domain:example.com");
}

#[test]
fn a_property_google_cannot_name_is_refused_on_its_line() {
    for odd in [
        "example.com",
        "sc-domain:",
        "sc-domain:exa mple.com",
        "ftp://x.com/",
    ] {
        let text = format!("{SITE}{CONNECTED}property = \"{odd}\"\n");
        let said = load(&text, &Environment::empty()).unwrap_err();
        assert!(said.contains("line 8"), "{odd}: {said}");
        assert!(said.contains("sc-domain:"), "{odd}: {said}");
    }
}

#[test]
fn the_weekly_email_needs_an_address_and_mail() {
    let text = format!("{SITE}{CONNECTED}report_to = \"owner@example.com\"\n");
    let said = load(&text, &Environment::empty()).unwrap_err();
    assert!(said.contains("[mail]"), "{said}");

    let env = Environment::from_pairs(&[(Environment::RESEND_API_KEY, MADE_UP)]);
    let config = load(&format!("{text}{MAIL}"), &env).unwrap();
    let report_to = config.search_console().unwrap().report_to.as_deref();
    assert_eq!(report_to, Some("owner@example.com"));

    let text = format!("{SITE}{CONNECTED}report_to = \"owner\"\n{MAIL}");
    let said = load(&text, &env).unwrap_err();
    assert!(said.contains("report_to"), "{said}");
}

#[test]
fn a_client_id_google_never_issued_is_refused() {
    let text = format!("{SITE}\n[search_console]\nclient_id = \"\"\n");
    let said = load(&text, &Environment::empty()).unwrap_err();
    assert!(said.contains("client_id"), "{said}");
}

#[test]
fn the_google_secrets_come_from_the_environment_and_stay_hidden() {
    let env = Environment::from_pairs(&[
        (Environment::GOOGLE_CLIENT_SECRET, MADE_UP),
        (Environment::GOOGLE_REFRESH_TOKEN, " "),
    ]);
    assert_eq!(
        env.google_client_secret().map(|s| s.expose().clone()),
        Some(MADE_UP.to_owned())
    );
    assert!(env.google_refresh_token().is_none());
    assert!(!format!("{env:?}").contains(MADE_UP));
}
