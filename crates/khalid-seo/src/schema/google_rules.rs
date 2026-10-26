//! Google's required and recommended properties per schema.org type.
//!
//! Read from the structured data guides on Google Search Central on
//! 2026-10-26. Types Google no longer shows as a rich result have no rule:
//! `FAQPage` (ended 2026-05-07), course info (ended 2025-09-09) and `HowTo`
//! (ended 2023). Their schema is still written, and still valid schema.org.

/// What Google asks of one schema.org type.
#[derive(Debug)]
pub(crate) struct Rule {
    /// The types the rule is for, by exact name.
    pub(crate) kinds: &'static [&'static str],
    /// What Google shows when the rule holds, finishing "Google shows ...".
    pub(crate) feature: &'static str,
    /// Each entry is a set of properties, at least one of which must be there.
    pub(crate) required: &'static [&'static [&'static str]],
    /// Required only when the node stands alone, not inside another node.
    pub(crate) required_alone: &'static [&'static str],
    /// Properties that make the result fuller.
    pub(crate) recommended: &'static [&'static str],
}

const RULES: &[Rule] = &[
    Rule {
        kinds: &["Article", "BlogPosting", "NewsArticle", "TechArticle"],
        feature: "an article with its headline, image and date",
        required: &[],
        required_alone: &[],
        recommended: &[
            "author",
            "dateModified",
            "datePublished",
            "headline",
            "image",
        ],
    },
    Rule {
        kinds: &["BreadcrumbList"],
        feature: "the page's breadcrumb trail",
        required: &[&["itemListElement"]],
        required_alone: &[],
        recommended: &[],
    },
    Rule {
        kinds: &["ItemList"],
        feature: "the list as a carousel",
        required: &[&["itemListElement"]],
        required_alone: &[],
        recommended: &[],
    },
    Rule {
        kinds: &["Event"],
        feature: "an event in its event results",
        required: &[&["location"], &["name"], &["startDate"]],
        required_alone: &[],
        recommended: &[
            "description",
            "endDate",
            "eventStatus",
            "image",
            "offers",
            "organizer",
            "performer",
        ],
    },
    Rule {
        kinds: &["LocalBusiness"],
        feature: "a business in its local results",
        required: &[&["address"], &["name"]],
        required_alone: &[],
        recommended: &[
            "geo",
            "openingHoursSpecification",
            "priceRange",
            "telephone",
            "url",
        ],
    },
    Rule {
        kinds: &["Organization"],
        feature: "an organisation's logo and details",
        required: &[],
        required_alone: &[],
        recommended: &["logo", "name", "url"],
    },
    Rule {
        kinds: &["Person"],
        feature: "who a person is",
        required: &[&["name"]],
        required_alone: &[],
        recommended: &[],
    },
    Rule {
        kinds: &["ProfilePage"],
        feature: "a profile page with the person's name",
        required: &[&["mainEntity"]],
        required_alone: &[],
        recommended: &["dateCreated", "dateModified"],
    },
    Rule {
        kinds: &["WebSite"],
        feature: "the site's name",
        required: &[&["name"], &["url"]],
        required_alone: &[],
        recommended: &[],
    },
    Rule {
        kinds: &["Product"],
        feature: "a product with its price or rating",
        required: &[&["name"], &["review", "aggregateRating", "offers"]],
        required_alone: &[],
        recommended: &[],
    },
    Rule {
        kinds: &["Offer"],
        feature: "a price",
        required: &[&["price", "priceSpecification"]],
        required_alone: &[],
        recommended: &["availability", "priceCurrency"],
    },
    Rule {
        kinds: &["Recipe"],
        feature: "a recipe card",
        required: &[&["image"], &["name"]],
        required_alone: &[],
        recommended: &[
            "aggregateRating",
            "author",
            "cookTime",
            "datePublished",
            "description",
            "keywords",
            "nutrition",
            "prepTime",
            "recipeCategory",
            "recipeCuisine",
            "recipeIngredient",
            "recipeInstructions",
            "recipeYield",
            "totalTime",
        ],
    },
    Rule {
        kinds: &["Review"],
        feature: "a review with its stars",
        required: &[&["author"], &["reviewRating"]],
        required_alone: &["itemReviewed"],
        recommended: &["datePublished"],
    },
    Rule {
        kinds: &["AggregateRating"],
        feature: "a rating with its stars",
        required: &[&["ratingValue"], &["ratingCount", "reviewCount"]],
        required_alone: &["itemReviewed"],
        recommended: &["bestRating"],
    },
    Rule {
        kinds: &["Rating"],
        feature: "a review with its stars",
        required: &[&["ratingValue"]],
        required_alone: &[],
        recommended: &["bestRating"],
    },
    Rule {
        kinds: &["SoftwareApplication", "MobileApplication", "WebApplication"],
        feature: "an app with its rating and price",
        required: &[&["name"], &["offers"], &["aggregateRating", "review"]],
        required_alone: &[],
        recommended: &["applicationCategory", "operatingSystem"],
    },
    Rule {
        kinds: &["VideoObject"],
        feature: "a video in its video results",
        required: &[&["name"], &["thumbnailUrl"], &["uploadDate"]],
        required_alone: &[],
        recommended: &["contentUrl", "description", "duration", "embedUrl"],
    },
    Rule {
        kinds: &["Dataset"],
        feature: "a dataset in Dataset Search",
        required: &[&["description"], &["name"]],
        required_alone: &[],
        recommended: &["creator", "keywords", "license", "url"],
    },
];

/// Google's rule for the type `kind`, if it has one.
pub(crate) fn rule_for(kind: &str) -> Option<&'static Rule> {
    RULES.iter().find(|rule| rule.kinds.contains(&kind))
}
