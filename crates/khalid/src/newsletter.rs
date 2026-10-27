//! The newsletter's pages on `khalid serve` (task 5.5). Confirming a
//! sign-up needs an email, and the mail settings come with task 5.6, so
//! until a site can send, every newsletter page says sign-ups are not open
//! yet and nothing is stored.

use axum::Router;
use khalid_core::{Config, UserMessage};
use khalid_newsletter::{NewsletterSite, Pages, SiteLinks};

use crate::serve_message;

/// The newsletter's routes for the site with `config`, in its default
/// language.
pub(crate) fn router(config: &Config) -> Result<Router, UserMessage> {
    let site = NewsletterSite {
        name: config.name().to_owned(),
        locale: config.default_language().clone(),
        links: SiteLinks::new(config.address(), config.mount()),
    };
    let pages = Pages::new(site).map_err(|_| serve_message::internal())?;
    Ok(khalid_newsletter::router(pages, None))
}
