//! The broken links `khalid audit` finds, raised to the site's owner by
//! the `alert.raised` webhook and `[alerts] email` (KH-GRO-6). An audit
//! sees the whole site, so a link fixed since the last audit is forgotten
//! and raised again if it breaks again. Only links within the blog count:
//! links to other sites are checked only when the audit is not offline,
//! and the audit's report lists them.

use std::path::Path;

use khalid_alerts::{Finding, Kind};
use khalid_audit::{Check, Report};
use khalid_core::{Config, Secret};
use tokio::runtime::Runtime;

use crate::cli::Cli;
use crate::{alerts, announce, newsletter, outbound, serve};

/// Raises the broken links in `audit` of the `site` folder with `config`,
/// emailing with the `[mail]` secret `mail`. Nothing here changes the
/// audit's answer, so a failure is only logged.
pub(crate) fn raise(
    (site, config): (&Path, &Config),
    audit: &Report,
    mail: Option<Secret<String>>,
    cli: &Cli,
    runtime: &Runtime,
) {
    let found = findings(audit);
    let (Ok(outbound), Ok((database, id))) = (outbound::client(cli), serve::open(site, runtime))
    else {
        tracing::warn!("the broken links could not be raised as an alert");
        return;
    };
    runtime.block_on(async {
        let Ok((telling, announcing)) = announce::once(config, (&database, id), &outbound) else {
            return;
        };
        let mailer = newsletter::mailer(config, mail, &outbound).ok().flatten();
        let alerting = alerts::for_site(config, (&database, id), mailer, telling);
        alerting.raise(Kind::BrokenLinks, found).await;
        drop(alerting);
        // Waits for the webhook deliveries, which a failed one keeps in its log.
        announcing.stop().await;
    });
    runtime.block_on(database.close());
}

/// One finding for each link on the blog to a page that is broken.
fn findings(audit: &Report) -> Vec<Finding> {
    audit
        .issues()
        .iter()
        .filter(|issue| issue.check() == Check::BrokenLink)
        .map(|issue| {
            let line = format!("{}: {}", issue.url(), issue.detail());
            Finding {
                key: Kind::BrokenLinks.key(&line),
                line,
            }
        })
        .collect()
}
