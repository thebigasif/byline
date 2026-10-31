//! [`Offer`]: a price for something a post sells or lists.

use khalid_core::AbsoluteUrl;

use crate::{Money, SchemaNode, SchemaValue};

/// Whether the thing can be bought now, as schema.org's `ItemAvailability`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum Availability {
    /// It can be bought now.
    #[default]
    InStock,
    /// It cannot be bought now.
    OutOfStock,
    /// It can be ordered before it is out.
    PreOrder,
}

impl Availability {
    /// The schema.org address of the value, such as `https://schema.org/InStock`.
    fn address(self) -> &'static str {
        match self {
            Self::InStock => "https://schema.org/InStock",
            Self::OutOfStock => "https://schema.org/OutOfStock",
            Self::PreOrder => "https://schema.org/PreOrder",
        }
    }
}

/// A price, whether it can be bought now, and where.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Offer {
    /// The price; a free thing costs 0.
    pub price: Money,
    /// Whether it can be bought now; in stock unless set.
    pub availability: Availability,
    /// The page to buy it on, if there is one.
    pub url: Option<AbsoluteUrl>,
}

impl Offer {
    /// An offer at `price`, in stock, with no page to buy it on.
    pub fn new(price: Money) -> Self {
        Self {
            price,
            availability: Availability::InStock,
            url: None,
        }
    }

    /// The `Offer` node, its price written with the currency's decimals.
    pub(crate) fn node(&self) -> SchemaNode {
        let mut node = SchemaNode::new("Offer")
            .with("price", SchemaValue::Text(self.price.to_string()))
            .with("priceCurrency", SchemaValue::text(self.price.currency()))
            .with(
                "availability",
                SchemaValue::text(self.availability.address()),
            );
        if let Some(url) = &self.url {
            node.set("url", SchemaValue::Url(url.clone()));
        }
        node
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{Availability, Offer};
    use crate::Money;

    #[test]
    fn an_offer_writes_its_price_currency_and_availability() {
        let mut offer = Offer::new(Money::parse("1500", "JPY").unwrap());
        offer.availability = Availability::PreOrder;
        let want = json!({
            "@type": "Offer",
            "price": "1500",
            "priceCurrency": "JPY",
            "availability": "https://schema.org/PreOrder"
        });
        assert_eq!(offer.node().to_json(), want);
    }
}
