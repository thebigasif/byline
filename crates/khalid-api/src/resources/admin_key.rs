//! An API key as the Admin API lists it: never its secret or hash.

use chrono::{DateTime, Utc};
use khalid_core::Secret;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::keys::{Access, ListedKey};

/// A key that works, in Admin API shape. The secret is shown once, when
/// the key is made, and never again.
#[derive(Clone, PartialEq, Eq, Serialize, ToSchema)]
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
    /// The key itself. Only the answer that makes it holds it; keep it
    /// then, because it is never shown again.
    key: Option<String>,
}

/// Never prints the key itself (RULES.md BL-SEC-3).
impl std::fmt::Debug for AdminKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AdminKey")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("kind", &self.kind)
            .finish_non_exhaustive()
    }
}

impl AdminKey {
    /// The same key, holding `key` for the one answer that shows it.
    #[must_use]
    pub fn showing(self, key: &Secret<String>) -> Self {
        Self {
            key: Some(key.expose().clone()),
            ..self
        }
    }

    /// The identity.
    pub fn id(&self) -> &str {
        &self.id
    }
}

/// A new key, as a create request sends it.
#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct NewKey {
    /// Who or what will use it, such as `SerpDad`.
    pub name: String,
    /// What kind of key to make.
    pub kind: NewKeyKind,
    /// An integration key's scopes, such as `posts:write`; a content key
    /// takes none.
    #[serde(default)]
    #[schema(example = json!(["posts:write"]))]
    pub scopes: Vec<String>,
}

/// The kinds of key the Admin API makes. Staff tokens belong to staff
/// accounts and are made by signing in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum NewKeyKind {
    /// Reads published content and is safe in a browser.
    Content,
    /// Does what its scopes allow, for one tool or service.
    Integration,
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
            key: None,
        }
    }
}
