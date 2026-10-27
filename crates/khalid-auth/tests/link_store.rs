//! A sign-in link carries its token after `#`, and every link store lets
//! a link be used once, within 15 minutes. The SQLite store in `khalid-db`
//! runs the same cases.

use chrono::{DateTime, TimeDelta, TimeZone, Utc};
use khalid_auth::{AccountId, LINK_LIFETIME, LinkStore, LinkToken, MemoryLinks};

#[cfg(test)]
fn start() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 27, 9, 0, 0).unwrap()
}

#[test]
fn a_link_points_into_the_admin_with_the_token_after_the_hash() {
    let token = LinkToken::generate().unwrap();
    let link = token.link("https://blog.example.com/admin/");
    let text = link.expose();
    let rest = text
        .strip_prefix("https://blog.example.com/admin/sign-in/link#token=")
        .unwrap();
    assert_eq!(rest.len(), 64, "{text}");
    assert_eq!(LinkToken::parse(rest).unwrap().hash(), token.hash());
    assert!(LinkToken::parse("not-a-token").is_err());
    assert!(
        !format!("{token:?}").contains(rest),
        "Debug hides the token"
    );
    let without_slash = token.link("https://blog.example.com/admin");
    assert_eq!(without_slash.expose(), text);
}

#[tokio::test]
async fn a_link_is_used_once_and_only_before_it_expires() {
    let links = MemoryLinks::new();
    let mia = AccountId::from_row(2);
    let (fresh, stale, unknown) = (
        LinkToken::generate().unwrap(),
        LinkToken::generate().unwrap(),
        LinkToken::generate().unwrap(),
    );
    links.create(fresh.hash(), mia, start()).await.unwrap();
    links
        .create(stale.hash(), mia, start() - LINK_LIFETIME)
        .await
        .unwrap();
    let later = start() + TimeDelta::minutes(1);
    assert_eq!(links.redeem(fresh.hash(), later).await.unwrap(), Some(mia));
    assert_eq!(
        links.redeem(fresh.hash(), later).await.unwrap(),
        None,
        "used once"
    );
    assert_eq!(
        links.redeem(stale.hash(), start()).await.unwrap(),
        None,
        "expired"
    );
    assert_eq!(links.redeem(unknown.hash(), later).await.unwrap(), None);
}
