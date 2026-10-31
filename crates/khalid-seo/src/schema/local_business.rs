//! `LocalBusiness`: a site's shops, offices or clinics, one node each.

use chrono::{NaiveTime, Weekday};
use khalid_core::AbsoluteUrl;

use crate::schema::{node_id, publisher_id, site_home};
use crate::{SchemaNode, SchemaValue, SeoError, SeoSite};

/// A street address.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct PostalAddress {
    /// The street and number, such as `12 Mill Lane`.
    pub street: String,
    /// The town or city.
    pub locality: String,
    /// The postcode.
    pub postal_code: String,
    /// The two-letter country code, such as `GB`.
    pub country: String,
    /// The county, state or region, if the address uses one.
    pub region: Option<String>,
}

impl PostalAddress {
    /// An address with no region.
    pub fn new(street: &str, locality: &str, postal_code: &str, country: &str) -> Self {
        Self {
            street: street.to_owned(),
            locality: locality.to_owned(),
            postal_code: postal_code.to_owned(),
            country: country.to_owned(),
            region: None,
        }
    }
}

/// When a location is open on some days of the week.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct OpeningHours {
    /// The days these hours apply to.
    pub days: Vec<Weekday>,
    /// When it opens.
    pub opens: NaiveTime,
    /// When it closes.
    pub closes: NaiveTime,
}

impl OpeningHours {
    /// Open from `opens` to `closes` on `days`.
    pub fn new(days: Vec<Weekday>, opens: NaiveTime, closes: NaiveTime) -> Self {
        Self {
            days,
            opens,
            closes,
        }
    }
}

/// One location of a local business.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct LocalBusiness {
    /// The schema.org type, `LocalBusiness` or a more exact one such as
    /// `Dentist` or `Bakery`. The vocabulary check confirms it exists.
    pub business_type: String,
    /// The location's name.
    pub name: String,
    /// Where it is.
    pub address: PostalAddress,
    /// Its phone number.
    pub telephone: Option<String>,
    /// When it is open.
    pub opening_hours: Vec<OpeningHours>,
    /// How expensive it is, such as `££`.
    pub price_range: Option<String>,
    /// A map of where it is.
    pub map: Option<AbsoluteUrl>,
}

impl LocalBusiness {
    /// A location with a type, a name and an address.
    pub fn new(business_type: &str, name: &str, address: PostalAddress) -> Self {
        Self {
            business_type: business_type.to_owned(),
            name: name.to_owned(),
            address,
            telephone: None,
            opening_hours: Vec::new(),
            price_range: None,
            map: None,
        }
    }
}

fn day_url(day: Weekday) -> String {
    let name = match day {
        Weekday::Mon => "Monday",
        Weekday::Tue => "Tuesday",
        Weekday::Wed => "Wednesday",
        Weekday::Thu => "Thursday",
        Weekday::Fri => "Friday",
        Weekday::Sat => "Saturday",
        Weekday::Sun => "Sunday",
    };
    format!("https://schema.org/{name}")
}

pub(crate) fn address_node(address: &PostalAddress) -> SchemaValue {
    let mut node = SchemaNode::new("PostalAddress")
        .with("streetAddress", SchemaValue::text(&address.street))
        .with("addressLocality", SchemaValue::text(&address.locality))
        .with("postalCode", SchemaValue::text(&address.postal_code))
        .with("addressCountry", SchemaValue::text(&address.country));
    if let Some(region) = &address.region {
        node.set("addressRegion", SchemaValue::text(region));
    }
    SchemaValue::Node(Box::new(node))
}

fn hours_node(hours: &OpeningHours) -> SchemaValue {
    let days = hours
        .days
        .iter()
        .map(|day| SchemaValue::Text(day_url(*day)))
        .collect();
    let node = SchemaNode::new("OpeningHoursSpecification")
        .with("dayOfWeek", SchemaValue::List(days))
        .with(
            "opens",
            SchemaValue::Text(hours.opens.format("%H:%M").to_string()),
        )
        .with(
            "closes",
            SchemaValue::Text(hours.closes.format("%H:%M").to_string()),
        );
    SchemaValue::Node(Box::new(node))
}

/// One node per location, at `<home>#location-<n>`, each part of the publisher.
pub(crate) fn business_nodes(site: &SeoSite) -> Result<Vec<SchemaNode>, SeoError> {
    let home = site_home(site)?;
    let mut nodes = Vec::with_capacity(site.locations.len());
    for (number, business) in (1..).zip(&site.locations) {
        let mut node = SchemaNode::new(&business.business_type)
            .with_id(node_id(&home, &format!("location-{number}"))?)
            .with("name", SchemaValue::text(&business.name))
            .with("url", SchemaValue::Url(home.clone()))
            .with("address", address_node(&business.address))
            .with(
                "parentOrganization",
                SchemaValue::Reference(publisher_id(site)?),
            );
        if let Some(telephone) = &business.telephone {
            node.set("telephone", SchemaValue::text(telephone));
        }
        if !business.opening_hours.is_empty() {
            let hours = business.opening_hours.iter().map(hours_node).collect();
            node.set("openingHoursSpecification", SchemaValue::List(hours));
        }
        if let Some(price) = &business.price_range {
            node.set("priceRange", SchemaValue::text(price));
        }
        if let Some(map) = &business.map {
            node.set("hasMap", SchemaValue::Url(map.clone()));
        }
        nodes.push(node);
    }
    Ok(nodes)
}
