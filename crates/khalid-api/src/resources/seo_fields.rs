//! A post's own search settings that need checking before they are kept:
//! the title search results show, and hand-written JSON-LD (KH-SEO-1,
//! KH-SEO-4).

use khalid_seo::{CustomSchema, MetaTemplate};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// The title search results show for a post, in place of the site's title
/// template. It may use the template variables, such as `{site}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(try_from = "String", into = "String")]
#[schema(value_type = String, example = "Anchor text that says where it goes {sep} {site}")]
pub struct SearchTitle(String);

impl SearchTitle {
    /// `text` as a search title.
    ///
    /// # Errors
    ///
    /// Says why when `text` is blank, or has a brace without its partner or
    /// a name in braces that is not a variable.
    pub fn try_new(text: &str) -> Result<Self, String> {
        if text.trim().is_empty() {
            return Err(
                "the search title must not be blank; send null for the post's own title".to_owned(),
            );
        }
        MetaTemplate::parse(text)
            .map(|_| Self(text.to_owned()))
            .map_err(|error| error.to_string())
    }

    /// The title as written.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for SearchTitle {
    type Error = String;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        Self::try_new(&text)
    }
}

impl From<SearchTitle> for String {
    fn from(title: SearchTitle) -> Self {
        title.0
    }
}

/// JSON-LD written by hand for a type Khalid does not write itself, kept
/// as written. It is read as schema.org nodes before it is kept, so a page
/// never carries JSON-LD a search engine would throw away.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(try_from = "String", into = "String")]
#[schema(value_type = String, example = "{\"@type\": \"Event\", \"name\": \"Anchor text clinic\"}")]
pub struct CustomJsonLd(String);

impl CustomJsonLd {
    /// `text` as custom JSON-LD.
    ///
    /// # Errors
    ///
    /// Says why, and for a slip in the JSON on which line and column, when
    /// `text` is blank or is not schema.org JSON-LD.
    pub fn try_new(text: &str) -> Result<Self, String> {
        if text.trim().is_empty() {
            return Err("the schema must not be blank; send null for none".to_owned());
        }
        CustomSchema::parse(text)
            .map(|_| Self(text.to_owned()))
            .map_err(|error| error.to_string())
    }

    /// The JSON-LD as written.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for CustomJsonLd {
    type Error = String;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        Self::try_new(&text)
    }
}

impl From<CustomJsonLd> for String {
    fn from(schema: CustomJsonLd) -> Self {
        schema.0
    }
}
