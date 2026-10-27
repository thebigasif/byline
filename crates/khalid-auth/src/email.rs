//! A staff member's email address, the name they sign in with.

use std::fmt;
use std::str::FromStr;

/// The longest address SMTP can carry (RFC 5321, section 4.5.3.1.3).
const MAX_LEN: usize = 254;

/// An email address, trimmed and lower-cased, so `Ana@Example.com` and
/// `ana@example.com` are one account. It is personal data: never log it
/// (RULES.md BL-RUN-5).
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Email(String);

impl Email {
    /// The address as stored.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Email {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Email(<redacted>)")
    }
}

impl FromStr for Email {
    type Err = ParseEmailError;

    /// Reads an address with one `@`, something before it, a domain with a
    /// dot after it, and no spaces. Khalid proves an address by sending a
    /// link to it, so this check only catches typing mistakes.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let address = text.trim().to_lowercase();
        let Some((local, domain)) = address.split_once('@') else {
            return Err(ParseEmailError::NoAt);
        };
        let domain_ok = domain.contains('.')
            && !domain.starts_with('.')
            && !domain.ends_with('.')
            && !domain.contains('@');
        if local.is_empty() || !domain_ok {
            return Err(ParseEmailError::Incomplete);
        }
        if address.chars().any(char::is_whitespace) || address.len() > MAX_LEN {
            return Err(ParseEmailError::Incomplete);
        }
        Ok(Self(address))
    }
}

/// Text that is not an email address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ParseEmailError {
    /// There is no `@`.
    #[error("an email address needs an @")]
    NoAt,
    /// A part is missing, there is a space, or it is too long.
    #[error("the email address is not complete")]
    Incomplete,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addresses_are_trimmed_and_lower_cased() {
        let email: Email = "  Ana@Example.COM ".parse().expect("an address");
        assert_eq!(email.as_str(), "ana@example.com");
    }

    #[test]
    fn broken_addresses_are_refused() {
        assert_eq!(
            "ana.example.com".parse::<Email>(),
            Err(ParseEmailError::NoAt)
        );
        for text in [
            "@example.com",
            "ana@",
            "ana@example",
            "ana@.com",
            "a b@example.com",
            "a@b@c.com",
        ] {
            assert_eq!(
                text.parse::<Email>(),
                Err(ParseEmailError::Incomplete),
                "{text}"
            );
        }
        let long = format!("{}@example.com", "a".repeat(250));
        assert_eq!(long.parse::<Email>(), Err(ParseEmailError::Incomplete));
    }

    #[test]
    fn debug_hides_the_address() {
        let email: Email = "ana@example.com".parse().expect("an address");
        assert!(!format!("{email:?}").contains("ana"));
    }
}
