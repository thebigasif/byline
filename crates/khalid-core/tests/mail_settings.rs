//! `[mail]` in `site.toml`: who mail comes from and the one way it is
//! sent, SMTP, Resend or Postmark.

use std::path::Path;

use khalid_core::{Config, Environment, MailTransport, SmtpSecurity};

const SITE: &str = r#"name = "Field Notes on SEO"
address = "https://www.example.com"
default_language = "en"
languages = ["en"]
"#;

const SMTP: &str = r#"
[mail]
from = "news@example.com"
name = "Field Notes"

[mail.smtp]
host = "smtp.example.com"
username = "news@example.com"
"#;

#[cfg(test)]
fn load(text: &str) -> Result<Config, String> {
    Config::from_toml(Path::new("site.toml"), text, &Environment::empty())
        .map_err(|error| error.to_string())
}

#[test]
fn a_site_without_the_table_sends_no_mail() {
    let config = load(SITE).unwrap();
    assert_eq!(config.mail(), None);
}

#[test]
fn reads_smtp_with_its_defaults() {
    let config = load(&format!("{SITE}{SMTP}")).unwrap();
    let mail = config.mail().unwrap();
    assert_eq!(mail.from, "news@example.com");
    assert_eq!(mail.name.as_deref(), Some("Field Notes"));
    let MailTransport::Smtp(smtp) = &mail.transport else {
        panic!("not SMTP: {mail:?}");
    };
    assert_eq!(smtp.host, "smtp.example.com");
    assert_eq!(smtp.port, 587);
    assert_eq!(smtp.security, SmtpSecurity::StartTls);
    assert_eq!(smtp.username.as_deref(), Some("news@example.com"));
    assert!(!smtp.private_host);
}

#[test]
fn reads_resend_and_postmark() {
    let resend = format!("{SITE}\n[mail]\nfrom = \"news@example.com\"\n\n[mail.resend]\n");
    let config = load(&resend).unwrap();
    assert_eq!(config.mail().unwrap().transport, MailTransport::Resend);
    let postmark = format!("{SITE}\n[mail]\nfrom = \"news@example.com\"\n\n[mail.postmark]\n");
    let config = load(&postmark).unwrap();
    let MailTransport::Postmark(settings) = &config.mail().unwrap().transport else {
        panic!("not Postmark");
    };
    assert_eq!(settings.message_stream, "outbound");
}

#[test]
fn refuses_none_or_two_ways_to_send() {
    let none = format!("{SITE}\n[mail]\nfrom = \"news@example.com\"\n");
    let two = format!("{SITE}{SMTP}\n[mail.resend]\n");
    for text in [none, two] {
        assert_eq!(
            load(&text).unwrap_err(),
            "site.toml, line 7, field \"from\": mail is sent one way, so add exactly one of [mail.smtp], [mail.resend] or [mail.postmark]",
        );
    }
}

#[test]
fn refuses_a_from_that_is_not_an_address() {
    for from in [
        "news",
        "news@",
        "@example.com",
        "news@example",
        "a b@example.com",
    ] {
        let text = format!("{SITE}{SMTP}")
            .replace("\"news@example.com\"\nname", &format!("{from:?}\nname"));
        assert_eq!(
            load(&text).unwrap_err(),
            "site.toml, line 7, field \"from\": from is the address mail comes from, such as news@example.com",
            "from {from}"
        );
    }
}

#[test]
fn refuses_a_name_on_two_lines() {
    let text = format!("{SITE}{SMTP}").replace("\"Field Notes\"", "\"Field\\nNotes\"");
    assert_eq!(
        load(&text).unwrap_err(),
        "site.toml, line 8, field \"name\": the sender's name is one line of up to 100 characters",
    );
}

#[test]
fn plain_smtp_only_to_a_private_host() {
    let plain =
        format!("{SITE}{SMTP}").replace("[mail.smtp]\n", "[mail.smtp]\nsecurity = \"none\"\n");
    assert_eq!(
        load(&plain).unwrap_err(),
        "site.toml, line 11, field \"security\": mail without encryption may only go to a host on your own network, so set private_host = true or use starttls",
    );
    let private = plain.replace("[mail.smtp]\n", "[mail.smtp]\nprivate_host = true\n");
    let config = load(&private).unwrap();
    let MailTransport::Smtp(smtp) = &config.mail().unwrap().transport else {
        panic!("not SMTP");
    };
    assert_eq!(smtp.security, SmtpSecurity::Plain);
    assert!(smtp.private_host);
}
