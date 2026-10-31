//! `khalid db migrate`, `khalid db rebuild` and `khalid db export`: the
//! database in the site's `data/` folder.

use std::fs;
use std::path::{Path, PathBuf};

use khalid_core::{Slug, UserMessage};
use khalid_db::{ContentIndex, Database, DbError};
use serde::Serialize;

use crate::cli::{DbCommand, Output};
use crate::exit::Exit;
use crate::{db_export, db_message, index, report};

/// The folder inside a site that holds everything Khalid derives or keeps.
pub(crate) const DATA: &str = "data";

/// The database file inside [`DATA`].
pub(crate) const FILE: &str = "site.db";

/// The handle of the one site a site folder holds. Multi-site, in a later
/// phase, gives each site in a process its own.
pub(crate) const MAIN: &str = "main";

/// The JSON shape of a database command's result. `posts` is there after a
/// rebuild.
#[derive(Serialize)]
struct DbBody<'a> {
    database: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    posts: Option<usize>,
}

/// Runs a database command on the `site` folder, whose settings and content were checked.
pub(crate) fn run(site: &Path, command: DbCommand, output: Output) -> Exit {
    let index = match command {
        DbCommand::Export { out } => return db_export::run(site, out.as_deref(), output),
        DbCommand::Migrate => None,
        DbCommand::Rebuild => match index::of(site) {
            Ok(index) => Some(index),
            Err(message) => return report::failure(output, &message, Exit::Failure),
        },
    };
    let path = match data_file(site) {
        Ok(path) => path,
        Err(message) => return report::failure(output, &message, Exit::Failure),
    };
    let (Ok(main), Ok(runtime)) = (
        Slug::try_new(MAIN),
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build(),
    ) else {
        return report::failure(output, &db_message::internal(), Exit::Internal);
    };
    match runtime.block_on(apply(&path, &main, index.as_ref())) {
        Ok(posts) => success(output, &path, posts),
        Err(error) => report::failure(output, &db_message::of(&path, &error), Exit::Failure),
    }
}

/// `data/site.db` in the site folder, after making `data/` when it is missing.
pub(crate) fn data_file(site: &Path) -> Result<PathBuf, UserMessage> {
    let mut folder = site.to_path_buf();
    folder.push(DATA);
    fs::create_dir_all(&folder).map_err(|error| db_message::folder(&folder, &error))?;
    folder.push(FILE);
    Ok(folder)
}

/// Opens the database, making it when it is missing, applies every new
/// migration, then rebuilds the index when there is one to write. Returns how
/// many posts the index holds after a rebuild.
async fn apply(
    path: &Path,
    handle: &Slug,
    index: Option<&ContentIndex>,
) -> Result<Option<usize>, DbError> {
    let db = Database::open(path).await?;
    let done = async {
        db.migrate().await?;
        let Some(index) = index else {
            return Ok(None);
        };
        let site = db.site(handle).await?;
        Ok(Some(db.rebuild(site, index).await?.posts()))
    }
    .await;
    db.close().await;
    done
}

/// Says what the command did.
fn success(output: Output, path: &Path, posts: Option<usize>) -> Exit {
    let shown = path.display().to_string();
    match output {
        Output::Human => {
            let said = match posts {
                Some(posts) => format!("Rebuilt the index in {shown} from {posts} posts.\n"),
                None => format!("{shown} is up to date.\n"),
            };
            report::text(&said, Exit::Success)
        }
        Output::Json => report::json(
            &DbBody {
                database: &shown,
                posts,
            },
            Exit::Success,
        ),
    }
}
