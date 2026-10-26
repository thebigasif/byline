//! Typed data from blocks that search engines read as schema markup.

use std::time::Duration;

/// A block whose content becomes schema markup, in the order of the body,
/// for the schema builders in `khalid-seo` (task 1.14).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum StructuredBlock {
    /// A video block, which becomes a `VideoObject`.
    Video(VideoBlock),
    /// A review block, which becomes a `Review`. The body is plain text.
    Review(ReviewBlock),
    /// A recipe block, which becomes a `Recipe`.
    Recipe(RecipeBlock),
    /// A product card, which becomes a `Product`. The description is plain
    /// text.
    Product(ProductBlock),
    /// A business card, which becomes a `LocalBusiness` or a more exact type.
    Business(BusinessBlock),
    /// An FAQ block, which becomes an `FAQPage`. Answers are plain text.
    Faq(Vec<Question>),
    /// A how-to block, which becomes a `HowTo`. Steps are plain text.
    HowTo(HowToBlock),
}

/// One question of an FAQ and its answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Question {
    /// The question.
    pub text: String,
    /// The answer: markdown in a block, plain text in structured data.
    pub answer: String,
}

/// A how-to: steps that help a reader do one thing, from `::::howto{...}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HowToBlock {
    /// What it helps a reader do.
    pub name: String,
    /// How long it takes in all.
    pub time: Option<Duration>,
    /// The steps, in order.
    pub steps: Vec<Step>,
}

/// One step of a how-to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    /// A few words for the step.
    pub name: String,
    /// What to do: markdown in a block, plain text in structured data.
    pub text: String,
}

/// A video file played on the page, from `::video{...}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoBlock {
    /// The video file: a path on this site or a full address.
    pub src: String,
    /// The video's title.
    pub title: String,
    /// The still picture shown before it plays: a path or a full address.
    pub poster: String,
    /// What the video shows, in a line.
    pub description: Option<String>,
    /// How long it runs.
    pub duration: Option<Duration>,
}

/// A rating of one thing, from `:::review{...}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewBlock {
    /// The thing reviewed.
    pub item: String,
    /// Its schema.org type, one of [`REVIEW_TYPES`](crate::REVIEW_TYPES).
    pub item_type: String,
    /// The score, a number as written, such as `4.5`.
    pub rating: String,
    /// The best possible score, a number as written; `5` when not given.
    pub best: String,
    /// Who wrote the review, when it is not the post's author.
    pub author: Option<String>,
    /// The review: markdown in a block, plain text in structured data.
    pub body: String,
}

/// The schema.org types search engines show review stars for, so a review
/// of anything else would never show its rating.
pub const REVIEW_TYPES: [&str; 17] = [
    "Book",
    "Course",
    "CreativeWorkSeason",
    "CreativeWorkSeries",
    "Episode",
    "Event",
    "Game",
    "HowTo",
    "LocalBusiness",
    "MediaObject",
    "Movie",
    "MusicPlaylist",
    "MusicRecording",
    "Organization",
    "Product",
    "Recipe",
    "SoftwareApplication",
];

/// A recipe, from `::::recipe{...}` with its ingredients and method.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecipeBlock {
    /// The dish.
    pub name: String,
    /// One line about it.
    pub description: Option<String>,
    /// A photo of the dish: a path on this site or a full address.
    pub image: Option<String>,
    /// What it makes, such as `12 bars`.
    pub servings: Option<String>,
    /// Time to prepare.
    pub prep: Option<Duration>,
    /// Time to cook.
    pub cook: Option<Duration>,
    /// The kind of dish, such as `Dessert`.
    pub category: Option<String>,
    /// Where it comes from, such as `British`.
    pub cuisine: Option<String>,
    /// Calories per serving.
    pub calories: Option<u32>,
    /// Each ingredient with its amount, as plain text.
    pub ingredients: Vec<String>,
    /// The steps in order, as plain text.
    pub steps: Vec<String>,
}

/// A product card, from `:::product{...}` with its description as the body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductBlock {
    /// The product.
    pub name: String,
    /// Its picture: a path on this site or a full address.
    pub image: Option<String>,
    /// The price as written, such as `49.00`, always with a currency.
    pub price: Option<String>,
    /// The price's three-letter currency code, such as `USD`.
    pub currency: Option<String>,
    /// The writer's score, a number as written, such as `4.5`.
    pub rating: Option<String>,
    /// The best possible score; `5` when not given.
    pub best: String,
    /// Where to buy it: a path on this site or a full address.
    pub href: Option<String>,
    /// The words on the button, always with `href`.
    pub button: Option<String>,
    /// What it is: markdown in a block, plain text in structured data.
    pub description: String,
}

/// A business card, from `::business{...}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BusinessBlock {
    /// The business.
    pub name: String,
    /// Its schema.org type, such as `Bakery`; `LocalBusiness` when not given.
    pub business_type: String,
    /// The street and number.
    pub street: String,
    /// The town or city.
    pub locality: String,
    /// The postcode.
    pub postal_code: String,
    /// The two-letter country code, such as `GB`.
    pub country: String,
    /// Its phone number as written.
    pub phone: Option<String>,
    /// Its email address.
    pub email: Option<String>,
    /// Its website: a path on this site or a full address.
    pub website: Option<String>,
}
