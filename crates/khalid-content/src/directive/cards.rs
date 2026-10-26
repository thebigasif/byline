//! One-line cards that describe something with many attributes.

use super::Attributes;
use crate::{BlockError, BlockKind, BusinessBlock};

/// Reads a business card: the name and postal address, and how to reach it.
pub(super) fn business(attributes: &Attributes) -> Result<BlockKind, BlockError> {
    attributes.only(
        "business",
        &BUSINESS_ATTRIBUTES,
        "name, type, street, locality, postcode, country, phone, email and href",
    )?;
    let name = attributes.need("name", "a business card needs name, the business")?;
    let street = attributes.need(
        "street",
        "a business card needs street, such as 12 Mill Lane",
    )?;
    let locality = attributes.need("locality", "a business card needs locality, the town")?;
    let postal_code = attributes.need("postcode", "a business card needs postcode")?;
    let country = attributes.need("country", "a business card needs country, such as GB")?;
    let business_type = attributes
        .get("type")
        .unwrap_or_else(|| "LocalBusiness".to_owned());
    let email = attributes.get("email");
    let problem = if country.len() != 2 || !country.bytes().all(|b| b.is_ascii_uppercase()) {
        "a business card's country is a two-letter code in capitals, such as GB or DE"
    } else if !business_type.starts_with(|c: char| c.is_ascii_uppercase())
        || !business_type.chars().all(|c| c.is_ascii_alphabetic())
    {
        "a business card's type is a schema.org type in one word, such as LocalBusiness or Bakery"
    } else if email
        .as_deref()
        .is_some_and(|email| !email.contains('@') || email.contains([' ', ':']))
    {
        "a business card's email is an address such as hello@example.com"
    } else {
        ""
    };
    if !problem.is_empty() {
        return Err(BlockError::Value {
            line: attributes.line,
            problem,
        });
    }
    Ok(BlockKind::Business(BusinessBlock {
        name,
        business_type,
        street,
        locality,
        postal_code,
        country,
        phone: attributes.get("phone"),
        email,
        website: attributes
            .get("href")
            .map(|href| attributes.media("business", "href", href))
            .transpose()?,
    }))
}

const BUSINESS_ATTRIBUTES: [&str; 9] = [
    "name", "type", "street", "locality", "postcode", "country", "phone", "email", "href",
];
