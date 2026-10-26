//! What a key may do: scopes for integration keys, roles for staff.

use std::str::FromStr;

/// One permission an integration key can hold (`docs/standards/api.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Scope {
    /// `content:read`: read published content.
    ContentRead,
    /// `posts:write`: create and edit posts, including `SerpDad`'s drafts.
    PostsWrite,
    /// `media:write`: upload images and files.
    MediaWrite,
    /// `settings:write`: change site settings.
    SettingsWrite,
    /// `webhooks:manage`: add, change and remove webhooks.
    WebhooksManage,
    /// `subscribers:write`: add and change subscribers.
    SubscribersWrite,
    /// `admin:all`: everything.
    AdminAll,
}

impl Scope {
    /// Every scope, in the order the API standard lists them.
    pub const ALL: [Self; 7] = [
        Self::ContentRead,
        Self::PostsWrite,
        Self::MediaWrite,
        Self::SettingsWrite,
        Self::WebhooksManage,
        Self::SubscribersWrite,
        Self::AdminAll,
    ];

    /// The name, such as `posts:write`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ContentRead => "content:read",
            Self::PostsWrite => "posts:write",
            Self::MediaWrite => "media:write",
            Self::SettingsWrite => "settings:write",
            Self::WebhooksManage => "webhooks:manage",
            Self::SubscribersWrite => "subscribers:write",
            Self::AdminAll => "admin:all",
        }
    }
}

impl FromStr for Scope {
    type Err = ParseScopeError;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|scope| scope.as_str() == name)
            .ok_or_else(|| ParseScopeError(name.to_owned()))
    }
}

/// A name that is not a scope.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0} is not a key scope")]
pub struct ParseScopeError(String);

/// A staff member's role (PRD KH-EDT-7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Role {
    /// Owns the site: everything.
    Owner,
    /// Runs the site: everything.
    Admin,
    /// Edits and publishes everyone's posts and media.
    Editor,
    /// Writes and publishes their own posts.
    Writer,
    /// Writes drafts for others to publish.
    Contributor,
}

impl Role {
    /// Every role, from the most to the least it may do.
    pub const ALL: [Self; 5] = [
        Self::Owner,
        Self::Admin,
        Self::Editor,
        Self::Writer,
        Self::Contributor,
    ];

    /// The name, such as `editor`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::Admin => "admin",
            Self::Editor => "editor",
            Self::Writer => "writer",
            Self::Contributor => "contributor",
        }
    }

    fn allows(self, scope: Scope) -> bool {
        match self {
            Self::Owner | Self::Admin => true,
            Self::Editor | Self::Writer => matches!(
                scope,
                Scope::ContentRead | Scope::PostsWrite | Scope::MediaWrite
            ),
            Self::Contributor => matches!(scope, Scope::ContentRead | Scope::PostsWrite),
        }
    }
}

impl FromStr for Role {
    type Err = ParseRoleError;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|role| role.as_str() == name)
            .ok_or_else(|| ParseRoleError(name.to_owned()))
    }
}

/// A name that is not a role.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0} is not a staff role")]
pub struct ParseRoleError(String);

/// What a stored key may do.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Access {
    /// A content key: read published content, nothing else.
    Content,
    /// An integration key with these scopes.
    Integration(Vec<Scope>),
    /// A staff token with this role.
    Staff(Role),
}

impl Access {
    /// Whether this access includes `scope`. `admin:all` includes every
    /// scope; other scopes include only themselves.
    pub fn allows(&self, scope: Scope) -> bool {
        match self {
            Self::Content => scope == Scope::ContentRead,
            Self::Integration(scopes) => {
                scopes.contains(&Scope::AdminAll) || scopes.contains(&scope)
            }
            Self::Staff(role) => role.allows(scope),
        }
    }
}
