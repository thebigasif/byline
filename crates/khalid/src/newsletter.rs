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
    Mailer, Newsletter, NewsletterSite, Pages, PostalAddress, ReaderPass, SiteLinks, Welcomes,
};
use khalid_outbound::{Outbound, Resolver};
use tokio::runtime::Runtime;

use crate::announce::SiteAnnounce;
use crate::subscriber_events::AnnouncedSubscribers;
use crate::{newsletter_files, serve_message};

/// The newsletter's routes for the site in `folder` with `config`, in its
/// default language, taking sign-ups for the newsletters in
/// `content/newsletters/`, keeping subscribers in site `id` of `database`,
/// telling `announce` of each one confirmed, and sending through `mailer`,
/// the way [`mailer`] made from `[mail]`. A reader who confirms gets the
/// reader cookie, signed with the site's preview key read on `runtime`.
pub(crate) fn router(
    (folder, config): (&Path, &Config),
    (database, id): (&Database, SiteId),
    (runtime, announce): (&Runtime, &SiteAnnounce),
    mailer: Option<Arc<dyn Mailer>>,
) -> Result<Router, UserMessage> {
    let site = site(config);
    let pages = Pages::new(site.clone()).map_err(|_| serve_message::internal())?;
    let newsletter = match mailer {
        Some(mailer) => {
            let table = Arc::new(SubscriberTable::new(database.clone(), id));
            let store = Arc::new(AnnouncedSubscribers::new(table, announce.clone()));
            let key = runtime.block_on(database.preview_key(id, &SystemClock));
            let key = key.map_err(|_| serve_message::internal())?;
            let made = Newsletter::new(store, mailer, Arc::new(SystemClock), site);
            let made = made.map_err(|_| serve_message::internal())?;
            let made = made.with_pass(reader_pass(config, &key));
            Some(with_files(made, folder, config))
        }
        None => None,
    };
    Ok(khalid_newsletter::router(pages, newsletter))
}

/// The reader pass of the site with `config`, signed with its `key`.
pub(crate) fn reader_pass(config: &Config, key: &Secret<[u8; 32]>) -> ReaderPass {
    let links = SiteLinks::new(config.address(), config.mount());
    ReaderPass::new(key, &links, Arc::new(SystemClock))
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

/// The way mail leaves, sending with the `secret` from the environment
/// and reaching the APIs through `outbound`, as `[mail]` names it, or
/// `None` when the site sends none. The newsletter, the alerts and the
/// form answers share it. The settings and the secret were checked at
/// start, so a failure here is Khalid's own.
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
