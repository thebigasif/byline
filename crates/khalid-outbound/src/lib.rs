//! The one way Khalid talks to other servers (RULES.md BL-RUN-8).
//!
//! This crate owns outbound HTTP: theme sync, bookmark cards, imports,
//! webhooks and search engine pings all go through it. Its guard refuses private,
//! loopback and link-local addresses, so a site address or a pasted link can
//! never make Khalid reach into the network it runs in. The one exception is
//! the client `khalid audit` crawls a running blog with, which may reach
//! loopback and still nothing else inward. It must not know about content,
//! rendering or storage.

mod address;
mod outbound;
mod resolver;

pub use address::is_public;
pub use outbound::{Fetched, Limits, Outbound, OutboundError};
pub use resolver::{Reach, ResolveError, Resolver};
