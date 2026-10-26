//! The private-network guard: which addresses the outbound client may reach.

use std::net::IpAddr;

use khalid_outbound::is_public;

#[cfg(test)]
fn ip(text: &str) -> IpAddr {
    text.parse().unwrap()
}

#[test]
fn public_addresses_are_allowed() {
    for text in [
        "93.184.215.14",
        "1.1.1.1",
        "8.8.8.8",
        "2606:4700:4700::1111",
        "2a00:1450:4001:80b::200e",
    ] {
        assert!(is_public(ip(text)), "{text} should be public");
    }
}

#[test]
fn loopback_is_refused() {
    for text in ["127.0.0.1", "127.255.255.254", "::1"] {
        assert!(!is_public(ip(text)), "{text}");
    }
}

#[test]
fn link_local_is_refused_including_the_cloud_metadata_address() {
    for text in ["169.254.169.254", "169.254.0.1", "fe80::1", "febf::1"] {
        assert!(!is_public(ip(text)), "{text}");
    }
}

#[test]
fn rfc_1918_private_ranges_are_refused() {
    for text in [
        "10.0.0.1",
        "10.255.255.255",
        "172.16.0.1",
        "172.31.255.255",
        "192.168.0.1",
        "192.168.255.255",
    ] {
        assert!(!is_public(ip(text)), "{text}");
    }
    for text in ["172.15.255.255", "172.32.0.0", "11.0.0.0", "192.169.0.0"] {
        assert!(
            is_public(ip(text)),
            "{text} sits just outside a private range"
        );
    }
}

#[test]
fn ipv6_unique_local_addresses_are_refused() {
    for text in ["fc00::1", "fd12:3456:789a::1", "fdff:ffff::1"] {
        assert!(!is_public(ip(text)), "{text}");
    }
}

#[test]
fn ipv4_hidden_inside_ipv6_is_checked_as_ipv4() {
    for text in [
        "::ffff:127.0.0.1",
        "::ffff:10.0.0.1",
        "::ffff:169.254.169.254",
    ] {
        assert!(!is_public(ip(text)), "{text}");
    }
    assert!(is_public(ip("::ffff:93.184.215.14")));
}

#[test]
fn other_special_ranges_are_refused() {
    for text in [
        "0.0.0.0",
        "0.1.2.3",
        "100.64.0.1",
        "100.127.255.255",
        "192.0.0.1",
        "192.0.2.1",
        "198.18.0.1",
        "198.51.100.1",
        "203.0.113.1",
        "224.0.0.1",
        "240.0.0.1",
        "255.255.255.255",
        "::",
        "ff02::1",
        "2001:db8::1",
        "64:ff9b::7f00:1",
    ] {
        assert!(!is_public(ip(text)), "{text}");
    }
}
