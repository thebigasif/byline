//! An unlock cookie opens one post, only while its password stays the same,
//! and only until it runs out.

use chrono::{TimeDelta, TimeZone, Utc};
use khalid_api::unlock::{Transport, UNLOCK_DAYS, UnlockSigner};
use khalid_core::{FixedClock, PasswordHash, Secret};

const PATH: &str = "/blog/client-draft/";

#[cfg(test)]
fn hash(salt: &str) -> PasswordHash {
    PasswordHash::try_new(&format!("$argon2id$v=19$m=19456,t=2,p=1${salt}$aGFzaA")).unwrap()
}

#[cfg(test)]
fn signer(days_later: i64, key: u8) -> UnlockSigner {
    let start = Utc.with_ymd_and_hms(2026, 10, 27, 9, 0, 0).unwrap();
    let now = start + TimeDelta::days(days_later);
    UnlockSigner::new(
        &Secret::new([key; 32]),
        Transport::Https,
        FixedClock::new(now),
    )
}

/// The `name=value` a browser sends back for `set_cookie`.
#[cfg(test)]
fn sent_back(set_cookie: &str) -> String {
    set_cookie.split(';').next().unwrap().to_owned()
}

#[test]
fn the_cookie_is_kept_to_its_post_and_hidden_from_scripts() {
    let set = signer(0, 7).cookie(PATH, &hash("c2FsdA"));
    assert!(set.starts_with("khalid_unlock_"), "{set}");
    for part in [
        "; Path=/blog/client-draft/",
        "; HttpOnly",
        "; SameSite=Lax",
        "; Secure",
    ] {
        assert!(set.contains(part), "{part} in {set}");
    }
    let days = TimeDelta::days(UNLOCK_DAYS).num_seconds();
    assert!(set.contains(&format!("; Max-Age={days}")), "{set}");
    let local = UnlockSigner::new(
        &Secret::new([7; 32]),
        Transport::Http,
        FixedClock::new(Utc.with_ymd_and_hms(2026, 10, 27, 9, 0, 0).unwrap()),
    );
    assert!(!local.cookie(PATH, &hash("c2FsdA")).contains("Secure"));
}

#[test]
fn the_cookie_opens_its_post_until_it_runs_out() {
    let cookie = sent_back(&signer(0, 7).cookie(PATH, &hash("c2FsdA")));
    let header = format!("theme=light; {cookie}");
    assert!(signer(0, 7).opens(&header, PATH, &hash("c2FsdA")));
    assert!(signer(UNLOCK_DAYS - 1, 7).opens(&header, PATH, &hash("c2FsdA")));
    assert!(
        !signer(UNLOCK_DAYS, 7).opens(&header, PATH, &hash("c2FsdA")),
        "run out"
    );
}

#[test]
fn the_cookie_opens_nothing_else() {
    let cookie = sent_back(&signer(0, 7).cookie(PATH, &hash("c2FsdA")));
    assert!(
        !signer(0, 7).opens(&cookie, "/blog/other/", &hash("c2FsdA")),
        "another post"
    );
    assert!(
        !signer(0, 7).opens(&cookie, PATH, &hash("bmV3c2FsdA")),
        "a new password"
    );
    assert!(
        !signer(0, 8).opens(&cookie, PATH, &hash("c2FsdA")),
        "another site's key"
    );
    let (name, value) = cookie.split_once('=').unwrap();
    let (until, sig) = value.split_once('.').unwrap();
    let later = until.parse::<i64>().unwrap() + 86_400;
    let stretched = format!("{name}={later}.{sig}");
    assert!(
        !signer(0, 7).opens(&stretched, PATH, &hash("c2FsdA")),
        "a longer time"
    );
    for broken in ["", "khalid_unlock_x=1.zz", &format!("{name}=nonsense")] {
        assert!(
            !signer(0, 7).opens(broken, PATH, &hash("c2FsdA")),
            "{broken}"
        );
    }
}
