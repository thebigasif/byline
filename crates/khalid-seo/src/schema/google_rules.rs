//! Google's required and recommended properties per schema.org type.
//!
//! Read from the structured data guides on Google Search Central on
//! 2026-10-26. Types Google no longer shows as a rich result have no rule:
//! `FAQPage` (ended 2026-05-07), course info (ended 2025-09-09) and `HowTo`
//! (ended 2023). Their schema is still written, and still valid schema.org.

/// One rule per line, five columns split by `;`: the types, by exact name;
/// what Google shows when the rule holds, finishing "Google shows ..."; the
/// required properties, where names joined by `|` are a choice; the ones
/// needed only when the node stands alone, not inside another; and the
/// recommended ones.
const TABLE: &str = "\
Article BlogPosting NewsArticle TechArticle; an article with its headline, image and date; ; ; author dateModified datePublished headline image
BreadcrumbList; the page's breadcrumb trail; itemListElement; ;
ItemList; the list as a carousel; itemListElement; ;
Event; an event in its event results; location name startDate; ; description endDate eventStatus image offers organizer performer
LocalBusiness; a business in its local results; address name; ; geo openingHoursSpecification priceRange telephone url
Organization; an organisation's logo and details; ; ; logo name url
Person; who a person is; name; ;
ProfilePage; a profile page with the person's name; mainEntity; ; dateCreated dateModified
WebSite; the site's name; name url; ;
Product; a product with its price or rating; name review|aggregateRating|offers; ;
Offer; a price; price|priceSpecification; ; availability priceCurrency
Recipe; a recipe card; image name; ; aggregateRating author cookTime datePublished description keywords \
nutrition prepTime recipeCategory recipeCuisine recipeIngredient recipeInstructions recipeYield totalTime
Review; a review with its stars; author reviewRating; itemReviewed; datePublished
AggregateRating; a rating with its stars; ratingValue ratingCount|reviewCount; itemReviewed; bestRating
Rating; a review with its stars; ratingValue; ; bestRating
SoftwareApplication MobileApplication WebApplication; an app with its rating and price; name offers aggregateRating|review; ; applicationCategory operatingSystem
VideoObject; a video in its video results; name thumbnailUrl uploadDate; ; contentUrl description duration embedUrl
Dataset; a dataset in Dataset Search; description name; ; creator keywords license url
";

/// What Google asks of one schema.org type: one row of the table.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Rule {
    kinds: &'static str,
    /// What Google shows when the rule holds, finishing "Google shows ...".
    pub(crate) feature: &'static str,
    required: &'static str,
    required_alone: &'static str,
    recommended: &'static str,
}

impl Rule {
    fn parse(row: &'static str) -> Option<Self> {
        let mut columns = row.split(';').map(str::trim);
        let rule = Self {
            kinds: columns.next()?,
            feature: columns.next()?,
            required: columns.next()?,
            required_alone: columns.next()?,
            recommended: columns.next()?,
        };
        columns.next().is_none().then_some(rule)
    }

    /// Each required choice, such as `["review", "aggregateRating", "offers"]`.
    pub(crate) fn required(self) -> impl Iterator<Item = Vec<&'static str>> {
        self.required
            .split_whitespace()
            .map(|choice| choice.split('|').collect())
    }

    /// The properties a node needs only when it stands alone.
    pub(crate) fn required_alone(self) -> impl Iterator<Item = Vec<&'static str>> {
        self.required_alone
            .split_whitespace()
            .map(|name| vec![name])
    }

    /// The recommended properties.
    pub(crate) fn recommended(self) -> impl Iterator<Item = &'static str> {
        self.recommended.split_whitespace()
    }
}

/// Google's rule for the type `kind`, if it has one.
pub(crate) fn rule_for(kind: &str) -> Option<Rule> {
    TABLE
        .lines()
        .filter_map(Rule::parse)
        .find(|rule| rule.kinds.split_whitespace().any(|name| name == kind))
}

#[cfg(test)]
mod tests {
    use super::{Rule, TABLE};

    #[test]
    fn every_row_has_five_columns_and_names_a_type_and_a_feature() {
        for row in TABLE.lines() {
            let rule = Rule::parse(row).unwrap_or_else(|| panic!("{row}"));
            assert!(!rule.kinds.is_empty() && !rule.feature.is_empty(), "{row}");
        }
        assert_eq!(TABLE.lines().count(), 18);
    }
}
