//! The command line of `khalid keys` and `khalid staff`, the commands that
//! let people and tools into a site. Kept apart from `cli` to keep each
//! file short (BL-SIZE-1); `cli` names them all.

use clap::{Args, Subcommand, ValueEnum};
use khalid_api::keys::{KeyId, Role, Scope};

/// The key commands. Keys are stored as hashes in `data/site.db`.
#[derive(Debug, Clone, Subcommand)]
pub(crate) enum KeysCommand {
    /// Make a key and show it once. Khalid keeps only a hash of it
    Create(NewKeyArgs),
    /// Stop a key from working, at once and for good
    Revoke {
        /// The key's identity: the 16 characters after the first `_`
        #[arg(value_name = "ID", value_parser = |text: &str| KeyId::try_new(text))]
        id: KeyId,
    },
}

/// The staff commands. Accounts are kept in `data/site.db`.
#[derive(Debug, Clone, Subcommand)]
pub(crate) enum StaffCommand {
    /// Make the owner if the site has none, and show a sign-in link once
    Owner(OwnerArgs),
}

/// What `khalid staff owner` takes.
#[derive(Debug, Clone, Args)]
pub(crate) struct OwnerArgs {
    /// The owner's email address, which they sign in with
    #[arg(long)]
    pub(crate) email: String,

    /// The owner's name, as the admin shows it
    #[arg(long)]
    pub(crate) name: String,
}

/// What `khalid keys create` takes.
#[derive(Debug, Clone, Args)]
pub(crate) struct NewKeyArgs {
    /// Who or what uses the key, such as a publishing tool or a name
    #[arg(long)]
    pub(crate) name: String,

    /// What the key is for
    #[arg(long, value_enum, default_value_t = KeyKindArg::Integration)]
    pub(crate) kind: KeyKindArg,

    /// What an integration key may do, such as posts:write. Repeat for more
    #[arg(long = "scope", value_name = "SCOPE")]
    pub(crate) scopes: Vec<Scope>,

    /// A staff token's role: owner, admin, editor, writer or contributor
    #[arg(long)]
    pub(crate) role: Option<Role>,
}

/// The kinds of key, as `--kind` spells them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum KeyKindArg {
    /// Reads published posts through the Content API. Safe in a web page
    Content,
    /// Lets a tool, such as a publishing service, use the Admin API, with scopes
    Integration,
    /// Lets one staff member use the Admin API, with a role
    Staff,
}
