//! The one outbound HTTP client the commands use (RULES.md BL-RUN-8).

use khalid_outbound::{Limits, Outbound, OutboundError};

use crate::cli::Cli;

/// A PEM certificate read from the file `--test-root` names.
#[cfg(feature = "test-util")]
#[derive(Debug, Clone)]
pub(crate) struct TestRoot(Vec<u8>);

/// Reads the `--test-root` file, so a missing file stops the command
/// before it starts.
///
/// # Errors
///
/// Returns what went wrong when the file cannot be read.
#[cfg(feature = "test-util")]
pub(crate) fn read_test_root(path: &str) -> Result<TestRoot, String> {
    std::fs::read(path)
        .map(TestRoot)
        .map_err(|error| format!("{path} cannot be read: {error}"))
}

/// A client for public addresses, with the default time and size limits.
///
/// In a build with the `test-util` feature only, `--test-hosts` answers host
/// names from a table and allows loopback, so CLI tests reach a local
/// stand-in server, and `--test-root` trusts a test certificate authority
/// alone. A release build has no such flags, so the private-network guard
/// cannot be turned off.
///
/// # Errors
///
/// Returns [`OutboundError::Client`] when the TLS setup fails.
pub(crate) fn client(cli: &Cli) -> Result<Outbound, OutboundError> {
    client_within(cli, Limits::default())
}

/// A client for public addresses, as [`client`] makes, under `limits`.
///
/// # Errors
///
/// Returns [`OutboundError::Client`] when the TLS setup fails.
pub(crate) fn client_within(cli: &Cli, limits: Limits) -> Result<Outbound, OutboundError> {
    #[cfg(feature = "test-util")]
    if let Some(table) = &cli.test_hosts {
        let resolver = test_resolver(table);
        return match &cli.test_root {
            Some(TestRoot(root)) => Outbound::trusting(limits, resolver, root),
            None => Outbound::with_resolver(limits, resolver),
        };
    }
    #[cfg(not(feature = "test-util"))]
    let _ = cli;
    Outbound::new(limits)
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
