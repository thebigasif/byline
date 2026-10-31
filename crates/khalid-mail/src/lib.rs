//! How a site's mail leaves (task 5.6, KH-GRO-2): through an SMTP server,
//! or through Resend's or Postmark's API, as `[mail]` in `site.toml` says.
//!
//! Each way is a [`khalid_newsletter::Mailer`], so the newsletter, sign-in
//! links and anything else that sends mail call one trait and never learn
//! which way was picked. The APIs are reached through the one shared
//! [`khalid_outbound::Outbound`] client. A mail's text holds working
//! links, so nothing here logs it (RULES.md BL-RUN-5).

mod api;
mod send_error;
mod sender;

pub use api::{ApiMailer, Service};
pub use send_error::SendError;
pub use sender::Sender;
