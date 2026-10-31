//! Name resolution with the private-network guard.

use std::collections::BTreeMap;
use std::io;
use std::net::IpAddr;
use std::sync::Arc;

use crate::address::is_public;

/// Which addresses a client may reach.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reach {
    /// Public internet addresses only. The choice for every client but one.
    Public,
    /// Public addresses and loopback, for the one client `khalid audit`
    /// crawls a running blog with, such as `khalid serve` on this machine.
    /// Every other inward range stays refused (ADR "the audit may reach
    /// the site under audit on loopback").
    SiteUnderAudit,
    /// Public addresses and loopback, so tests can talk to a local stand-in
    /// server. Every other inward range stays refused.
    #[cfg(any(test, feature = "test-util"))]
    PublicAndLoopback,
}

impl Reach {
    /// Whether `ip` may be reached.
    pub fn allows(self, ip: IpAddr) -> bool {
        match self {
            Self::Public => is_public(ip),
            Self::SiteUnderAudit => is_public(ip) || ip.is_loopback(),
            #[cfg(any(test, feature = "test-util"))]
            Self::PublicAndLoopback => is_public(ip) || ip.is_loopback(),
        }
    }
}

/// Turns host names into addresses, and refuses a name when any address it
/// resolves to points inward, so a name with one public and one private
/// answer cannot be used to reach the private one.
#[derive(Debug, Clone)]
pub struct Resolver {
    hosts: Option<Arc<BTreeMap<String, Vec<IpAddr>>>>,
    reach: Reach,
}

impl Resolver {
    /// A resolver that asks the system and allows public addresses only.
    pub fn new() -> Self {
        Self {
            hosts: None,
            reach: Reach::Public,
        }
    }

    /// A resolver that asks the system and allows public addresses and
    /// loopback, for the client `khalid audit` crawls a running blog with
    /// and for nothing else.
    pub fn for_site_under_audit() -> Self {
        Self {
            hosts: None,
            reach: Reach::SiteUnderAudit,
        }
    }

    /// A resolver that answers only from `hosts`, never from real DNS.
    #[cfg(any(test, feature = "test-util"))]
    pub fn for_tests(hosts: Vec<(String, Vec<IpAddr>)>, reach: Reach) -> Self {
        Self {
            hosts: Some(Arc::new(hosts.into_iter().collect())),
            reach,
        }
    }

    /// Which addresses this resolver allows.
    pub fn reach(&self) -> Reach {
        self.reach
    }

    /// Every address `host` resolves to, once each has passed the guard.
    ///
    /// # Errors
    ///
    /// Returns [`ResolveError::Blocked`] naming the first address that points
    /// inward, and [`ResolveError::Lookup`] when the name cannot be resolved.
    pub async fn lookup(&self, host: &str) -> Result<Vec<IpAddr>, ResolveError> {
        let addresses: Vec<IpAddr> = match &self.hosts {
            Some(table) => table.get(host).cloned().unwrap_or_default(),
            None => tokio::net::lookup_host((host, 0))
                .await
                .map_err(|source| ResolveError::Lookup {
                    host: host.to_owned(),
                    source,
                })?
                .map(|address| address.ip())
                .collect(),
        };
        if let Some(&address) = addresses.iter().find(|&&ip| !self.reach.allows(ip)) {
            return Err(ResolveError::Blocked {
                host: host.to_owned(),
                address,
            });
        }
        if addresses.is_empty() {
            return Err(ResolveError::Lookup {
                host: host.to_owned(),
                source: io::Error::from(io::ErrorKind::NotFound),
            });
        }
        Ok(addresses)
    }
}

impl Default for Resolver {
    fn default() -> Self {
        Self::new()
    }
}

/// Why a host name was not resolved.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ResolveError {
    /// The name resolves to an address that is not public.
    #[error("{host} resolves to {address}, which is not a public address")]
    Blocked {
        /// The host name.
        host: String,
        /// The first address the guard refused.
        address: IpAddr,
    },
    /// The name could not be resolved.
    #[error("{host} could not be found")]
    Lookup {
        /// The host name.
        host: String,
        /// What the resolver reported.
        #[source]
        source: io::Error,
    },
}
