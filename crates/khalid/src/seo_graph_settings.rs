//! `[seo.publisher]`, `[[seo.locations]]` and `[[seo.schema]]` from
//! `site.toml`: who is behind the site, where it is, and the schema node
//! every post in a category or collection gets (audit G11, KH-SEO-4).

use chrono::{NaiveDate, NaiveTime, Weekday};
use khalid_core::{
    HoursSettings, LocationSettings, PublisherSettings, PublisherType, SchemaTemplateSettings,
    WeekdaySetting,
};
use khalid_seo::{
    LocalBusiness, MetaTemplate, OpeningHours, PostalAddress, PublisherKind, SchemaTemplate,
    SchemaType, SeoSite, TemplateScope,
};

use crate::seo_head_settings::share_image;
use crate::seo_settings::{SettingMistake, TEMPLATE};

/// Checks who publishes the site and sets it on `site`.
///
/// # Errors
///
/// A logo, email or founding date that is refused.
pub(crate) fn publisher(
    settings: &PublisherSettings,
    site: &mut SeoSite,
) -> Result<(), SettingMistake> {
    const TABLE: &str = "seo.publisher";
    let logo = settings
        .logo
        .as_ref()
        .map(|logo| share_image("seo.publisher.logo", logo))
        .transpose()?;
    if let Some(email) = &settings.email
        && !is_email(email)
    {
        let problem = format!("{email:?} is not an email address");
        return Err(SettingMistake::new(
            TABLE,
            "email",
            &problem,
            Some("an address such as hello@example.com"),
        ));
    }
    let founded = settings
        .founded
        .as_deref()
        .map(|text| {
            NaiveDate::parse_from_str(text, "%Y-%m-%d").map_err(|_| {
                let problem = format!("{text:?} is not a date");
                let expected = "a date written year, month, day, such as 2019-04-01";
                SettingMistake::new(TABLE, "founded", &problem, Some(expected))
            })
        })
        .transpose()?;
    let publisher = &mut site.publisher;
    publisher.kind = match settings.kind {
        PublisherType::Organization => PublisherKind::Organization,
        PublisherType::Person => PublisherKind::Person,
    };
    if let Some(name) = &settings.name {
        publisher.name.clone_from(name);
    }
    publisher.logo = logo;
    publisher.same_as.clone_from(&settings.same_as);
    publisher.email.clone_from(&settings.email);
    publisher.telephone.clone_from(&settings.telephone);
    publisher.founding_date = founded;
    Ok(())
}

/// One `@`, a dotted host and no spaces: catches a slip, refuses no real address.
fn is_email(text: &str) -> bool {
    text.split_once('@')
        .is_some_and(|(user, host)| !user.is_empty() && host.contains('.') && !host.contains('@'))
        && !text.chars().any(char::is_whitespace)
}

/// Checks each location and sets them on `site`.
///
/// # Errors
///
/// A type, country or opening time that is refused, in the entry it sits in.
pub(crate) fn locations(
    settings: &[LocationSettings],
    site: &mut SeoSite,
) -> Result<(), SettingMistake> {
    let mut hours_seen = 0_usize;
    for (entry, location) in settings.iter().enumerate() {
        let at = |key: &str, problem: &dyn std::fmt::Display, expected| {
            SettingMistake::new("seo.locations", key, problem, expected).in_entry(entry)
        };
        let kind = SchemaType::try_new(&location.kind)
            .map_err(|error| at("type", &error, Some("a schema.org type such as Dentist")))?;
        let country = &location.country;
        if country.len() != 2 || !country.chars().all(|c| c.is_ascii_uppercase()) {
            let problem = format!("{country:?} is not a country code");
            return Err(at(
                "country",
                &problem,
                Some("two capital letters, such as GB"),
            ));
        }
        let mut address = PostalAddress::new(
            &location.street,
            &location.locality,
            &location.postal_code,
            country,
        );
        address.region.clone_from(&location.region);
        let mut business = LocalBusiness::new(kind.as_str(), &location.name, address);
        business.telephone.clone_from(&location.telephone);
        business.price_range.clone_from(&location.price_range);
        business.map.clone_from(&location.map);
        for hours in &location.hours {
            business
                .opening_hours
                .push(opening_hours(hours).map_err(|mistake| mistake.in_entry(hours_seen))?);
            hours_seen = hours_seen.saturating_add(1);
        }
        site.locations.push(business);
    }
    Ok(())
}

