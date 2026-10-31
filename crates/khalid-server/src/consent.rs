//! The site's own code held back until a reader agrees to it, and the
//! reader's choice read from a cookie (KH-SEO-12, ADR "consent is a form
//! and a cookie").
//!
//! A page is rendered once, for a reader who has not answered: it holds
//! the banner between [`ASK_START`] and [`ASK_END`], a smaller form to
//! change the choice inside an inert `template` between [`CHOSEN_START`]
//! and [`CHOSEN_END`], a post's own code between [`HELD_START`] and
//! [`HELD_END`] when it has some, and the two places the site's own code
//! goes, [`SITE_HEAD_SLOT`] and [`SITE_BODY_SLOT`]. A reader who answered
//! gets the page changed for them alone: the banner cut, the smaller form
//! shown, and the code put in only when they agreed.

use axum::http::{HeaderMap, header};

/// The cookie that keeps a reader's choice: `yes` or `no`.
pub const CONSENT_COOKIE: &str = "khalid-consent";

/// Where the banner starts. It is cut once a reader has chosen.
pub const ASK_START: &str = "<!--khalid-ask-->";

/// Where the banner ends.
pub const ASK_END: &str = "<!--/khalid-ask-->";

/// Where the form to change a choice starts, inside a `template` no
/// browser shows. Once a reader has chosen, the two marks go and the form
/// shows.
pub const CHOSEN_START: &str = "<!--khalid-chosen--><template>";

/// Where the form to change a choice ends.
pub const CHOSEN_END: &str = "</template><!--/khalid-chosen-->";

/// Where a post's own code starts when it waits for consent. It is cut
/// for every reader who has not agreed.
pub const HELD_START: &str = "<!--khalid-held-->";

/// Where a post's own code ends.
pub const HELD_END: &str = "<!--/khalid-held-->";

/// Where the site's head code goes, for a reader who agreed.
pub const SITE_HEAD_SLOT: &str = "<!--khalid-site-head-->";

/// Where the site's body code goes, for a reader who agreed.
pub const SITE_BODY_SLOT: &str = "<!--khalid-site-body-->";

/// The site's own code that waits for consent, with
/// [`NONCE_SLOT`](crate::NONCE_SLOT) on each `script` and `style`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HeldCode {
    head: String,
    body: String,
}

impl HeldCode {
    /// The site's `head` and `body` code, already checked and written with
    /// the nonce slot.
    pub fn new(head: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            head: head.into(),
            body: body.into(),
        }
    }
}

/// What a reader chose about the site's code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choice {
    /// No answer yet: the page as it was rendered, banner and all.
    Unasked,
    /// No: no code, and the smaller form to change it.
    Declined,
    /// Yes: the code, and the smaller form to change it.
    Agreed,
}

impl Choice {
    /// The choice the `Cookie` headers of `request` hold. Anything other
    /// than `yes` or `no` counts as no answer.
    pub fn of(request: &HeaderMap) -> Self {
        let pairs = request
            .get_all(header::COOKIE)
            .iter()
            .filter_map(|value| value.to_str().ok())
            .flat_map(|cookies| cookies.split(';'));
        for pair in pairs {
            match pair.trim().split_once('=') {
                Some((CONSENT_COOKIE, "yes")) => return Self::Agreed,
                Some((CONSENT_COOKIE, "no")) => return Self::Declined,
                _ => {}
            }
        }
        Self::Unasked
    }
}

/// `html`, a page rendered for a reader who has not answered, as the reader
/// who made `choice` sees it, with `held`'s code put in when they agreed.
///
/// The router applies it to every HTML page; a page drawn outside the
/// router, such as a page of search results, may apply it too.
pub fn chosen(html: &str, choice: Choice, held: &HeldCode) -> String {
    let mut html = match choice {
        Choice::Unasked => return cut(html, HELD_START, HELD_END),
        Choice::Declined => cut(&cut(html, ASK_START, ASK_END), HELD_START, HELD_END),
        Choice::Agreed => cut(html, ASK_START, ASK_END)
            .replacen(SITE_HEAD_SLOT, &held.head, 1)
            .replacen(SITE_BODY_SLOT, &held.body, 1),
    };
    for mark in [CHOSEN_START, CHOSEN_END] {
        html = html.replacen(mark, "", 1);
    }
    html
}

/// `html` without each part from `start` to `end`, marks included. A part
/// never closed is cut to the end, so held code never runs by mistake.
fn cut(html: &str, start: &str, end: &str) -> String {
    let mut kept = String::with_capacity(html.len());
    let mut rest = html;
    while let Some((before, after)) = rest.split_once(start) {
        kept.push_str(before);
        rest = after.split_once(end).map_or("", |(_, after)| after);
    }
    kept.push_str(rest);
    kept
}
