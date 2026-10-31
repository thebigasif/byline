//! SMTP: one plain-text email per connection, to a host the guard allows
//! or one `site.toml` marks as private, checked against a stand-in server.

use std::error::Error;
use std::net::{IpAddr, Ipv4Addr, TcpListener};

use khalid_core::{SmtpSecurity, SmtpSettings};
use khalid_mail::{SendError, Sender, SmtpMailer, StubSmtp};
use khalid_newsletter::{EmailAddress, Mail, MailError, Mailer};
use khalid_outbound::{Reach, Resolver};

/// A link like the ones a confirmation email holds, made up for these
/// tests.
const LINK: &str = "https://www.example.com/blog/subscribe/confirm?token=made-up";

#[cfg(test)]
fn mail() -> Mail {
    let to = EmailAddress::parse("grace@example.com").unwrap();
    Mail::new(to, "Confirm".to_owned(), format!("Open {LINK}"))
}

/// A relay at `127.0.0.1:port` that `site.toml` marks as private.
#[cfg(test)]
fn private_relay(port: u16) -> SmtpSettings {
    toml::from_str(&format!(
        "host = \"127.0.0.1\"\nport = {port}\nsecurity = \"none\"\nprivate_host = true\n"
    ))
    .unwrap()
}

/// `smtp.example.com` on `port`, a public host as far as `site.toml` says.
#[cfg(test)]
fn public_host(port: u16) -> SmtpSettings {
    toml::from_str(&format!(
        "host = \"smtp.example.com\"\nport = {port}\nsecurity = \"none\"\n"
    ))
    .unwrap()
}

#[cfg(test)]
fn mailer(settings: SmtpSettings, resolver: Resolver) -> SmtpMailer {
    let sender = Sender::new(
        "news@example.com".to_owned(),
        Some("Field Notes".to_owned()),
    );
    SmtpMailer::new(settings, None, sender, resolver)
}

/// A resolver that answers `smtp.example.com` with loopback, allowed or
/// not by `reach`.
#[cfg(test)]
fn resolver(reach: Reach) -> Resolver {
    let loopback = IpAddr::V4(Ipv4Addr::LOCALHOST);
    Resolver::for_tests(vec![("smtp.example.com".to_owned(), vec![loopback])], reach)
}

#[cfg(test)]
fn send_error(error: &MailError) -> &SendError {
    error.source().unwrap().downcast_ref::<SendError>().unwrap()
}

#[tokio::test]
async fn a_private_host_gets_one_plain_text_email() {
    let stub = StubSmtp::start().unwrap();
    let smtp = mailer(private_relay(stub.port()), Resolver::new());
    smtp.send(&mail()).await.unwrap();
    let received = stub.received();
    assert_eq!(received.len(), 1, "{received:?}");
    let message = &received[0];
    for line in [
        "From: \"Field Notes\" <news@example.com>",
        "To: grace@example.com",
        "Subject: Confirm",
        "Content-Type: text/plain; charset=utf-8",
    ] {
        assert!(message.contains(line), "{line} in {message}");
    }
    let body = message.replace("=\n", "").replace("=3D", "=");
    assert!(body.contains(LINK), "{message}");
}

#[tokio::test]
async fn a_public_host_is_looked_up_through_the_guard() {
    let stub = StubSmtp::start().unwrap();
    let host = public_host(stub.port());
    let refused = mailer(host.clone(), resolver(Reach::Public));
    let error = refused.send(&mail()).await.unwrap_err();
    assert!(
        matches!(send_error(&error), SendError::Resolve(_)),
        "{error:?}"
    );
    assert_eq!(stub.received(), Vec::<String>::new());
    let allowed = mailer(host, resolver(Reach::PublicAndLoopback));
    allowed.send(&mail()).await.unwrap();
    assert_eq!(stub.received().len(), 1);
}

#[tokio::test]
async fn starttls_never_falls_back_to_plain_text() {
    let stub = StubSmtp::start().unwrap();
    let mut settings = private_relay(stub.port());
    settings.security = SmtpSecurity::StartTls;
    let error = mailer(settings, Resolver::new())
        .send(&mail())
        .await
        .unwrap_err();
    assert!(
        matches!(send_error(&error), SendError::Smtp(_)),
        "{error:?}"
    );
    assert_eq!(stub.received(), Vec::<String>::new());
}

#[tokio::test]
async fn a_server_that_is_down_is_an_smtp_error_without_the_text() {
    let closed = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = closed.local_addr().unwrap().port();
    drop(closed);
    let smtp = mailer(private_relay(port), Resolver::new());
    let error = smtp.send(&mail()).await.unwrap_err();
    assert!(
        matches!(send_error(&error), SendError::Smtp(_)),
        "{error:?}"
    );
    assert!(!format!("{error:?}").contains("token=made-up"));
}
