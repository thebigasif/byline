//! A sign-up carries when its confirmation link stops working and how long
//! another confirmation email is held back.

use chrono::{TimeDelta, TimeZone, Utc};
use khalid_core::Slug;
use khalid_newsletter::{EmailAddress, SignUp, SubscriberToken};

#[test]
fn a_link_works_two_days_and_a_second_email_waits_ten_minutes() {
    let at = Utc.with_ymd_and_hms(2026, 10, 27, 9, 0, 0).unwrap();
    let email = EmailAddress::parse("grace@example.com").unwrap();
    let token = SubscriberToken::generate().unwrap();
    let sign_up = SignUp::new(email, Slug::try_new("main").unwrap(), token.hash(), at);
    assert_eq!(sign_up.expires_at - at, TimeDelta::days(2));
    assert_eq!(at - sign_up.quiet_since, TimeDelta::minutes(10));
    assert_eq!(sign_up.confirm, token.hash());
}
