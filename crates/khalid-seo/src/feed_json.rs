//! The JSON Feed 1.1 writer for [`Feed`].

use serde_json::{Map, Value, json};

use crate::social::rfc3339;
use crate::{Feed, FeedFormat, FeedItem, SeoError};

fn item_json(item: &FeedItem) -> Value {
    let mut object = Map::new();
    object.insert("id".to_owned(), json!(item.url.as_str()));
    object.insert("url".to_owned(), json!(item.url.as_str()));
    object.insert("title".to_owned(), json!(item.title));
    object.insert("content_html".to_owned(), json!(item.content_html));
    if !item.summary.is_empty() {
        object.insert("summary".to_owned(), json!(item.summary));
    }
    if let Some(image) = &item.image {
        object.insert("image".to_owned(), json!(image.as_str()));
    }
    object.insert("date_published".to_owned(), json!(rfc3339(&item.published)));
    if let Some(updated) = &item.updated {
        object.insert("date_modified".to_owned(), json!(rfc3339(updated)));
    }
    if let Some(author) = &item.author {
        object.insert("authors".to_owned(), json!([{ "name": author }]));
    }
    if let Some(category) = &item.category {
        object.insert("tags".to_owned(), json!([category]));
    }
    Value::Object(object)
}

impl Feed {
    /// The feed as a JSON Feed 1.1 document, with its fields in a fixed order.
    ///
    /// # Errors
    ///
    /// [`SeoError::Url`] when the feed's own address cannot be built.
    pub fn to_json(&self) -> Result<String, SeoError> {
        let mut object = Map::new();
        object.insert(
            "version".to_owned(),
            json!("https://jsonfeed.org/version/1.1"),
        );
        object.insert("title".to_owned(), json!(self.title));
        object.insert("home_page_url".to_owned(), json!(self.home.as_str()));
        object.insert(
            "feed_url".to_owned(),
            json!(self.url(FeedFormat::Json)?.as_str()),
        );
        if !self.description.is_empty() {
            object.insert("description".to_owned(), json!(self.description));
        }
        object.insert("language".to_owned(), json!(self.language.as_str()));
        object.insert("authors".to_owned(), json!([{ "name": self.site_name }]));
        let items: Vec<Value> = self.items.iter().map(item_json).collect();
        object.insert("items".to_owned(), Value::Array(items));
        Ok(format!("{:#}\n", Value::Object(object)))
    }
}
