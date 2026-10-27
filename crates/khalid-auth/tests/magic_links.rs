//! Signing in by a link sent by email: one link per request, only to a
//! real account, used once, within 15 minutes.

use std::sync::{Arc, Mutex};

use chrono::{DateTime, TimeDelta, TimeZone, Utc};
use khalid_api::keys::Role;
use khalid_auth::{
    AccountStore, Accounts, Email, LINK_LIFETIME, LinkToken, Links, MemoryAccounts, MemoryLinks,
    MemoryMailbox, MemorySessions, PasswordHashing, SignInError,
};
use khalid_core::Clock;

/// A clock the test moves by hand.
#[derive(Debug)]
struct Hands(Mutex<DateTime<Utc>>);

impl Clock for Hands {
    #[expect(clippy::expect_used, reason = "a test clock")]
    fn now(&self) -> DateTime<Utc> {
        *self.0.lock().expect("lock poisoned")
    }
}

struct Site {
    accounts: Accounts,
    mailbox: Arc<MemoryMailbox>,
    clock: Arc<Hands>,
}

#[cfg(test)]
fn email(text: &str) -> Email {
    text.parse().unwrap()
}

/// A site with one writer, Mia, who has no password.
#[cfg(test)]
async fn site() -> Site {
    let start = Utc.with_ymd_and_hms(2026, 10, 27, 9, 0, 0).unwrap();
    let clock = Arc::new(Hands(Mutex::new(start)));
    let store = Arc::new(MemoryAccounts::new());
    store
        .create(&email("mia@example.com"), "Mia", Role::Writer, start)
        .await
        .unwrap();
    let mailbox = Arc::new(MemoryMailbox::new());
    let shared_clock: Arc<dyn Clock> = Arc::<Hands>::clone(&clock);
    let links = Links {
        store: Arc::new(MemoryLinks::new()),
        sender: Arc::<MemoryMailbox>::clone(&mailbox),
        admin: "https://blog.example.com/admin/".to_owned(),
    };
    let accounts = Accounts::new(
        store,
        Arc::new(MemorySessions::new()),
        PasswordHashing::fast_for_tests(),
        shared_clock,
    )
    .with_links(links);
    Site {
        accounts,
        mailbox,
        clock,
    }
}

/// The token in the last link mailed.
#[cfg(test)]
fn last_token(site: &Site) -> LinkToken {
    let sent = site.mailbox.sent();
    let (_, link) = sent.last().unwrap();
    let token = link
        .strip_prefix("https://blog.example.com/admin/sign-in/link#token=")
        .unwrap();
    LinkToken::parse(token).unwrap()
}

#[tokio::test]
async fn a_link_goes_only_to_an_account_and_signs_it_in_once() {
    let site = site().await;
    site.accounts
        .send_link(&email("nobody@example.com"), None)
        .await
        .unwrap();
    assert!(
        site.mailbox.sent().is_empty(),
        "no mail to an email with no account"
    );
    site.accounts
        .send_link(&email("Mia@example.com"), None)
        .await
        .unwrap();
    assert_eq!(site.mailbox.sent().len(), 1);
    let token = last_token(&site);
    let (session, signed_in) = site.accounts.redeem_link(&token).await.unwrap();
    assert_eq!(signed_in.account.email().as_str(), "mia@example.com");
    assert!(site.accounts.session(&session).await.is_ok());
    let again = site.accounts.redeem_link(&token).await;
    assert!(
        matches!(again, Err(SignInError::Refused)),
        "used once: {again:?}"
    );
}

#[tokio::test]
async fn a_link_stops_working_after_fifteen_minutes() {
    let site = site().await;
    site.accounts
        .send_link(&email("mia@example.com"), None)
        .await
        .unwrap();
    let token = last_token(&site);
    *site.clock.0.lock().unwrap() += LINK_LIFETIME;
    let late = site.accounts.redeem_link(&token).await;
    assert!(matches!(late, Err(SignInError::Refused)), "{late:?}");
}

#[tokio::test]
async fn a_made_up_token_signs_nobody_in() {
    let site = site().await;
    let made_up = LinkToken::generate().unwrap();
    let result = site.accounts.redeem_link(&made_up).await;
    assert!(matches!(result, Err(SignInError::Refused)), "{result:?}");
}

#[tokio::test]
async fn nobody_can_flood_an_inbox_with_links() {
    let site = site().await;
    let mia = email("mia@example.com");
    for _ in 0..5 {
        site.accounts.send_link(&mia, None).await.unwrap();
    }
    let sixth = site.accounts.send_link(&mia, None).await;
    assert!(
        matches!(sixth, Err(SignInError::Limited { .. })),
        "{sixth:?}"
    );
    assert_eq!(site.mailbox.sent().len(), 5);
    *site.clock.0.lock().unwrap() += TimeDelta::minutes(15);
    site.accounts.send_link(&mia, None).await.unwrap();
}

#[tokio::test]
async fn a_site_that_cannot_mail_says_so() {
    let start = Utc.with_ymd_and_hms(2026, 10, 27, 9, 0, 0).unwrap();
    let accounts = Accounts::new(
        Arc::new(MemoryAccounts::new()),
        Arc::new(MemorySessions::new()),
        PasswordHashing::fast_for_tests(),
        Arc::new(khalid_core::FixedClock::new(start)),
    );
    let result = accounts.send_link(&email("mia@example.com"), None).await;
    assert!(matches!(result, Err(SignInError::LinksOff)), "{result:?}");
}
