//! Resend and Postmark: one JSON POST per email with the site's key,
//! checked against a stand-in server.

use std::error::Error;

use khalid_core::{AbsoluteUrl, Secret};
use khalid_mail::{ApiMailer, SendError, Sender, Service};
use khalid_newsletter::{EmailAddress, Mail, Mailer};
use khalid_outbound::{Limits, Outbound, Reach, Resolver};
use serde_json::json;
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// A link like the ones a confirmation email holds, made up for these
/// tests.
const LINK: &str = "https://www.example.com/blog/subscribe/confirm?token=made-up";

#[cfg(test)]
fn mail() -> Mail {
    let to = EmailAddress::parse("grace@example.com").unwrap();
    Mail::new(to, "Confirm".to_owned(), format!("Open {LINK}"))
}

#[cfg(test)]
fn sender() -> Sender {
    Sender::new(
        "news@example.com".to_owned(),
        Some("Field \"Notes\"".to_owned()),
    )
}

/// A client that may reach the stand-in server on loopback.
#[cfg(test)]
fn loopback() -> Outbound {
    let resolver = Resolver::for_tests(Vec::new(), Reach::PublicAndLoopback);
    Outbound::with_resolver(Limits::default(), resolver).unwrap()
}

#[cfg(test)]
fn mailer(service: Service, key: &str, server: &MockServer, at: &str) -> ApiMailer {
    let key = Secret::new(key.to_owned());
    let endpoint = AbsoluteUrl::parse(&format!("{}{at}", server.uri())).unwrap();
    ApiMailer::new(service, key, sender(), loopback())
        .unwrap()
        .at(endpoint)
}

#[cfg(test)]
fn send_error(error: &khalid_newsletter::MailError) -> &SendError {
    error.source().unwrap().downcast_ref::<SendError>().unwrap()
}

#[tokio::test]
async fn resend_gets_one_email_with_the_bearer_key() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/emails"))
        .and(header("Authorization", "Bearer made-up-resend-key"))
        .and(body_json(json!({
            "from": "\"Field \\\"Notes\\\"\" <news@example.com>",
            "to": ["grace@example.com"],
            "subject": "Confirm",
            "text": format!("Open {LINK}"),
        })))
        .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"id":"made-up"}"#))
        .expect(1)
        .mount(&server)
        .await;
    let resend = mailer(Service::Resend, "made-up-resend-key", &server, "/emails");
    resend.send(&mail()).await.unwrap();
}

#[tokio::test]
async fn postmark_gets_its_shape_its_token_and_its_stream() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/email"))
        .and(header("X-Postmark-Server-Token", "made-up-postmark-token"))
        .and(body_json(json!({
            "From": "\"Field \\\"Notes\\\"\" <news@example.com>",
            "To": "grace@example.com",
            "Subject": "Confirm",
            "TextBody": format!("Open {LINK}"),
            "MessageStream": "broadcast",
        })))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;
    let message_stream = "broadcast".to_owned();
    let service = Service::Postmark { message_stream };
    let postmark = mailer(service, "made-up-postmark-token", &server, "/email");
    postmark.send(&mail()).await.unwrap();
}

#[tokio::test]
async fn a_refusal_names_the_status_and_never_the_link_or_the_key() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(422))
        .mount(&server)
        .await;
    let resend = mailer(Service::Resend, "made-up-resend-key", &server, "/emails");
    let error = resend.send(&mail()).await.unwrap_err();
    assert!(
        matches!(send_error(&error), SendError::Refused { status: 422 }),
        "{error:?}"
    );
    let shown = format!("{error:?} {resend:?}");
    assert!(!shown.contains("made-up-resend-key"), "{shown}");
    assert!(!shown.contains("token=made-up"), "{shown}");
}

#[tokio::test]
async fn the_private_network_guard_still_applies() {
    let server = MockServer::start().await;
    let endpoint = AbsoluteUrl::parse(&format!("{}/emails", server.uri())).unwrap();
    let public_only = Outbound::new(Limits::default()).unwrap();
    let key = Secret::new("made-up-resend-key".to_owned());
    let resend = ApiMailer::new(Service::Resend, key, sender(), public_only)
        .unwrap()
        .at(endpoint);
    let error = resend.send(&mail()).await.unwrap_err();
    assert!(
        matches!(send_error(&error), SendError::Unreachable(_)),
        "{error:?}"
    );
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[test]
fn a_sender_without_a_name_is_the_address_alone() {
    let plain = Sender::new("news@example.com".to_owned(), None);
    assert_eq!(plain.mailbox(), "news@example.com");
    let named = Sender::new("news@example.com".to_owned(), Some("A\\B".to_owned()));
    assert_eq!(named.mailbox(), "\"A\\\\B\" <news@example.com>");
}
