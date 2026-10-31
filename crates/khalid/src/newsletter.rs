//! The newsletter's pages on `khalid serve` (tasks 5.5 and 5.6). Confirming
//! a sign-up needs an email, so a site without `[mail]` in `site.toml` says
//! on every newsletter page that sign-ups are not open yet, and stores
//! nothing. With `[mail]`, subscribers are kept in the site's database and
//! the confirmation leaves the way `[mail]` names.

use std::path::Path;
use std::sync::Arc;

use axum::Router;
use khalid_core::{Config, MailTransport, Secret, SystemClock, UserMessage};
use khalid_db::{Database, SiteId, SubscriberTable};
use khalid_mail::{ApiMailer, Sender, Service, SmtpMailer};
use khalid_newsletter::{
    Mailer, Newsletter, NewsletterSite, Pages, PostalAddress, SiteLinks, Welcomes,
};
use khalid_outbound::{Outbound, Resolver};

use crate::{newsletter_files, serve_message};

/// The newsletter's routes for the site in `folder` with `config`, in its
/// default language, taking sign-ups for the newsletters in
/// `content/newsletters/`, keeping subscribers in site `id` of `database`
/// and sending with the mail `secret` from the environment, the APIs
/// through `outbound`.
pub(crate) fn router(
    (folder, config): (&Path, &Config),
    (database, id): (&Database, SiteId),
    (secret, outbound): (Option<Secret<String>>, &Outbound),
) -> Result<Router, UserMessage> {
    let site = site(config);
    let pages = Pages::new(site.clone()).map_err(|_| serve_message::internal())?;
    let newsletter = match mailer(config, secret, outbound)? {
        Some(mailer) => {
            let store = Arc::new(SubscriberTable::new(database.clone(), id));
            let made = Newsletter::new(store, mailer, Arc::new(SystemClock), site);
            let made = made.map_err(|_| serve_message::internal())?;
            Some(with_files(made, folder, config))
        }
        None => None,
    };
    Ok(khalid_newsletter::router(pages, newsletter))
}

/// `newsletter` with the lists and welcome emails in the site's
/// `content/newsletters/`. Welcomes go out only once `[mail]` has a postal
/// address, as every newsletter email ends with it.
fn with_files(newsletter: Newsletter, folder: &Path, config: &Config) -> Newsletter {
    let lists = newsletter_files::lists(folder);
    let postal = config
        .mail()
        .and_then(|mail| mail.postal_address.as_deref());
    let welcomes = postal
        .and_then(PostalAddress::new)
        .map(|postal| Welcomes::new(newsletter_files::welcomes(folder, &lists), postal));
    let newsletter = newsletter.with_lists(lists);
    match welcomes {
        Some(welcomes) => newsletter.with_welcomes(welcomes),
        None => newsletter,
    }
}

/// The site the emails come from, in its default language.
pub(crate) fn site(config: &Config) -> NewsletterSite {
    NewsletterSite {
        name: config.name().to_owned(),
        locale: config.default_language().clone(),
        links: SiteLinks::new(config.address(), config.mount()),
    }
}

/// The way mail leaves, as `[mail]` names it, or `None` when the site
/// sends none. The settings and the secret were checked at start, so a
/// failure here is Khalid's own.
pub(crate) fn mailer(
    config: &Config,
    secret: Option<Secret<String>>,
    outbound: &Outbound,
) -> Result<Option<Arc<dyn Mailer>>, UserMessage> {
    let Some(mail) = config.mail() else {
        return Ok(None);
    };
    let sender = Sender::from(mail);
    let service = match &mail.transport {
        MailTransport::Smtp(smtp) => {
            let smtp = SmtpMailer::new(smtp.clone(), secret, sender, Resolver::new());
            return Ok(Some(Arc::new(smtp)));
        }
        MailTransport::Postmark(postmark) => Service::Postmark {
            message_stream: postmark.message_stream.clone(),
        },
        MailTransport::Resend => Service::Resend,
        _ => return Err(serve_message::internal()),
    };
    let key = secret.ok_or_else(serve_message::internal)?;
    let api = ApiMailer::new(service, key, sender, outbound.clone())
        .map_err(|_| serve_message::internal())?;
    Ok(Some(Arc::new(api)))
}
