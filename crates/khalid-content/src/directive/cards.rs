//! Cards: blocks drawn as a box from their attributes.

use khalid_core::Slug;

use super::Attributes;
use crate::{BlockError, BlockKind, BusinessBlock, Visibility};

/// Reads a snippet's place: the slug of the snippet shown there.
pub(super) fn snippet(attributes: &Attributes) -> Result<BlockKind, BlockError> {
    attributes.only("snippet", &["slug"], "slug")?;
    let slug = attributes.need(
        "slug",
        "a snippet needs slug, the name of a file in snippets/",
    )?;
    if slug.parse::<Slug>().is_err() {
        return Err(BlockError::Value {
            line: attributes.line,
            problem: "a snippet's slug is written in small letters, digits and hyphens",
        });
    }
    Ok(BlockKind::Snippet { slug })
}

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

/// Reads a header: an optional picture, which then needs alt text.
pub(super) fn header(attributes: &Attributes) -> Result<BlockKind, BlockError> {
    attributes.only("header", &["image", "alt"], "image and alt")?;
    let image = attributes
        .get("image")
        .map(|image| attributes.media("header", "image", image))
        .transpose()?;
    let alt = attributes.get("alt");
    if image.is_some() && alt.is_none() {
        return Err(BlockError::Missing {
            line: attributes.line,
            problem: "a header's picture needs alt, the text that describes it to people who cannot see it",
        });
    }
    Ok(BlockKind::Header { image, alt })
}

/// Reads an HTML block: who sees it, everyone when not written.
pub(super) fn html(attributes: &Attributes) -> Result<BlockKind, BlockError> {
    attributes.only("html", &["show"], "show")?;
    let show = match attributes.get("show").as_deref() {
        None | Some("everyone") => Visibility::Everyone,
        Some("visitors") => Visibility::Visitors,
        Some("members") => Visibility::Members,
        Some(_) => {
            return Err(BlockError::Value {
                line: attributes.line,
                problem: "an HTML block's show is everyone, visitors or members",
            });
        }
    };
    Ok(BlockKind::SafeHtml { show })
}
