//! A page rendered for a reader who has not answered, made for the reader
//! who said no and the reader who said yes.

use axum::http::{HeaderMap, HeaderValue, header};
use khalid_server::{Choice, HeldCode, chosen};

const PAGE: &str = "<head><!--khalid-held--><script nonce=\"s\">post()</script><!--/khalid-held--><!--khalid-site-head--></head><body><p>Text.</p><!--khalid-site-body--><!--khalid-ask--><aside>Allow?</aside><!--/khalid-ask--><!--khalid-chosen--><template><aside>Change</aside></template><!--/khalid-chosen--></body>";

#[cfg(test)]
fn held() -> HeldCode {
    HeldCode::new("<script>site()</script>", "<script>chat()</script>")
}

#[cfg(test)]
fn with_cookie(cookie: &str) -> Choice {
    let mut headers = HeaderMap::new();
    headers.insert(header::COOKIE, HeaderValue::from_str(cookie).unwrap());
    Choice::of(&headers)
}

#[test]
fn the_cookie_names_the_choice() {
    assert_eq!(Choice::of(&HeaderMap::new()), Choice::Unasked);
    assert_eq!(with_cookie("a=1; khalid-consent=yes"), Choice::Agreed);
    assert_eq!(with_cookie("khalid-consent=no"), Choice::Declined);
    assert_eq!(with_cookie("khalid-consent=maybe"), Choice::Unasked);
    assert_eq!(with_cookie("not-khalid-consent=yes"), Choice::Unasked);
}

#[test]
fn a_reader_who_has_not_answered_gets_the_banner_and_no_code() {
    let html = chosen(PAGE, Choice::Unasked, &held());
    assert_eq!(
        html,
        "<head><!--khalid-site-head--></head><body><p>Text.</p><!--khalid-site-body--><!--khalid-ask--><aside>Allow?</aside><!--/khalid-ask--><!--khalid-chosen--><template><aside>Change</aside></template><!--/khalid-chosen--></body>"
    );
}

#[test]
fn a_reader_who_said_no_gets_the_change_form_and_no_code() {
    let html = chosen(PAGE, Choice::Declined, &held());
    assert_eq!(
        html,
        "<head><!--khalid-site-head--></head><body><p>Text.</p><!--khalid-site-body--><aside>Change</aside></body>"
    );
}

#[test]
fn a_reader_who_agreed_gets_every_piece_of_code() {
    let html = chosen(PAGE, Choice::Agreed, &held());
    assert_eq!(
        html,
        "<head><!--khalid-held--><script nonce=\"s\">post()</script><!--/khalid-held--><script>site()</script></head><body><p>Text.</p><script>chat()</script><aside>Change</aside></body>"
    );
}

#[test]
fn held_code_never_closed_is_cut_to_the_end() {
    let html = chosen(
        "<p>a</p><!--khalid-held--><script>x()",
        Choice::Unasked,
        &held(),
    );
    assert_eq!(html, "<p>a</p>");
}
