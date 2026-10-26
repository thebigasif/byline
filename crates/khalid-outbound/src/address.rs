//! Which addresses the outbound client may reach.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// Whether `ip` is a public internet address the outbound client may connect
/// to.
///
/// It refuses loopback, private (RFC 1918), link-local (including the cloud
/// metadata address `169.254.169.254`), shared, documentation, benchmarking,
/// multicast, reserved and unspecified IPv4 ranges, and the IPv6 loopback,
/// unique-local, link-local, multicast, documentation and unspecified ranges.
/// IPv4 carried inside IPv6, as mapped (`::ffff:a.b.c.d`) or NAT64
/// (`64:ff9b::a.b.c.d`) addresses, is checked as IPv4. The client checks every
/// address a host name resolves to, so a name that points inward is refused
/// too.
///
/// # Examples
///
/// ```
/// use std::net::IpAddr;
///
/// use khalid_outbound::is_public;
///
/// assert!(is_public(IpAddr::from([93, 184, 215, 14])));
/// assert!(!is_public(IpAddr::from([169, 254, 169, 254])));
/// ```
pub fn is_public(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_public_v4(v4),
        IpAddr::V6(v6) => is_public_v6(v6),
    }
}

fn is_public_v4(ip: Ipv4Addr) -> bool {
    let [a, b, c, _] = ip.octets();
    let special = ip.is_unspecified()
        || ip.is_loopback()
        || ip.is_private()
        || ip.is_link_local()
        || ip.is_broadcast()
        || ip.is_documentation()
        || ip.is_multicast()
        // 0.0.0.0/8, "this network" (RFC 791).
        || a == 0
        // 100.64.0.0/10, shared address space for carrier NAT (RFC 6598).
        || (a == 100 && (64..=127).contains(&b))
        // 192.0.0.0/24, protocol assignments (RFC 6890).
        || (a == 192 && b == 0 && c == 0)
        // 198.18.0.0/15, benchmarking (RFC 2544).
        || (a == 198 && (18..=19).contains(&b))
        // 240.0.0.0/4, reserved (RFC 1112).
        || a >= 240;
    !special
}

fn is_public_v6(ip: Ipv6Addr) -> bool {
    if let Some(v4) = ip.to_ipv4_mapped() {
        return is_public_v4(v4);
    }
    let segments = ip.segments();
    // 64:ff9b::/96, NAT64 (RFC 6052): the last 32 bits are an IPv4 address.
    if let [0x64, 0xff9b, 0, 0, 0, 0, high, low] = segments {
        return is_public_v4(Ipv4Addr::from((u32::from(high) << 16) | u32::from(low)));
    }
    let special = ip.is_unspecified()
        || ip.is_loopback()
        || ip.is_unique_local()
        || ip.is_unicast_link_local()
        || ip.is_multicast()
        // 2001:db8::/32, documentation (RFC 3849).
        || matches!(segments, [0x2001, 0x0db8, ..]);
    !special
}
