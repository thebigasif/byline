//! An API key as the Admin API lists it: never its secret or hash.

use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;

use crate::keys::{Access, ListedKey};

/// A key that works, in Admin API shape. The secret is shown once, when
/// the key is made, and never again.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct AdminKey {
    /// The key's identity, the part before its secret, such as
    /// `1111222233334444`.
    id: String,
    /// Who or what uses it, such as `SerpDad`.
    name: String,
    /// `content`, `integration` or `staff`.
    #[schema(example = "integration")]
    kind: &'static str,
    /// An integration key's scopes, such as `posts:write`.
    #[schema(example = json!(["posts:write"]))]
    scopes: Vec<&'static str>,
    /// A staff token's role, such as `editor`.
    role: Option<&'static str>,
    /// When the key was made.
    created_at: DateTime<Utc>,
}

impl From<ListedKey> for AdminKey {
    fn from(key: ListedKey) -> Self {
        let (kind, scopes, role) = match &key.access {
            Access::Content => ("content", Vec::new(), None),
            Access::Integration(scopes) => (
                "integration",
                scopes.iter().map(|s| s.as_str()).collect(),
                None,
            ),
            Access::Staff(role) => ("staff", Vec::new(), Some(role.as_str())),
        };
        Self {
            id: key.id.as_str().to_owned(),
            name: key.name,
            kind,
            scopes,
            role,
            created_at: key.created_at,
        }
    }
}
