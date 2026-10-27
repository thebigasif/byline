//! The `[seo]` tables that say who is behind the site, for its schema graph:
//! the publisher, its locations and the schema templates per category or
//! collection.
//!
//! Core reads only their shape. `khalid` checks each value against the rules
//! in `khalid-seo` before any command runs.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::{AbsoluteUrl, ImageSettings};

/// Who publishes the site, from `[seo.publisher]`. By default an
/// organisation named after the site:
///
/// ```toml
/// [seo.publisher]
/// type = "person"
/// name = "Asif Khan"
/// same_as = ["https://www.linkedin.com/in/asif"]
/// founded = "2019-04-01"
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
#[non_exhaustive]
pub struct PublisherSettings {
    /// A company or a person.
    #[serde(rename = "type")]
    pub kind: PublisherType,
    /// The company's or the person's name. `None` uses the site's name.
    pub name: Option<String>,
    /// The logo, or for a person their photo.
    pub logo: Option<ImageSettings>,
    /// Profiles elsewhere that are the same publisher.
    pub same_as: Vec<AbsoluteUrl>,
    /// A public contact address.
    pub email: Option<String>,
    /// A public phone number.
    pub telephone: Option<String>,
    /// When the company was founded, such as `"2019-04-01"`.
    pub founded: Option<String>,
}

/// Whether the site is a company's or a person's: `"organization"` or `"person"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PublisherType {
    /// A company or another organisation.
    #[default]
    Organization,
    /// One person.
    Person,
}

/// One shop, office or clinic, from a `[[seo.locations]]` table:
///
/// ```toml
/// [[seo.locations]]
/// type = "Dentist"
/// name = "Mill Lane Dental"
/// street = "12 Mill Lane"
/// locality = "Leeds"
/// postal_code = "LS1 4AB"
/// country = "GB"
///
/// [[seo.locations.hours]]
/// days = ["mon", "tue", "wed"]
/// opens = "09:00"
/// closes = "17:30"
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct LocationSettings {
    /// The schema.org type, `LocalBusiness` by default or a more exact one
    /// such as `Dentist`.
    #[serde(rename = "type", default = "local_business")]
    pub kind: String,
    /// The location's name.
    pub name: String,
    /// The street and number.
    pub street: String,
    /// The town or city.
    pub locality: String,
    /// The postcode.
    pub postal_code: String,
    /// The two-letter country code, such as `GB`.
    pub country: String,
    /// The county, state or region, if the address uses one.
    #[serde(default)]
    pub region: Option<String>,
    /// Its phone number.
    #[serde(default)]
    pub telephone: Option<String>,
    /// How expensive it is, such as `££`.
    #[serde(default)]
    pub price_range: Option<String>,
    /// A map of where it is.
    #[serde(default)]
    pub map: Option<AbsoluteUrl>,
    /// When it is open.
    #[serde(default)]
    pub hours: Vec<HoursSettings>,
}

fn local_business() -> String {
    "LocalBusiness".to_owned()
}

/// Opening hours on some days of the week, times written `"09:00"`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct HoursSettings {
    /// The days these hours apply to.
    pub days: Vec<WeekdaySetting>,
    /// When it opens.
    pub opens: String,
    /// When it closes.
    pub closes: String,
}

/// A day of the week, written `"mon"` to `"sun"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WeekdaySetting {
    /// Monday.
    Mon,
    /// Tuesday.
    Tue,
    /// Wednesday.
    Wed,
    /// Thursday.
    Thu,
    /// Friday.
    Fri,
    /// Saturday.
    Sat,
    /// Sunday.
    Sun,
}

/// One schema.org node for every post in a category or a collection, from
/// a `[[seo.schema]]` table. Each property is a template with the same
/// variables as a title:
///
/// ```toml
/// [[seo.schema]]
/// category = "Courses"
/// type = "Course"
///
/// [seo.schema.properties]
/// provider = "{site}"
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct SchemaTemplateSettings {
    /// The category whose posts get the node. Set this or `collection`.
    #[serde(default)]
    pub category: Option<String>,
    /// The collection whose posts get the node, such as `guides`.
    #[serde(default)]
    pub collection: Option<String>,
    /// The schema.org type, such as `Course`.
    #[serde(rename = "type")]
    pub kind: String,
    /// Each property's template, by property name.
    #[serde(default)]
    pub properties: BTreeMap<String, String>,
}
