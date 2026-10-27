//! Signing in and out with a password, how long a session lasts, and what
//! a stolen session cookie can and cannot do.

use std::sync::{Arc, Mutex};

use chrono::{DateTime, TimeDelta, TimeZone, Utc};
use khalid_api::keys::Role;
use khalid_auth::{
    Account, AccountStore, Accounts, Email, FRESH, IDLE, MemoryAccounts, MemorySessions, Password,
    PasswordHashing, SessionStore, SessionToken, SignInError,
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

impl Hands {
    #[cfg(test)]
    fn advance(&self, by: TimeDelta) {
        *self.0.lock().expect("lock poisoned") += by;
    }
}

struct Site {
    accounts: Accounts,
    sessions: Arc<MemorySessions>,
    clock: Arc<Hands>,
    owner: Account,
}

#[cfg(test)]
fn password(text: &str) -> Password {
    Password::new(text.to_owned()).expect("long enough")
}

#[cfg(test)]
fn email(text: &str) -> Email {
    text.parse().expect("an email address")
}

/// A site whose owner, Asif, has the password "correct horse battery".
#[cfg(test)]
async fn site() -> Site {
    let start = Utc
        .with_ymd_and_hms(2026, 10, 27, 9, 0, 0)
        .single()
        .expect("a time");
    let clock = Arc::new(Hands(Mutex::new(start)));
    let store = Arc::new(MemoryAccounts::new());
    let sessions = Arc::new(MemorySessions::new());
    let hashing = PasswordHashing::fast_for_tests();
    let owner = store
        .create(&email("asif@example.com"), "Asif", Role::Owner, start)
        .await
        .expect("made");
    let hash = hashing
        .hash(password("correct horse battery"))
        .await
        .expect("hashed");
    store.set_password(owner.id(), &hash).await.expect("stored");
    let shared_sessions: Arc<dyn SessionStore> = Arc::<MemorySessions>::clone(&sessions);
    let shared_clock: Arc<dyn Clock> = Arc::<Hands>::clone(&clock);
    let accounts = Accounts::new(store, shared_sessions, hashing, shared_clock);
    Site {
        accounts,
        sessions,
        clock,
        owner,
    }
}

#[cfg(test)]
async fn sign_in(site: &Site) -> SessionToken {
    let (token, signed_in) = site
        .accounts
        .sign_in(
            &email("asif@example.com"),
            password("correct horse battery"),
            None,
        )
        .await
        .expect("signed in");
    assert_eq!(signed_in.account, site.owner);
    token
}

#[tokio::test]
async fn the_right_password_starts_a_session_that_finds_its_account() {
    let site = site().await;
    let token = sign_in(&site).await;
    let checked = site.accounts.session(&token).await.expect("live");
    assert_eq!(checked.account, site.owner);
    assert!(checked.csrf.matches(token.csrf().as_str()));
}

#[tokio::test]
async fn a_wrong_password_and_an_unknown_email_get_the_same_refusal() {
    let site = site().await;
    let wrong = site
        .accounts
        .sign_in(
            &email("asif@example.com"),
            password("incorrect horse"),
            None,
        )
        .await;
    let unknown = site
        .accounts
        .sign_in(
            &email("nobody@example.com"),
            password("correct horse battery"),
            None,
        )
        .await;
    assert!(matches!(wrong, Err(SignInError::Refused)), "{wrong:?}");
    assert!(matches!(unknown, Err(SignInError::Refused)), "{unknown:?}");
    assert!(site.sessions.is_empty());
}

#[tokio::test]
async fn after_five_failures_even_the_right_password_waits() {
    let site = site().await;
    let asif = email("asif@example.com");
    for _ in 0..5 {
        let _ = site
            .accounts
            .sign_in(&asif, password("incorrect horse"), None)
            .await;
    }
    let right = site
        .accounts
        .sign_in(&asif, password("correct horse battery"), None)
        .await;
    assert!(
        matches!(right, Err(SignInError::Limited { minutes: 16 })),
        "{right:?}"
    );
    site.clock.advance(TimeDelta::minutes(16));
    sign_in(&site).await;
}

#[tokio::test]
async fn a_session_unused_for_twelve_hours_is_ended() {
    let site = site().await;
    let token = sign_in(&site).await;
    site.clock.advance(IDLE);
    let result = site.accounts.session(&token).await;
    assert!(matches!(result, Err(SignInError::Refused)), "{result:?}");
    assert!(site.sessions.is_empty(), "the expired row is removed");
}

#[tokio::test]
async fn signing_out_ends_only_that_session() {
    let site = site().await;
    let (laptop, phone) = (sign_in(&site).await, sign_in(&site).await);
    site.accounts.sign_out(&laptop).await.expect("signed out");
    let result = site.accounts.session(&laptop).await;
    assert!(matches!(result, Err(SignInError::Refused)), "{result:?}");
    assert!(site.accounts.session(&phone).await.is_ok());
}

#[tokio::test]
async fn a_removed_account_signs_nobody_in() {
    let site = site().await;
    let token = sign_in(&site).await;
    site.accounts
        .store()
        .remove(site.owner.id())
        .await
        .expect("removed");
    let result = site.accounts.session(&token).await;
    assert!(matches!(result, Err(SignInError::Refused)), "{result:?}");
}

#[tokio::test]
async fn a_stolen_cookie_stops_working_when_the_owner_signs_out_everywhere() {
    let site = site().await;
    let stolen = sign_in(&site).await;
    let mine = sign_in(&site).await;
    assert!(
        site.accounts.session(&stolen).await.is_ok(),
        "a copied cookie works at first"
    );
    let ended = site
        .accounts
        .sign_out_everywhere(&mine)
        .await
        .expect("signed out");
    assert_eq!(ended, 2);
    let after = site.accounts.session(&stolen).await;
    assert!(matches!(after, Err(SignInError::Refused)), "{after:?}");
}

#[tokio::test]
async fn a_new_password_ends_every_other_session_and_needs_the_old_one() {
    let site = site().await;
    let stolen = sign_in(&site).await;
    let mine = sign_in(&site).await;
    let thief = site
        .accounts
        .change_password(&stolen, None, password("the thief's own choice"))
        .await;
    assert!(
        matches!(thief, Err(SignInError::Refused)),
        "a cookie alone cannot change it"
    );
    site.accounts
        .change_password(
            &mine,
            Some(password("correct horse battery")),
            password("a brand new passphrase"),
        )
        .await
        .expect("changed");
    assert!(
        site.accounts.session(&mine).await.is_ok(),
        "this session stays"
    );
    assert!(
        site.accounts.session(&stolen).await.is_err(),
        "the other one ended"
    );
}

#[tokio::test]
async fn a_first_password_may_be_set_only_soon_after_signing_in() {
    let site = site().await;
    let writer = site
        .accounts
        .store()
        .create(
            &email("mia@example.com"),
            "Mia",
            Role::Writer,
            site.clock.now(),
        )
        .await
        .expect("made");
    let late = site.accounts.begin(&writer).await.expect("began");
    site.clock.advance(FRESH);
    let fresh = site.accounts.begin(&writer).await.expect("began");
    let refused = site
        .accounts
        .change_password(&late, None, password("too late for this"))
        .await;
    assert!(matches!(refused, Err(SignInError::Refused)), "{refused:?}");
    site.accounts
        .change_password(&fresh, None, password("chosen right after the link"))
        .await
        .expect("set");
    let signed_in = site
        .accounts
        .sign_in(
            &email("mia@example.com"),
            password("chosen right after the link"),
            None,
        )
        .await;
    assert!(signed_in.is_ok(), "{signed_in:?}");
}
