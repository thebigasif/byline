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

    /// A price as people write it, such as `49.00` or `49`, in `currency`.
    /// A currency with a minor unit takes two decimals or none; one with
    /// none, such as the yen, takes none.
    ///
    /// # Errors
    ///
    /// [`SeoError::Currency`] when the code is not three capital letters,
    /// and [`SeoError::Price`] when `price` is not a whole number with the
    /// decimals the currency has, or is too large to hold.
    pub fn parse(price: &str, currency: &str) -> Result<Self, SeoError> {
        let refused = || SeoError::Price {
            price: price.to_owned(),
        };
        let digits = |text: &str| !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit());
        let (units, cents) = price.split_once('.').unwrap_or((price, ""));
        let whole = ZERO_DECIMAL.contains(&currency);
        let cents_fit = if whole {
            price == units
        } else {
            price == units || (cents.len() == 2 && digits(cents))
        };
        if !digits(units) || !cents_fit {
            return Err(refused());
        }
        let units: i64 = units.parse().map_err(|_| refused())?;
        let cents: i64 = if cents.is_empty() {
            0
        } else {
            cents.parse().map_err(|_| refused())?
        };
        let minor = if whole {
            Some(units)
        } else {
            units.checked_mul(100).and_then(|m| m.checked_add(cents))
        };
        Self::new(minor.ok_or_else(refused)?, currency)
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

#[cfg(test)]
mod tests {
    use super::Money;

    #[test]
    fn prices_read_as_written_round_trip() {
        let cases = [
            ("49.00", "GBP", "49.00"),
            ("49", "USD", "49.00"),
            ("0.99", "EUR", "0.99"),
            ("1500", "JPY", "1500"),
        ];
        for (price, currency, written) in cases {
            let money = Money::parse(price, currency).unwrap();
            assert_eq!(money.to_string(), written, "{price} {currency}");
        }
    }

    #[test]
    fn prices_with_the_wrong_decimals_or_signs_are_refused() {
        let cases = [
            ("49.5", "GBP"),
            ("49.000", "GBP"),
            ("-1.00", "GBP"),
            ("1,000", "GBP"),
            ("", "GBP"),
            ("1500.00", "JPY"),
            ("99999999999999999999", "GBP"),
            ("49.00", "gbp"),
        ];
        for (price, currency) in cases {
            assert!(Money::parse(price, currency).is_err(), "{price} {currency}");
        }
    }
}
