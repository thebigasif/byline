//! Alerts for a site's owner (KH-GRO-6): broken links, bursts of 404s,
//! certificates near their end, scheduled posts that did not go live, and
//! posts left unchanged for a long time, and from the speed and uptime
//! watch (C6), a site that stopped answering or answers slowly.
//!
//! A check turns what it reads into [`Finding`]s, each with a key that
//! stays the same while the problem lasts. The findings of one kind not
//! raised before make one [`Alert`], which says what happened, why it
//! matters and what to do, for an email or the `alert.raised` webhook.
//! Sending is the binary's; this crate reads nothing and sends nothing.

mod alert;
mod checks;
mod raised;
mod watch;

pub use alert::{Alert, Finding, Kind};
pub use checks::{LivePost, ServedCertificate, certificate, not_found_spike, stale_posts};
#[cfg(any(test, feature = "test-util"))]
pub use raised::MemoryRaised;
pub use raised::{BoxFuture, Raised, RaisedError, fresh};
pub use watch::{Answer, Probe, site_down, slow_site};