fn opening_hours(settings: &HoursSettings) -> Result<OpeningHours, SettingMistake> {
    let time = |key: &str, text: &str| {
        NaiveTime::parse_from_str(text, "%H:%M").map_err(|_| {
            let problem = format!("{text:?} is not a time");
            let expected = "hours and minutes on a 24-hour clock, such as 09:00";
            SettingMistake::new("seo.locations.hours", key, &problem, Some(expected))
        })
    };
    let days = settings
        .days
        .iter()
        .map(|day| match day {
            WeekdaySetting::Mon => Weekday::Mon,
            WeekdaySetting::Tue => Weekday::Tue,
            WeekdaySetting::Wed => Weekday::Wed,
            WeekdaySetting::Thu => Weekday::Thu,
            WeekdaySetting::Fri => Weekday::Fri,
            WeekdaySetting::Sat => Weekday::Sat,
            WeekdaySetting::Sun => Weekday::Sun,
        })
        .collect();
    Ok(OpeningHours::new(
        days,
        time("opens", &settings.opens)?,
        time("closes", &settings.closes)?,
    ))
}

/// Checks each schema template and sets them on `site`.
///
/// # Errors
///
/// A scope, type, property name or template that is refused.
pub(crate) fn schema(
    settings: &[SchemaTemplateSettings],
    site: &mut SeoSite,
) -> Result<(), SettingMistake> {
    for (entry, template) in settings.iter().enumerate() {
        let at = |table: &str, key: &str, problem: &dyn std::fmt::Display, expected| {
            SettingMistake::new(table, key, problem, expected).in_entry(entry)
        };
        let scope = match (&template.category, &template.collection) {
            (Some(category), None) => TemplateScope::Category(category.clone()),
            (None, Some(collection)) => TemplateScope::Collection(collection.clone()),
            (Some(_), Some(_)) => {
                let problem = "a schema template may name a category or a collection, not both";
                return Err(at("seo.schema", "collection", &problem, None));
            }
            (None, None) => {
                let problem =
                    "a schema template needs the category or the collection it applies to";
                return Err(at("seo.schema", "type", &problem, None));
            }
        };
        let kind = SchemaType::try_new(&template.kind).map_err(|error| {
            at(
                "seo.schema",
                "type",
                &error,
                Some("a schema.org type such as Course"),
            )
        })?;
        let mut node = SchemaTemplate::new(scope, kind);
        for (name, text) in &template.properties {
            let table = "seo.schema.properties";
            let value = MetaTemplate::parse(text)
                .map_err(|error| at(table, name, &error, Some(TEMPLATE)))?;
            node.set(name, value)
                .map_err(|error| at(table, name, &error, None))?;
        }
        site.schema_templates.push(node);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use khalid_seo::PublisherKind;

    use crate::seo_settings::apply;
    use crate::seo_settings::testing::{settings, site};

    #[test]
    fn a_person_publisher_reaches_the_site() {
        let mut site = site();
        let tables =
            "[seo.publisher]\ntype = \"person\"\nname = \"Asif\"\nfounded = \"2019-04-01\"\n";
        apply(&settings(tables), &mut site).unwrap();
        assert_eq!(site.publisher.kind, PublisherKind::Person);
        assert_eq!(site.publisher.name, "Asif");
        assert_eq!(
            site.publisher
                .founding_date
                .map(|day| day.to_string())
                .as_deref(),
            Some("2019-04-01")
        );
    }

    #[test]
    fn an_email_without_a_domain_is_refused() {
        let error = apply(
            &settings("[seo.publisher]\nemail = \"asif\"\n"),
            &mut site(),
        )
        .unwrap_err();
        assert_eq!(error.key, "email");
    }

    #[test]
    fn a_mistake_in_the_second_location_names_that_entry() {
        let one = "[[seo.locations]]\nname = \"a\"\nstreet = \"b\"\nlocality = \"c\"\npostal_code = \"d\"\n";
        let tables = format!("{one}country = \"GB\"\n{one}country = \"gb\"\n");
        let error = apply(&settings(&tables), &mut site()).unwrap_err();
        assert_eq!((error.key.as_str(), error.entry), ("country", 1));
    }

    #[test]
    fn a_schema_template_needs_a_scope_and_a_property_name() {
        let error = apply(
            &settings("[[seo.schema]]\ntype = \"Course\"\n"),
            &mut site(),
        )
        .unwrap_err();
        assert_eq!(error.key, "type");
        let tables = "[[seo.schema]]\ncategory = \"a\"\ntype = \"Course\"\n[seo.schema.properties]\nProvider = \"{site}\"\n";
        let error = apply(&settings(tables), &mut site()).unwrap_err();
        assert_eq!(
            (error.table.as_str(), error.key.as_str()),
            ("seo.schema.properties", "Provider")
        );
    }
}
