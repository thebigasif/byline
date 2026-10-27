//! What a Ghost export holds, with each post's text as written.

use chrono::NaiveDateTime;

/// Everything a Ghost export holds that an import uses.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct Export {
    /// The site's title, from its settings.
    pub title: Option<String>,
    /// The version of Ghost that wrote the file, such as `5.96.0`.
    pub version: Option<String>,
    /// Every post and page, in file order.
    pub posts: Vec<Post>,
}

/// One post or page of the export.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Post {
    /// The id, which the export's other tables use to point at it.
    pub id: String,
    /// A post or a page.
    pub kind: PostKind,
    /// Whether it was published.
    pub status: PostStatus,
    /// Who could read it.
    pub visibility: Visibility,
    /// The headline.
    pub title: String,
    /// The slug, the last part of its address.
    pub slug: String,
    /// When it was published, or is due to be, in UTC.
    pub published: Option<NaiveDateTime>,
    /// The body, in the editor's own format.
    pub body: Body,
    /// The hand-written summary.
    pub excerpt: Option<String>,
    /// The image shown with it in lists and share cards. Ghost writes the
    /// site's own address as `__GHOST_URL__`.
    pub feature_image: Option<String>,
    /// Whether it was marked featured.
    pub featured: bool,
    /// The address search engines should credit instead.
    pub canonical_url: Option<String>,
    /// Its fields for search results and share cards.
    pub seo: Seo,
    /// The HTML it added to its own page, which an import never carries.
    pub code_injection: CodeInjection,
}

/// Whether an entry is a post or a page.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PostKind {
    /// A blog post.
    Post,
    /// A page, such as About.
    Page,
}

/// Whether a post was published.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PostStatus {
    /// Live on the site.
    Published,
    /// Being written.
    Draft,
    /// Due to go live at its date.
    Scheduled,
    /// Sent as a newsletter only, with no page on the site.
    Sent,
    /// Any other status, by its name in the file.
    Other(String),
}

impl PostStatus {
    pub(crate) fn from_name(name: &str) -> Self {
        match name {
            "published" => Self::Published,
            "draft" => Self::Draft,
            "scheduled" => Self::Scheduled,
            "sent" => Self::Sent,
            other => Self::Other(other.to_owned()),
        }
    }
}

/// Who could read a post.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Visibility {
    /// Everyone.
    Public,
    /// Signed-in members only.
    Members,
    /// Paying members only.
    Paid,
    /// Members of chosen tiers only.
    Tiers,
    /// Any other setting, by its name in the file.
    Other(String),
}

impl Visibility {
    pub(crate) fn from_name(name: &str) -> Self {
        match name {
            "public" => Self::Public,
            "members" => Self::Members,
            "paid" => Self::Paid,
            "tiers" => Self::Tiers,
            other => Self::Other(other.to_owned()),
        }
    }
}

/// A post's body as Ghost stored it. When a post holds more than one, the
/// editor's own format wins, Lexical first, because Ghost renders from it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Body {
    /// The Lexical document of Ghost 5 and later, as JSON text.
    Lexical(String),
    /// The Mobiledoc document of Ghost 1 to 4, as JSON text.
    Mobiledoc(String),
    /// Only the rendered HTML.
    Html(String),
    /// No body at all.
    Empty,
}

/// A post's fields for search results and share cards. Each is none when
/// Ghost left it blank, so its default applies.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct Seo {
    /// The title for search results.
    pub meta_title: Option<String>,
    /// The description for search results.
    pub meta_description: Option<String>,
    /// The Open Graph title, for Facebook and most other share cards.
    pub og_title: Option<String>,
    /// The Open Graph description.
    pub og_description: Option<String>,
    /// The Open Graph image.
    pub og_image: Option<String>,
    /// The title on X, formerly Twitter.
    pub twitter_title: Option<String>,
    /// The description on X.
    pub twitter_description: Option<String>,
    /// The image on X.
    pub twitter_image: Option<String>,
}

/// HTML a post added to its own page, in the head or before the end of the
/// body, often for analytics. An import notes it and never carries it,
/// because it would run on the new site.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct CodeInjection {
    /// What it added to the head.
    pub head: Option<String>,
    /// What it added before the end of the body.
    pub foot: Option<String>,
}
