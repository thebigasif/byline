//! [`SchemaValue`]: the value of one schema.org property.

use std::fmt::Write as _;
use std::time::Duration;

use chrono::{DateTime, NaiveDate, Utc};
use khalid_core::AbsoluteUrl;
use serde_json::{Number, Value, json};

use crate::SchemaNode;
use crate::social::rfc3339;

/// The value of one schema.org property.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum SchemaValue {
    /// Text, such as a headline.
    Text(String),
    /// A web address.
    Url(AbsoluteUrl),
    /// A day, written `2019-03-01`.
    Date(NaiveDate),
    /// A moment, written `2026-09-12T10:00:00Z`.
    DateTime(DateTime<Utc>),
    /// A length of time, written in ISO 8601 such as `PT5M12S`.
    Duration(Duration),
    /// A whole number, such as a word count.
    Integer(i64),
    /// A number with a fraction, such as a rating of 4.5.
    Number(f64),
    /// True or false, such as `isAccessibleForFree`.
    Boolean(bool),
    /// A node written inside this one.
    Node(Box<SchemaNode>),
    /// A node elsewhere in the graph, written as `{"@id": ...}`.
    Reference(AbsoluteUrl),
    /// Several values for one property.
    List(Vec<SchemaValue>),
}

impl SchemaValue {
    /// Text from a string slice.
    pub fn text(text: &str) -> Self {
        Self::Text(text.to_owned())
    }

    /// The value as JSON.
    pub(crate) fn to_json(&self) -> Value {
        match self {
            Self::Text(text) => json!(text),
            Self::Url(url) => json!(url.as_str()),
            Self::Reference(url) => json!({ "@id": url.as_str() }),
            Self::Date(date) => json!(date.format("%Y-%m-%d").to_string()),
            Self::DateTime(time) => json!(rfc3339(time)),
            Self::Duration(duration) => json!(iso_duration(*duration)),
            Self::Integer(number) => json!(number),
            Self::Number(number) => Number::from_f64(*number).map_or(Value::Null, Value::Number),
            Self::Boolean(flag) => json!(flag),
            Self::Node(node) => node.to_json(),
            Self::List(values) => Value::Array(values.iter().map(Self::to_json).collect()),
        }
    }
}

/// A duration in ISO 8601, to the second: `PT3H5M12S`, `PT45S`, `PT0S`.
fn iso_duration(duration: Duration) -> String {
    let total = duration.as_secs();
    let (hours, minutes, seconds) = (total / 3600, total % 3600 / 60, total % 60);
    let mut out = String::from("PT");
    if hours > 0 {
        let _ = write!(out, "{hours}H");
    }
    if minutes > 0 {
        let _ = write!(out, "{minutes}M");
    }
    if seconds > 0 || total == 0 {
        let _ = write!(out, "{seconds}S");
    }
    out
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::iso_duration;

    #[test]
    fn durations_write_only_the_parts_they_have() {
        let cases = [
            (0, "PT0S"),
            (45, "PT45S"),
            (300, "PT5M"),
            (3600, "PT1H"),
            (3661, "PT1H1M1S"),
        ];
        for (seconds, written) in cases {
            assert_eq!(iso_duration(Duration::from_secs(seconds)), written);
        }
    }
}
