//! The stacks a main site can be built with.

use std::fmt;
use std::str::FromStr;

use crate::MountError;

/// A stack the main site runs on. Webflow and Shopify use the Worker (ADR 0017).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Stack {
    /// An nginx `server` block.
    Nginx,
    /// A Caddyfile site block.
    Caddy,
    /// An Apache `VirtualHost`, with `mod_proxy` and `mod_ssl`.
    Apache,
    /// A Traefik dynamic configuration file.
    Traefik,
    /// A Cloudflare Worker on the main site's domain.
    CloudflareWorker,
    /// Rewrites in `vercel.json`.
    Vercel,
    /// Rewrites in `next.config.js`.
    Nextjs,
    /// A Netlify `_redirects` file.
    Netlify,
    /// Webflow, through a Cloudflare Worker.
    Webflow,
    /// Shopify, through a Cloudflare Worker.
    Shopify,
}

impl Stack {
    /// Every stack, in the order the help text lists them.
    #[rustfmt::skip]
    pub const ALL: [Self; 10] = [
        Self::Nginx, Self::Caddy, Self::Apache, Self::Traefik, Self::CloudflareWorker,
        Self::Vercel, Self::Nextjs, Self::Netlify, Self::Webflow, Self::Shopify,
    ];

    /// The name the command line takes, such as `cloudflare-worker`.
    pub fn name(self) -> &'static str {
        match self {
            Self::Nginx => "nginx",
            Self::Caddy => "caddy",
            Self::Apache => "apache",
            Self::Traefik => "traefik",
            Self::CloudflareWorker => "cloudflare-worker",
            Self::Vercel => "vercel",
            Self::Nextjs => "nextjs",
            Self::Netlify => "netlify",
            Self::Webflow => "webflow",
            Self::Shopify => "shopify",
        }
    }
}

impl fmt::Display for Stack {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for Stack {
    type Err = MountError;

    /// Reads a name, ignoring case, spaces, dots and hyphens: `Next.js` works.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let bare = |name: &str| -> String {
            name.chars()
                .filter(char::is_ascii_alphanumeric)
                .map(|c| c.to_ascii_lowercase())
                .collect()
        };
        Self::ALL
            .into_iter()
            .find(|stack| bare(stack.name()) == bare(text))
            .ok_or_else(|| MountError::UnknownStack {
                name: text.trim().to_owned(),
            })
    }
}
