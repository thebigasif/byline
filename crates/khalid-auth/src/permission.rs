//! What each role may do, and which accounts it may manage.

use std::str::FromStr;

use khalid_api::keys::Role;

/// One thing a staff member may do in the admin. The crate's `README.md`
/// shows the same table, and a test keeps the two equal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Permission {
    /// Write drafts and edit the posts they wrote.
    WriteOwnPosts,
    /// Publish, schedule and unpublish the posts they wrote.
    PublishOwnPosts,
    /// Edit, publish and delete anyone's posts.
    EditOthersPosts,
    /// Upload images and files to the media library.
    UploadMedia,
    /// Change tags, categories, authors, collections and snippets.
    ManageTaxonomy,
    /// Approve, hide and delete comments.
    ModerateComments,
    /// Add, change and remove newsletter subscribers.
    ManageSubscribers,
    /// Change site settings, the design, navigation and redirects.
    ManageSettings,
    /// Make and revoke API keys and webhooks.
    ManageIntegrations,
    /// Invite staff, change their roles and remove them, within
    /// [`may_manage`].
    ManageStaff,
    /// Hand the site to another account.
    TransferOwnership,
}

impl Permission {
    /// Every permission, in the order the README lists them.
    pub const ALL: [Self; 11] = [
        Self::WriteOwnPosts,
        Self::PublishOwnPosts,
        Self::EditOthersPosts,
        Self::UploadMedia,
        Self::ManageTaxonomy,
        Self::ModerateComments,
        Self::ManageSubscribers,
        Self::ManageSettings,
        Self::ManageIntegrations,
        Self::ManageStaff,
        Self::TransferOwnership,
    ];

    /// The name, such as `posts:publish-own`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::WriteOwnPosts => "posts:write-own",
            Self::PublishOwnPosts => "posts:publish-own",
            Self::EditOthersPosts => "posts:edit-others",
            Self::UploadMedia => "media:upload",
            Self::ManageTaxonomy => "taxonomy:manage",
            Self::ModerateComments => "comments:moderate",
            Self::ManageSubscribers => "subscribers:manage",
            Self::ManageSettings => "settings:manage",
            Self::ManageIntegrations => "integrations:manage",
            Self::ManageStaff => "staff:manage",
            Self::TransferOwnership => "site:transfer",
        }
    }

    /// Whether `role` has this permission.
    #[must_use]
    pub fn granted_to(self, role: Role) -> bool {
        match role {
            Role::Owner => true,
            Role::Admin => self != Self::TransferOwnership,
            Role::Editor => matches!(
                self,
                Self::WriteOwnPosts
                    | Self::PublishOwnPosts
                    | Self::EditOthersPosts
                    | Self::UploadMedia
                    | Self::ManageTaxonomy
                    | Self::ModerateComments
                    | Self::ManageStaff
            ),
            Role::Writer => matches!(
                self,
                Self::WriteOwnPosts | Self::PublishOwnPosts | Self::UploadMedia
            ),
            // Role is non_exhaustive: a role added later gets the least.
            _ => self == Self::WriteOwnPosts,
        }
    }
}

impl FromStr for Permission {
    type Err = ParsePermissionError;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|permission| permission.as_str() == name)
            .ok_or_else(|| ParsePermissionError(name.to_owned()))
    }
}

/// A name that is not a permission.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0} is not a staff permission")]
pub struct ParsePermissionError(String);

/// Whether an account with role `actor` may invite, change or remove an
/// account with role `target`. Nobody manages the owner: the site changes
/// hands only through [`Permission::TransferOwnership`]. An admin manages
/// admins and everyone below; an editor manages writers and contributors.
#[must_use]
pub fn may_manage(actor: Role, target: Role) -> bool {
    if target == Role::Owner || !Permission::ManageStaff.granted_to(actor) {
        return false;
    }
    match actor {
        Role::Owner | Role::Admin => true,
        Role::Editor => matches!(target, Role::Writer | Role::Contributor),
        _ => false,
    }
}

/// Whether `actor` may move an account from role `from` to role `to`: it
/// must manage both, so nobody hands out more than they hold.
#[must_use]
pub fn may_assign(actor: Role, from: Role, to: Role) -> bool {
    may_manage(actor, from) && may_manage(actor, to)
}
