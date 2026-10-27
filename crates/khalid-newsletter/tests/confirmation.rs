//! The confirmation email: in the site's language, with a link that
//! confirms and a link that removes the address, both under the site's
//! mount path.

use khalid_core::{AbsoluteUrl, Locale};
use khalid_newsletter::{
    ConfirmationLinks, EmailAddress, Mail, MailWords, Mailer, MemoryMailer, SiteLinks,
    SubscriberToken,
};

#[cfg(test)]
fn links() -> SiteLinks {
    let address = AbsoluteUrl::parse("https://www.example.com").unwrap();
    SiteLinks::new(&address, "/blog/")
}

#[cfg(test)]
fn mail(language: &str) -> (Mail, SubscriberToken, SubscriberToken) {
    let (confirm, remove) = (
        SubscriberToken::generate().unwrap(),
        SubscriberToken::generate().unwrap(),
    );
    let to = EmailAddress::parse("grace@example.com").unwrap();
    let site = links();
    let links = ConfirmationLinks {
        site: &site,
        confirm: &confirm,
        remove: &remove,
    };
    let locale: Locale = language.parse().unwrap();
    let mail = MailWords::load()
        .unwrap()
        .confirmation(&locale, "Field Notes", to, links);
    (mail, confirm, remove)
}

#[test]
fn links_open_the_pages_under_the_mount_path() {
    let token = SubscriberToken::parse(&"a".repeat(64)).unwrap();
    let base = "https://www.example.com/blog/";
    let token_text = "a".repeat(64);
    assert_eq!(
        links().confirm(&token),
        format!("{base}subscribe/confirm?token={token_text}")
    );
    assert_eq!(
        links().unsubscribe(&token),
        format!("{base}unsubscribe?token={token_text}")
    );
    let root = SiteLinks::new(
        &AbsoluteUrl::parse("https://blog.example.com/").unwrap(),
        "/",
    );
    assert!(
        root.confirm(&token)
            .starts_with("https://blog.example.com/subscribe/confirm?")
    );
}

#[test]
fn the_email_names_the_site_and_holds_both_links() {
    let (mail, confirm, remove) = mail("en");
    assert_eq!(mail.to.as_str(), "grace@example.com");
    assert_eq!(mail.subject, "Confirm your email for Field Notes");
    let (yes, no) = (links().confirm(&confirm), links().unsubscribe(&remove));
    for part in ["get Field Notes by email", "within two days", &yes, &no] {
        assert!(mail.text.contains(part), "{part} is missing: {}", mail.text);
    }
    assert!(
        !mail.text.contains('{'),
        "a placeholder was left: {}",
        mail.text
    );
}

#[test]
fn a_german_site_writes_in_german_and_others_fall_back_to_english() {
    let (german, _, _) = mail("de");
    assert!(
        german.subject.starts_with("Bitte bestätigen"),
        "{}",
        german.subject
    );
    assert!(!german.text.contains('{'), "{}", german.text);
    let (french, _, _) = mail("fr");
    assert_eq!(french.subject, "Confirm your email for Field Notes");
}

#[tokio::test]
async fn the_fake_transport_keeps_what_it_is_given_or_refuses_everything() {
    let (mail, _, _) = mail("en");
    let kept = MemoryMailer::default();
    kept.send(&mail).await.unwrap();
    assert_eq!(kept.sent(), std::slice::from_ref(&mail));
    let down = MemoryMailer::refusing();
    assert!(down.send(&mail).await.is_err());
    assert_eq!(down.sent(), []);
}
