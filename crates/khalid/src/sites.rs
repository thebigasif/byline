//! `khalid sites list`, `add` and `admin`: the sites one process serves,
//! from the command line, through the same list the super-admin panel
//! uses (plan 4.4, PRD KH-EDT-7).

use std::fmt::Write;
use std::path::Path;

use khalid_auth::Email;
use khalid_core::{DomainName, UserMessage};
use serde::Serialize;

use crate::cli::{Output, SitesCommand};
use crate::exit::Exit;
use crate::site_folders::{self, AddError};
use crate::site_list::{self, SiteList};
use crate::{report, sites_message};

/// The JSON shape of `khalid sites list --json` and `add --json`.
#[derive(Serialize)]
struct SitesBody {
    sites: Vec<SiteBody>,
}

/// One site as `--json` prints it.
#[derive(Serialize)]
struct SiteBody {
    domain: String,
    folder: String,
    home: bool,
}

/// The JSON shape of `khalid sites admin --json`.
#[derive(Serialize)]
struct AdminBody<'a> {
    super_admin: &'a str,
}

/// Runs a sites command on the sites folder `root`.
pub(crate) fn run(root: &Path, command: &SitesCommand, output: Output) -> Exit {
    let done = match command {
        SitesCommand::List => existing(root).map(|list| listed(root, &list, output)),
        SitesCommand::Add { domain } => added(root, domain, output),
        SitesCommand::Admin { email } => admin(root, email, output),
    };
    done.unwrap_or_else(|message| report::failure(output, &message, Exit::Failure))
}

/// The list in `root`, which must be a sites folder.
fn existing(root: &Path) -> Result<SiteList, UserMessage> {
    SiteList::read(root)?.ok_or_else(|| sites_message::not_a_sites_folder(root))
}

fn added(root: &Path, domain: &DomainName, output: Output) -> Result<Exit, UserMessage> {
    let list = match site_folders::add(root, domain) {
        Ok(list) => list,
        Err(AddError::Taken) => return Err(sites_message::taken(domain.as_str())),
        Err(AddError::Failed(message)) => return Err(message),
    };
    if output == Output::Json {
        return Ok(listed(root, &list, output));
    }
    let folder = site_list::folder(root, domain);
    Ok(report::text(
        &format!(
            "Added {} in {}. A running khalid serve starts it within a moment; \
             point the domain's DNS at this server.\n",
            domain.as_str(),
            folder.display()
        ),
        Exit::Success,
    ))
}

fn admin(root: &Path, email: &Email, output: Output) -> Result<Exit, UserMessage> {
    let mut list = existing(root)?;
    list.with_super_admin(email.clone());
    list.write(root)
        .map_err(|_| sites_message::not_saved(&site_list::file(root)))?;
    Ok(match output {
        Output::Json => report::json(
            &AdminBody {
                super_admin: email.as_str(),
            },
            Exit::Success,
        ),
        Output::Human => report::text(
            &format!(
                "{} is a super admin. They add sites from the admin of the home site, {}.\n",
                email.as_str(),
                list.domains().first().map_or("", DomainName::as_str),
            ),
            Exit::Success,
        ),
    })
}

/// Prints the sites in `list`, the home site first.
fn listed(root: &Path, list: &SiteList, output: Output) -> Exit {
    let sites = list
        .domains()
        .iter()
        .enumerate()
        .map(|(at, domain)| SiteBody {
            domain: domain.as_str().to_owned(),
            folder: site_list::folder(root, domain).display().to_string(),
            home: at == 0,
        });
    let sites: Vec<SiteBody> = sites.collect();
    if output == Output::Json {
        return report::json(&SitesBody { sites }, Exit::Success);
    }
    let mut text = String::new();
    for site in &sites {
        let home = if site.home { " (home)" } else { "" };
        let _ = writeln!(text, "{}{home}  {}", site.domain, site.folder);
    }
    report::text(&text, Exit::Success)
}
