//! A session token is 256 random bits, kept by the store only as a hash,
//! carried in a locked-down cookie, with a CSRF token derived from it.

use std::fmt::Write as _;

use chrono::{TimeDelta, TimeZone, Utc};
use khalid_auth::{
    AccountId, CLEAR_COOKIE, COOKIE, IDLE, LIFETIME, SessionError, SessionToken, StoredSession,
};

#[cfg(test)]
fn token() -> SessionToken {
    SessionToken::generate().expect("a random source")
}

#[test]
fn two_tokens_differ_and_neither_shows_in_debug() {
    let (first, second) = (token(), token());
    assert_ne!(first.hash(), second.hash());
    let cookie = first.set_cookie();
    let secret = cookie
        .expose()
        .strip_prefix("__Host-khalid-session=")
        .and_then(|rest| rest.split(';').next())
        .expect("the cookie holds the token");
    assert_eq!(secret.len(), 64, "256 bits as hex");
    assert!(
        !format!("{first:?}").contains(secret),
        "Debug hides the token"
    );
}

#[test]
fn the_cookie_is_host_only_secure_http_only_and_lax() {
    let cookie = token().set_cookie();
    let cookie = cookie.expose();
    assert!(cookie.starts_with(&format!("{COOKIE}=")));
    for part in [
        "Path=/",
        "Secure",
        "HttpOnly",
        "SameSite=Lax",
        "Max-Age=1209600",
    ] {
        assert!(cookie.split("; ").any(|p| p == part), "{part} in {cookie}");
    }
    assert!(
        !cookie.contains("Domain"),
        "a __Host- cookie names no domain"
    );
    assert!(CLEAR_COOKIE.contains("Max-Age=0"));
}

#[test]
fn the_token_is_read_back_from_a_cookie_header_among_others() {
    let token = token();
    let cookie = token.set_cookie();
    let pair = cookie.expose().split(';').next().expect("name=value");
    let header = format!("theme=light; {pair}; other=1");
    let read = SessionToken::from_cookie_header(&header).expect("found");
    assert_eq!(read.hash(), token.hash());
    assert!(SessionToken::from_cookie_header("theme=light").is_none());
    let forged = format!("{COOKIE}=not-a-token");
    assert!(SessionToken::from_cookie_header(&forged).is_none());
}

#[test]
fn only_sixty_four_lowercase_hex_digits_are_a_token() {
    assert!("0".repeat(64).parse::<SessionToken>().is_ok());
    let refused = [("0", 63), ("0", 65), ("A", 64), ("g", 64)]
        .map(|(digit, count)| digit.repeat(count).parse::<SessionToken>().err());
    assert_eq!(refused, [Some(SessionError::NotAToken); 4]);
}

#[test]
fn the_stored_hash_is_not_itself_a_token() {
    let token = token();
    let leaked = token
        .hash()
        .as_bytes()
        .iter()
        .fold(String::new(), |mut text, byte| {
            write!(text, "{byte:02x}").expect("writing to a String");
            text
        });
    let replayed: SessionToken = leaked.parse().expect("looks like a token");
    assert_ne!(
        replayed.hash(),
        token.hash(),
        "a copied database row signs nobody in"
    );
}

#[test]
fn the_csrf_token_belongs_to_one_session_and_is_compared_whole() {
    let (first, second) = (token(), token());
    let csrf = first.csrf();
    assert!(csrf.matches(csrf.as_str()));
    assert!(!csrf.matches(second.csrf().as_str()));
    assert!(!csrf.matches(""));
    let prefix = csrf.as_str().get(..32).expect("64 hex digits");
    assert!(!csrf.matches(prefix), "a prefix is not the token");
    let cookie = first.set_cookie();
    assert!(
        !cookie.expose().contains(csrf.as_str()),
        "the CSRF token is not the session token"
    );
}

#[test]
fn a_session_ends_fourteen_days_after_sign_in_or_twelve_hours_unused() {
    let start = Utc
        .with_ymd_and_hms(2026, 10, 27, 9, 0, 0)
        .single()
        .expect("a time");
    let session = StoredSession {
        account: AccountId::from_row(1),
        created_at: start,
        last_seen_at: start,
    };
    assert!(session.is_live(start + IDLE - TimeDelta::seconds(1)));
    assert!(!session.is_live(start + IDLE));
    let used = StoredSession {
        last_seen_at: start + LIFETIME - TimeDelta::hours(1),
        ..session
    };
    assert!(used.is_live(start + LIFETIME - TimeDelta::seconds(1)));
    assert!(
        !used.is_live(start + LIFETIME),
        "use does not stretch the lifetime"
    );
}
