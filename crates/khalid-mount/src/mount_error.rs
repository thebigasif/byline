//! Why a proxy rule could not be made.

/// Why a stack name, a mount path or the blog's address was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum MountError {
    /// The stack name is not one we print a rule for.
    #[error(
        "no proxy rule for `{name}`; the stacks are nginx, caddy, apache, traefik, \
         cloudflare-worker, vercel, nextjs, netlify, webflow and shopify"
    )]
    UnknownStack {
        /// The name as given.
        name: String,
    },
    /// The mount path is `/`. A blog at the root of its own address needs
    /// no proxy rule (plan section 1, decision 1).
    #[error("the blog is at the root, so no proxy rule is needed")]
    Root,
    /// No leading `/`, a trailing `/`, or an empty, `.` or `..` folder.
    #[error("the mount path must look like `/blog`, with no slash at the end")]
    Shape,
    /// A character outside letters, digits, `-`, `_`, `.` and `~`.
    #[error("the mount path has the character {character:?}")]
    Character {
        /// The first character refused.
        character: char,
    },
    /// The blog's address has a path, query or fragment, not just an origin.
    #[error("the blog's address must have no path or query")]
    UpstreamPath,
}
