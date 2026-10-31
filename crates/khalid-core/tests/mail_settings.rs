//! `[mail]` in `site.toml`: who mail comes from and the one way it is
//! sent, SMTP, Resend or Postmark, with its secret from the environment.

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

/// A value made up for these tests, never a real password.
const MADE_UP: &str = "made-for-this-test-password";

#[cfg(test)]
fn load(text: &str, env: &Environment) -> Result<Config, String> {
    Config::from_toml(Path::new("site.toml"), text, env).map_err(|error| error.to_string())
}

#[cfg(test)]
fn with(name: &str) -> Environment {
    Environment::from_pairs(&[(name, MADE_UP)])
}

#[test]
fn a_site_without_the_table_sends_no_mail() {
    let config = load(SITE, &Environment::empty()).unwrap();
    assert_eq!(config.mail(), None);
}

#[test]
fn reads_smtp_with_its_defaults() {
    let config = load(&format!("{SITE}{SMTP}"), &with(Environment::SMTP_PASSWORD)).unwrap();
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
    let config = load(&resend, &with(Environment::RESEND_API_KEY)).unwrap();
    assert_eq!(config.mail().unwrap().transport, MailTransport::Resend);
    let postmark = format!("{SITE}\n[mail]\nfrom = \"news@example.com\"\n\n[mail.postmark]\n");
    let config = load(&postmark, &with(Environment::POSTMARK_TOKEN)).unwrap();
    let MailTransport::Postmark(settings) = &config.mail().unwrap().transport else {
        panic!("not Postmark");
    };
    assert_eq!(settings.message_stream, "outbound");
}

#[test]
fn refuses_a_transport_without_its_secret() {
    let cases = [
        (
            format!("{SITE}{SMTP}"),
            12,
            "username",
            "KHALID_SMTP_PASSWORD",
        ),
        (
            format!("{SITE}\n[mail]\nfrom = \"news@example.com\"\n[mail.resend]\n"),
            7,
            "from",
            "KHALID_RESEND_API_KEY",
        ),
        (
            format!("{SITE}\n[mail]\nfrom = \"news@example.com\"\n[mail.postmark]\n"),
            7,
            "from",
            "KHALID_POSTMARK_TOKEN",
        ),
    ];
    for (text, line, field, variable) in cases {
        assert_eq!(
            load(&text, &Environment::empty()).unwrap_err(),
            format!(
                "site.toml, line {line}, field \"{field}\": this way of sending needs {variable} set in the environment"
            ),
        );
    }
}

#[test]
fn smtp_without_a_user_needs_no_password() {
    let text = format!("{SITE}{SMTP}").replace("username = \"news@example.com\"\n", "");
    let config = load(&text, &Environment::empty()).unwrap();
    let MailTransport::Smtp(smtp) = &config.mail().unwrap().transport else {
        panic!("not SMTP");
    };
    assert_eq!(smtp.username, None);
}

#[test]
fn refuses_none_or_two_ways_to_send() {
    let none = format!("{SITE}\n[mail]\nfrom = \"news@example.com\"\n");
    let two = format!("{SITE}{SMTP}\n[mail.resend]\n");
    for text in [none, two] {
        assert_eq!(
            load(&text, &with(Environment::SMTP_PASSWORD)).unwrap_err(),
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
            load(&text, &with(Environment::SMTP_PASSWORD)).unwrap_err(),
            "site.toml, line 7, field \"from\": from is the address mail comes from, such as news@example.com",
            "from {from}"
        );
    }
}

#[test]
fn refuses_a_name_on_two_lines() {
    let text = format!("{SITE}{SMTP}").replace("\"Field Notes\"", "\"Field\\nNotes\"");
    assert_eq!(
        load(&text, &with(Environment::SMTP_PASSWORD)).unwrap_err(),
        "site.toml, line 8, field \"name\": the sender's name is one line of up to 100 characters",
    );
}

#[test]
fn plain_smtp_only_to_a_private_host() {
    let plain =
        format!("{SITE}{SMTP}").replace("[mail.smtp]\n", "[mail.smtp]\nsecurity = \"none\"\n");
    assert_eq!(
        load(&plain, &with(Environment::SMTP_PASSWORD)).unwrap_err(),
        "site.toml, line 11, field \"security\": mail without encryption may only go to a host on your own network, so set private_host = true or use starttls",
    );
    let private = plain.replace("[mail.smtp]\n", "[mail.smtp]\nprivate_host = true\n");
    let config = load(&private, &with(Environment::SMTP_PASSWORD)).unwrap();
    let MailTransport::Smtp(smtp) = &config.mail().unwrap().transport else {
        panic!("not SMTP");
    };
    assert_eq!(smtp.security, SmtpSecurity::Plain);
    assert!(smtp.private_host);
}

#[test]
fn the_secret_matches_the_transport_and_never_prints() {
    let env = Environment::from_pairs(&[
        (Environment::SMTP_PASSWORD, MADE_UP),
        (Environment::RESEND_API_KEY, "made-up-resend"),
        (Environment::POSTMARK_TOKEN, "made-up-postmark"),
    ]);
    let shown = format!("{env:?}");
    for value in [MADE_UP, "made-up-resend", "made-up-postmark"] {
        assert!(!shown.contains(value), "{shown}");
    }
    let config = load(&format!("{SITE}{SMTP}"), &env).unwrap();
    let secret = env.mail_secret(&config.mail().unwrap().transport).unwrap();
    assert_eq!(secret.expose(), MADE_UP);
    assert_eq!(
        env.mail_secret(&MailTransport::Resend).unwrap().expose(),
        "made-up-resend"
    );
}
