//! The one outbound HTTP client the commands use (RULES.md BL-RUN-8).

use khalid_outbound::{Limits, Outbound, OutboundError};

use crate::cli::Cli;

/// A client for public addresses, with the default time and size limits.
///
/// In a build with the `test-util` feature only, `--test-hosts` answers host
/// names from a table and allows loopback, so CLI tests reach a local
/// stand-in server. A release build has no such flag, so the private-network
/// guard cannot be turned off.
///
/// # Errors
///
/// Returns [`OutboundError::Client`] when the TLS setup fails.
pub(crate) fn client(cli: &Cli) -> Result<Outbound, OutboundError> {
    #[cfg(feature = "test-util")]
    if let Some(table) = &cli.test_hosts {
        return Outbound::with_resolver(Limits::default(), test_resolver(table));
    }
    #[cfg(not(feature = "test-util"))]
    let _ = cli;
    Outbound::new(Limits::default())
}

/// A resolver that answers from `table`, pairs of `host=address` split by
/// commas, and allows loopback. A pair it cannot read is left out.
#[cfg(feature = "test-util")]
fn test_resolver(table: &str) -> khalid_outbound::Resolver {
    let hosts = table
        .split(',')
        .filter_map(|pair| pair.split_once('='))
        .filter_map(|(host, address)| Some((host.to_owned(), vec![address.parse().ok()?])))
        .collect();
    khalid_outbound::Resolver::for_tests(hosts, khalid_outbound::Reach::PublicAndLoopback)
}
