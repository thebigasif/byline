//! `khalid db migrate`: the database in the site's `data/` folder.

use std::fs;
use std::path::Path;

use khalid_core::UserMessage;
use khalid_db::Database;
use serde::Serialize;

use crate::cli::{DbCommand, Output};
use crate::db_message;
use crate::exit::Exit;
use crate::report;

/// The folder inside a site that holds everything Khalid derives or keeps.
const DATA: &str = "data";

/// The database file inside [`DATA`].
const FILE: &str = "site.db";

/// The JSON shape of a database command's result.
#[derive(Serialize)]
struct DbBody<'a> {
    database: &'a str,
}

/// Runs a database command on the `site` folder, whose settings were checked.
pub(crate) fn run(site: &Path, command: DbCommand, output: Output) -> Exit {
    if command == DbCommand::Rebuild {
        return report::failure(output, &not_yet(), Exit::Internal);
    }
    let mut folder = site.to_path_buf();
    folder.push(DATA);
    let mut path = folder.clone();
    path.push(FILE);
    if let Err(error) = fs::create_dir_all(&folder) {
        return report::failure(output, &db_message::folder(&folder, &error), Exit::Failure);
    }
    let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    else {
        return report::failure(output, &db_message::runtime(), Exit::Internal);
    };
    match runtime.block_on(migrate(&path)) {
        Ok(()) => success(output, &path),
        Err(error) => report::failure(output, &db_message::of(&path, &error), Exit::Failure),
    }
}

/// Opens the database, making it when it is missing, and applies every
/// migration it does not have yet.
async fn migrate(path: &Path) -> Result<(), khalid_db::DbError> {
    let db = Database::open(path).await?;
    let migrated = db.migrate().await;
    db.close().await;
    migrated
}

/// Says the database is up to date.
fn success(output: Output, path: &Path) -> Exit {
    let shown = path.display().to_string();
    match output {
        Output::Human => report::text(&format!("{shown} is up to date.\n"), Exit::Success),
        Output::Json => report::json(&DbBody { database: &shown }, Exit::Success),
    }
}

/// The message for `khalid db rebuild` until it reads the content tree.
fn not_yet() -> UserMessage {
    UserMessage::new(
        "The command `khalid db rebuild` is not built yet.",
        "It arrives in the next part of this phase.",
        "Run `khalid db migrate` to make the database for now.",
    )
}
