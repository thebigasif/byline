//! [`Money`]: an amount in a currency's smallest unit (BL-TYP-6).

use std::fmt;

use crate::SeoError;

/// Currencies with no minor unit, such as the yen: 1500 means 1500 yen.
const ZERO_DECIMAL: [&str; 12] = [
    "BIF", "CLP", "DJF", "GNF", "ISK", "JPY", "KMF", "KRW", "PYG", "UGX", "VND", "XAF",
];

/// An amount of money, held in the currency's smallest unit, such as pence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Money {
    minor: i64,
    currency: String,
}

impl Money {
    /// `minor` units of `currency`, an ISO 4217 code such as `GBP`.
    ///
    /// # Errors
    ///
    /// [`SeoError::Currency`] when the code is not three capital letters.
    pub fn new(minor: i64, currency: &str) -> Result<Self, SeoError> {
        if currency.len() == 3 && currency.chars().all(|c| c.is_ascii_uppercase()) {
            Ok(Self {
                minor,
                currency: currency.to_owned(),
            })
        } else {
            Err(SeoError::Currency {
                currency: currency.to_owned(),
            })
        }
    }

    /// The ISO 4217 code.
    pub fn currency(&self) -> &str {
        &self.currency
    }
}

/// The amount with the currency's decimals, such as `12.50` or `1500`.
impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if ZERO_DECIMAL.contains(&self.currency.as_str()) {
            return write!(f, "{}", self.minor);
        }
        let sign = if self.minor < 0 { "-" } else { "" };
        let units = self.minor.unsigned_abs();
        write!(f, "{sign}{}.{:02}", units / 100, units % 100)
    }
}
