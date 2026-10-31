//! The newsletter's pages on `khalid serve` (tasks 5.5 and 5.6). Confirming
//! a sign-up needs an email, so a site without `[mail]` in `site.toml` says
//! on every newsletter page that sign-ups are not open yet, and stores
//! nothing. With `[mail]`, subscribers are kept in the site's database and
//! the confirmation leaves the way `[mail]` names.

use std::sync::Arc;

use axum::Router;
use khalid_core::{Config, MailTransport, Secret, SystemClock, UserMessage};
use khalid_db::{Database, SiteId, SubscriberTable};
use khalid_mail::{ApiMailer, Sender, Service, SmtpMailer};
use khalid_newsletter::{Mailer, Newsletter, NewsletterSite, Pages, SiteLinks};
use khalid_outbound::{Outbound, Resolver};

use crate::serve_message;

/// The newsletter's routes for the site with `config`, in its default
/// language, keeping subscribers in site `id` of `database` and sending
/// through `mailer`, see [`mailer`].
pub(crate) fn router(
    config: &Config,
    (database, id): (&Database, SiteId),
    mailer: Option<Arc<dyn Mailer>>,
) -> Result<Router, UserMessage> {
    let site = NewsletterSite {
        name: config.name().to_owned(),
        locale: config.default_language().clone(),
        links: SiteLinks::new(config.address(), config.mount()),
    };
    let pages = Pages::new(site.clone()).map_err(|_| serve_message::internal())?;
    let newsletter = match mailer {
        Some(mailer) => {
            let store = Arc::new(SubscriberTable::new(database.clone(), id));
            let made = Newsletter::new(store, mailer, Arc::new(SystemClock), site);
            Some(made.map_err(|_| serve_message::internal())?)
        }
        None => None,
    };
    Ok(khalid_newsletter::router(pages, newsletter))
}

/// The way mail leaves, as `[mail]` names it, with the mail `secret` from
/// the environment and the APIs reached through `outbound`, or `None` when
/// the site sends none. The settings and the secret were checked at start,
/// so a failure here is Khalid's own.
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
