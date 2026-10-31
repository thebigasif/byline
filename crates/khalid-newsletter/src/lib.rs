//! Subscribers and newsletters (plan section 5.5, KH-GRO-2, task 5.5).
//!
//! A reader signs up with a plain form, gets one email with a confirmation
//! link, and is a subscriber only once they open it (double opt-in). Every
//! email they get later carries a link that unsubscribes them in one click.
//! This crate owns those rules, the addresses and tokens they use, and the
//! [`SubscriberStore`] the database implements. Subscribers are visitor data:
//! a rebuild from the content files never touches them.
//!
//! It must not know about SQL, files or how mail leaves the machine. The
//! store lives in `khalid-db`, and sending goes through a transport the
//! binary picks.

mod confirmation;
mod email_address;
mod issue;
mod lists;
mod mail;
mod newsletter;
mod pages;
mod reader_pass;
mod routes;
mod segment;
mod store;
mod token;
mod welcome;

pub use confirmation::{ConfirmationLinks, NewsletterWords, SiteLinks, WordsError};
pub use email_address::{EmailAddress, ParseEmailError};
pub use issue::{Issue, PostalAddress, SendId, SendNumbers, SendTotals, Sent};
pub use lists::Lists;
#[cfg(any(test, feature = "test-util"))]
pub use mail::MemoryMailer;
pub use mail::{Mail, MailError, Mailer};
pub use newsletter::{Newsletter, NewsletterError, NewsletterSite};
pub use pages::{Page, Pages};
pub use reader_pass::{BackPath, PASS_DAYS, ReaderPass};
pub use routes::router;
pub use segment::{Labelled, Member, Recipient, Segment};
pub use store::{
    BoxFuture, Confirmed, SignUp, SignedUp, StoreError, SubscriberId, SubscriberStore, Unsubscribed,
};
pub use token::{GenerateTokenError, ParseTokenError, SubscriberToken, TokenHash};
pub use welcome::{Welcome, Welcomes};
