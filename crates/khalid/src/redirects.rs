//! `khalid redirects import`, `export` and `explain`: the site's stored
//! redirects from the command line, through the same `RedirectDesk` the
//! Admin API uses (KH-SEO-8).

use std::fs;
use std::path::Path;
use std::sync::Arc;

use khalid_api::redirect_desk::{RedirectDesk, StaticPages, StoreRefusal};
use khalid_api::redirect_store::Origin;
use khalid_core::{Config, Environment, Slug, SystemClock, UserMessage};
use khalid_db::{Database, NotFoundTable, RedirectRows};
use khalid_seo::{RedirectEntry, UrlPath, read_redirects_csv, write_redirects_csv};
use serde::Serialize;

use crate::cli::{Output, RedirectsCommand};
use crate::exit::Exit;
use crate::{config_message, db, db_message, report};

/// The JSON shape of `khalid redirects import --json`.
#[derive(Serialize)]
struct ImportedBody {
    stored: usize,
    outside_mount: usize,
}

/// The JSON shape of `khalid redirects export --json`.
#[derive(Serialize)]
struct ExportedBody {
    csv: String,
}

/// The JSON shape of `khalid redirects explain --json`.
#[derive(Serialize)]
struct ExplainedBody {
    path: String,
    status: Option<u16>,
    to: Option<String>,
    text: String,
}

/// Runs a redirect command on the `site` folder. Only its settings are
/// read, not its content, so a large site answers at once.
pub(crate) fn run(site: &Path, command: &RedirectsCommand, output: Output) -> Exit {
    let done = match command {
        RedirectsCommand::Import { file } => Config::load(site, &Environment::from_process())
            .map_err(|error| (config_message::of(&error), Exit::Failure))
            .and_then(|config| import(site, &config, file, output)),
        RedirectsCommand::Export => {
            on_desk(site, async |desk| export(desk, output).await).flatten()
        }
        RedirectsCommand::Explain { path } => match UrlPath::try_new(path) {
            Ok(path) => on_desk(site, async |desk| explain(desk, &path, output).await).flatten(),
            Err(error) => Err((not_a_path(&error.to_string()), Exit::Usage)),
        },
    };
    done.unwrap_or_else(|(message, exit)| report::failure(output, &message, exit))
}

type Done = Result<Exit, (UserMessage, Exit)>;

/// Reads `file` and stores every redirect in it, or none.
fn import(site: &Path, config: &Config, file: &Path, output: Output) -> Done {
    let shown = file.display();
    let text = fs::read_to_string(file).map_err(|error| {
        let message = UserMessage::new(
            format!("Khalid could not read {shown}: {error}."),
            "Nothing was stored.",
            "Check the file's name and that it is a text file, then run the command again.",
        );
        (message, Exit::Usage)
    })?;
    let entries = read_redirects_csv(&text).map_err(|error| {
        let message = UserMessage::new(
            format!("The list in {shown} cannot be stored: {error}."),
            "Nothing from it was stored, so no half list sends readers astray.",
            "Fix that row, keep the header from,to,status,match, and run the command again.",
        );
        (message, Exit::Failure)
    })?;
    let outside = entries
        .iter()
        .filter(|entry| outside(config, entry))
        .count();
    let stored = on_desk(site, async |desk| {
        desk.store_checked(entries, Origin::Import).await
    })?;
    let stored = stored
        .map_err(|refusal| (refused(&refusal), Exit::Failure))?
        .len();
    Ok(match output {
        Output::Json => report::json(
            &ImportedBody {
                stored,
                outside_mount: outside,
            },
            Exit::Success,
        ),
        Output::Human => {
            let mount = config.mount();
            let note = if outside > 0 {
                format!(
                    "{outside} of them start outside {mount}. Khalid never sees those addresses, \
                     so the main site's server must redirect them.\n"
                )
            } else {
                String::new()
            };
            let text = format!(
                "Stored {stored} redirects from {shown}.\n{note}\
                 A running khalid serve answers them once it is started again.\n"
            );
            report::text(&text, Exit::Success)
        }
    })
}

/// Whether an exact rule starts outside the blog's mount folder.
fn outside(config: &Config, entry: &RedirectEntry) -> bool {
    matches!(entry, RedirectEntry::Exact(rule) if !rule.from.as_str().starts_with(config.mount()))
}

async fn export(desk: &RedirectDesk, output: Output) -> Done {
    let table = desk
        .table()
        .await
        .map_err(|refusal| (refused(&refusal), Exit::Failure))?;
    let csv = write_redirects_csv(&table.entries());
    Ok(match output {
        Output::Json => report::json(&ExportedBody { csv }, Exit::Success),
        Output::Human => report::text(&csv, Exit::Success),
    })
}

async fn explain(desk: &RedirectDesk, path: &UrlPath, output: Output) -> Done {
    let table = desk
        .table()
        .await
        .map_err(|refusal| (refused(&refusal), Exit::Failure))?;
    let explained = table.explain(path);
    let text = explained.to_string();
    Ok(match output {
        Output::Json => {
            let answer = explained.answer.as_ref();
            let body = ExplainedBody {
                path: path.to_string(),
                status: answer.map(khalid_seo::Redirect::status),
                to: answer.and_then(|a| a.location()).map(ToString::to_string),
                text,
            };
            report::json(&body, Exit::Success)
        }
        Output::Human => report::text(&text, Exit::Success),
    })
}

fn not_a_path(reason: &str) -> UserMessage {
    UserMessage::new(
        format!("That is not an address Khalid can explain: {reason}."),
        "Only a path from the site root can match a redirect.",
        "Give the address as it appears after the domain, such as /blog/old-links/.",
    )
}

fn refused(refusal: &StoreRefusal) -> UserMessage {
    UserMessage::new(
        format!("The redirects cannot be stored: {refusal}."),
        "Nothing was stored, so readers are never sent round in a loop or to two places.",
        "Remove the row that loops or repeats an address, or remove the stored redirect it \
         clashes with, then run the command again.",
    )
}

/// Opens `data/site.db` in `site`, runs `work` with the main site's
/// redirect desk, and closes it.
pub(crate) fn on_desk<T>(
    site: &Path,
    work: impl AsyncFnOnce(&RedirectDesk) -> T,
) -> Result<T, (UserMessage, Exit)> {
    let failed = |message| (message, Exit::Failure);
    let path = db::data_file(site).map_err(failed)?;
    let (Ok(main), Ok(runtime)) = (
        Slug::try_new(db::MAIN),
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build(),
    ) else {
        return Err((db_message::internal(), Exit::Internal));
    };
    runtime.block_on(async {
        let database = Database::open(&path)
            .await
            .map_err(|e| failed(db_message::of(&path, &e)))?;
        let opened = async {
            database.migrate().await?;
            database.site(&main).await
        };
        let done = match opened.await {
            Ok(id) => {
                let desk = RedirectDesk::new(
                    Arc::new(RedirectRows::new(database.clone(), id)),
                    Arc::new(NotFoundTable::new(database.clone(), id)),
                    Arc::new(StaticPages::default()),
                    Arc::new(SystemClock),
                );
                Ok(work(&desk).await)
            }
            Err(error) => Err(failed(db_message::of(&path, &error))),
        };
        database.close().await;
        done
    })
}
