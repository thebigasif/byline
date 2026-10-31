//! One row of the visitor log: what it keeps of a visitor, and what it
//! never keeps (GEO C1, `docs/design/visitor-log.md`).

use std::net::IpAddr;

use khalid_analytics::{BotCheck, Crawler, CrawlerKind, ShortAddress, VisitorCode, VisitorKey};

#[cfg(test)]
fn ip(text: &str) -> IpAddr {
    text.parse().unwrap()
}

#[test]
fn an_ipv4_address_keeps_its_first_three_parts() {
    let short = ShortAddress::of(ip("203.0.113.57"));
    assert_eq!(short.to_string(), "203.0.113.0");
    assert_eq!(short, ShortAddress::of(ip("203.0.113.200")));
}

#[test]
fn an_ipv6_address_keeps_its_first_48_bits() {
    let short = ShortAddress::of(ip("2001:db8:85a3:8d3:1319:8a2e:370:7348"));
    assert_eq!(short.to_string(), "2001:db8:85a3::");
}

#[test]
fn an_ipv4_address_inside_ipv6_is_shortened_as_ipv4() {
    let short = ShortAddress::of(ip("::ffff:198.51.100.23"));
    assert_eq!(short.to_string(), "198.51.100.0");
}

#[test]
fn a_shortened_address_reads_back_as_written() {
    for text in ["203.0.113.0", "2001:db8:85a3::"] {
        let short: ShortAddress = text.parse().unwrap();
        assert_eq!(short.to_string(), text);
    }
    assert!("203.0.113.57".parse::<ShortAddress>().is_err());
    assert!("2001:db8:85a3::1".parse::<ShortAddress>().is_err());
    assert!("not an address".parse::<ShortAddress>().is_err());
}

#[test]
fn one_visitor_gets_one_code_for_the_day() {
    let key = VisitorKey::from_bytes([7; 32]);
    let agent = Some("Mozilla/5.0 (X11; Linux x86_64) Firefox/131.0");
    let first = key.code(ip("203.0.113.57"), agent);
    assert_eq!(first, key.code(ip("203.0.113.57"), agent));
    assert_eq!(first.to_string().len(), 16);
}

#[test]
fn another_address_or_browser_gets_another_code() {
    let key = VisitorKey::from_bytes([7; 32]);
    let agent = Some("Mozilla/5.0 Firefox/131.0");
    let first = key.code(ip("203.0.113.57"), agent);
    // Same shortened address, another visitor.
    assert_ne!(first, key.code(ip("203.0.113.58"), agent));
    assert_ne!(
        first,
        key.code(ip("203.0.113.57"), Some("Mozilla/5.0 Chrome/130.0"))
    );
    assert_ne!(first, key.code(ip("203.0.113.57"), None));
}

#[test]
fn a_new_key_makes_codes_that_do_not_match_the_old_ones() {
    let agent = Some("Mozilla/5.0 Firefox/131.0");
    let today = VisitorKey::from_bytes([7; 32]).code(ip("203.0.113.57"), agent);
    let tomorrow = VisitorKey::from_bytes([8; 32]).code(ip("203.0.113.57"), agent);
    assert_ne!(today, tomorrow);
}

#[test]
fn a_code_reads_back_as_written() {
    let code = VisitorKey::from_bytes([1; 32]).code(ip("192.0.2.1"), None);
    let read: VisitorCode = code.to_string().parse().unwrap();
    assert_eq!(read, code);
    assert!("9f3c".parse::<VisitorCode>().is_err());
    assert!("zz3c2a71d05be448".parse::<VisitorCode>().is_err());
}

#[test]
fn the_key_never_shows_in_debug_output() {
    let key = VisitorKey::from_bytes([0xAB; 32]);
    let shown = format!("{key:?}");
    assert!(!shown.to_lowercase().contains("ab, ab"), "{shown}");
    assert!(!shown.contains("171"), "{shown}");
}

#[test]
fn a_bot_check_reads_back_as_stored() {
    for check in BotCheck::ALL {
        assert_eq!(BotCheck::from_stored(check.as_str()), Some(check));
    }
    assert_eq!(BotCheck::from_stored("maybe"), None);
}

#[test]
fn a_known_bot_is_found_by_its_stored_name() {
    let gptbot = Crawler::named("GPTBot").unwrap();
    assert_eq!(gptbot.kind(), CrawlerKind::Ai);
    assert_eq!(
        Crawler::named("Other bot").map(Crawler::kind),
        Some(CrawlerKind::Other)
    );
    assert_eq!(Crawler::named("Made-up bot"), None);
}
