//! `khalid staff owner`: makes the site's first owner, or signs the owner
//! in again, with a sign-in link shown once (plan section 4.6).

use std::path::Path;

use khalid_api::keys::Role;
use khalid_auth::{AccountStore, CreateAccountError, Email, LinkStore, LinkToken};
use khalid_core::{Clock, Config, Slug, SystemClock, UserMessage};
use khalid_db::{AccountTable, Database, LinkTable};
use serde::Serialize;

use crate::cli::{Output, OwnerArgs, StaffCommand};
use crate::exit::Exit;
use crate::{db, db_message, report, staff, staff_message};

/// The JSON shape of `khalid staff owner --json`. `link` appears here and
/// nowhere else, ever.
#[derive(Serialize)]
struct OwnerBody<'a> {
    email: &'a str,
    link: &'a str,
}

/// Runs a staff command on the `site` folder, whose settings are `config`.
pub(crate) fn run(site: &Path, config: &Config, command: &StaffCommand, output: Output) -> Exit {
    match command {
        StaffCommand::Owner(args) => owner(site, config, args, output),
    }
}

/// Makes the owner when the site has none, then shows a sign-in link once.
fn owner(site: &Path, config: &Config, args: &OwnerArgs, output: Output) -> Exit {
    let Ok(email) = args.email.parse::<Email>() else {
        return report::failure(output, &staff_message::bad_email(), Exit::Usage);
    };
    let name = args.name.trim();
    if name.is_empty() {
        return report::failure(output, &staff_message::no_name(), Exit::Usage);
    }
    let Ok(token) = LinkToken::generate() else {
        return report::failure(output, &staff_message::internal(), Exit::Internal);
    };
    let made = on_tables(site, async |accounts, links| {
        let now = SystemClock.now();
        let owner = match accounts.find_by_email(&email).await {
            Ok(Some(found)) if found.account.role() == Role::Owner => found.account,
            Ok(Some(_)) => return Err(staff_message::not_the_owner()),
            Ok(None) => match accounts.create(&email, name, Role::Owner, now).await {
                Ok(account) => account,
                Err(CreateAccountError::OwnerTaken) => return Err(staff_message::owner_taken()),
                Err(_) => return Err(staff_message::not_saved(site)),
            },
            Err(_) => return Err(staff_message::not_saved(site)),
        };
        let saved = links.create(token.hash(), owner.id(), now).await;
        saved.map_err(|_| staff_message::not_saved(site))
    });
    match made {
        Ok(Ok(())) => shown(output, &email, &token, config),
        Ok(Err(message)) | Err(message) => report::failure(output, &message, Exit::Failure),
    }
}

/// Prints the sign-in link: the one time it is ever shown.
fn shown(output: Output, email: &Email, token: &LinkToken, config: &Config) -> Exit {
    let link = token.link(&staff::admin_address(config));
    match output {
        Output::Json => report::json(
            &OwnerBody {
                email: email.as_str(),
                link: link.expose(),
            },
            Exit::Success,
        ),
        Output::Human => report::text(
            &format!(
                "The owner is {email}.\n\
                 Open this link within 15 minutes to sign in. It works once, and this is the\n\
                 only time Khalid shows it:\n\n    {link}\n\n\
                 Once signed in, choose a password or add a passkey.\n",
                email = email.as_str(),
                link = link.expose(),
            ),
            Exit::Success,
        ),
    }
}

/// Runs `work` on the site's accounts and sign-in links, in `data/site.db`.
fn on_tables<T>(
    site: &Path,
    work: impl AsyncFnOnce(&AccountTable, &LinkTable) -> T,
) -> Result<T, UserMessage> {
    let path = db::data_file(site)?;
    let (Ok(main), Ok(runtime)) = (
        Slug::try_new(db::MAIN),
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build(),
    ) else {
        return Err(db_message::internal());
    };
    runtime.block_on(async {
        let database = Database::open(&path)
            .await
            .map_err(|error| db_message::of(&path, &error))?;
        let opened = async {
            database.migrate().await?;
            database.site(&main).await
        }
        .await;
        let done = match opened {
            Ok(id) => {
                let accounts = AccountTable::new(database.clone(), id);
                Ok(work(&accounts, &LinkTable::new(database.clone(), id)).await)
            }
            Err(error) => Err(db_message::of(&path, &error)),
        };
        database.close().await;
        done
    })
}
