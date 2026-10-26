//! A host name is refused when any address it resolves to points inward.
//! Names resolve through a test table, never through real DNS.

use std::net::IpAddr;

use khalid_outbound::{Reach, ResolveError, Resolver};

/// Looks up `name` in a table that maps it to `ips`, and returns the address
/// the guard refused.
#[cfg(test)]
async fn refused(name: &str, ips: &[&str]) -> String {
    let table = vec![(
        name.to_owned(),
        ips.iter().map(|ip| ip.parse().unwrap()).collect(),
    )];
    match Resolver::for_tests(table, Reach::Public).lookup(name).await {
        Err(ResolveError::Blocked { host, address }) => {
            assert_eq!(host, name);
            address.to_string()
        }
        other => panic!("{name} was not refused: {other:?}"),
    }
}

#[tokio::test]
async fn a_host_name_that_resolves_to_loopback_is_refused() {
    assert_eq!(
        refused("intranet.example", &["127.0.0.1"]).await,
        "127.0.0.1"
    );
    assert_eq!(refused("intranet6.example", &["::1"]).await, "::1");
}

#[tokio::test]
async fn a_host_name_that_resolves_to_link_local_is_refused() {
    assert_eq!(
        refused("metadata.example", &["169.254.169.254"]).await,
        "169.254.169.254"
    );
    assert_eq!(refused("link6.example", &["fe80::1"]).await, "fe80::1");
}

#[tokio::test]
async fn a_host_name_that_resolves_to_an_rfc_1918_address_is_refused() {
    for ip in ["10.0.0.8", "172.16.4.2", "192.168.1.1"] {
        assert_eq!(refused("office.example", &[ip]).await, ip);
    }
}

#[tokio::test]
async fn a_host_name_that_resolves_to_an_ipv6_unique_local_address_is_refused() {
    assert_eq!(refused("nas.example", &["fd00::1"]).await, "fd00::1");
}

#[tokio::test]
async fn a_host_name_with_one_inward_address_among_public_ones_is_refused() {
    assert_eq!(
        refused("rebind.example", &["93.184.215.14", "10.0.0.1"]).await,
        "10.0.0.1"
    );
}

#[tokio::test]
async fn a_host_name_with_only_public_addresses_resolves() {
    let ips: Vec<IpAddr> = vec![
        "93.184.215.14".parse().unwrap(),
        "2606:4700::1".parse().unwrap(),
    ];
    let resolver =
        Resolver::for_tests(vec![("example.com".to_owned(), ips.clone())], Reach::Public);
    assert_eq!(resolver.lookup("example.com").await.unwrap(), ips);
}

#[tokio::test]
async fn a_name_with_no_addresses_is_not_found() {
    let resolver = Resolver::for_tests(Vec::new(), Reach::Public);
    let error = resolver.lookup("missing.example").await.unwrap_err();
    assert!(matches!(error, ResolveError::Lookup { .. }), "{error:?}");
}

#[tokio::test]
async fn test_reach_allows_loopback_and_nothing_else_inward() {
    let table = vec![
        (
            "local.example".to_owned(),
            vec!["127.0.0.1".parse().unwrap()],
        ),
        (
            "office.example".to_owned(),
            vec!["10.0.0.1".parse().unwrap()],
        ),
    ];
    let resolver = Resolver::for_tests(table, Reach::PublicAndLoopback);
    assert!(resolver.lookup("local.example").await.is_ok());
    assert!(matches!(
        resolver.lookup("office.example").await,
        Err(ResolveError::Blocked { .. })
    ));
}
