//! The checks `khalid doctor` runs on what sits beside the content: the
//! database, the theme tokens, and whether Khalid may write where it saves.

use std::path::{Path, PathBuf};

use khalid_core::UserMessage;
use khalid_db::Database;
use khalid_render::Tokens;

use crate::db::{DATA, FILE};
use crate::doctor_check::{Check, Finding};
use crate::{build_message, db_message};

/// The theme's tokens file inside a site folder.
const TOKENS: &str = "theme/tokens.toml";

/// The folders Khalid saves into, as a person reads their names. The site
/// folder itself comes first, as `""`.
const SAVED_INTO: [&str; 4] = ["", "content", DATA, "theme"];

/// Checks the database, the theme tokens and the folders of `site`.
pub(crate) fn check(site: &Path) -> Vec<Finding> {
    vec![database(site), theme(site), permissions(site)]
}

/// Opens `data/site.db` when it exists. A site with none yet is fine: the
/// commands that need it make it.
fn database(site: &Path) -> Finding {
    let path = inside(site, &format!("{DATA}/{FILE}"));
    if !path.exists() {
        return Finding::ok(
            Check::Database,
            format!("There is no database yet, and Khalid makes {DATA}/{FILE} when it needs it."),
        );
    }
    let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    else {
        return Finding::problem(Check::Database, db_message::internal());
    };
    let opened = runtime.block_on(async {
        let db = Database::open(&path).await?;
        db.close().await;
        Ok(())
    });
    match opened {
        Ok(()) => Finding::ok(Check::Database, format!("{DATA}/{FILE} opens.")),
        Err(error) => Finding::problem(Check::Database, db_message::of(&path, &error)),
    }
}

/// Reads `theme/tokens.toml` the way the pages do, and checks every text
/// colour is readable on the background.
fn theme(site: &Path) -> Finding {
    if !inside(site, TOKENS).exists() {
        return Finding::ok(
            Check::Theme,
            format!("There is no {TOKENS}, so pages use the light default colours."),
        );
    }
    match Tokens::load(site) {
        Ok(tokens) => {
            let faint: Vec<&str> = tokens
                .contrast_failures()
                .iter()
                .map(|failure| failure.token)
                .collect();
            if faint.is_empty() {
                Finding::ok(
                    Check::Theme,
                    format!("{TOKENS} reads, and every text colour in it is readable."),
                )
            } else {
                Finding::problem(Check::Theme, too_faint(&faint))
            }
        }
        Err(error) => Finding::problem(Check::Theme, build_message::tokens(&error)),
    }
}

/// The message for the colours named in `faint`, too faint to read.
fn too_faint(faint: &[&str]) -> UserMessage {
    let verb = if faint.len() == 1 { "is" } else { "are" };
    UserMessage::new(
        format!(
            "{} in {TOKENS} {verb} too faint to read on the background.",
            faint.join(" and ")
        ),
        "WCAG AA, the web's standard for readable pages, asks for 4.5 to 1, so some readers cannot see that text.",
        format!("Choose a darker shade for each in {TOKENS}, then run `khalid doctor` again."),
    )
}

/// Tries to write a hidden empty file in each folder Khalid saves into that
/// exists, and removes it at once.
fn permissions(site: &Path) -> Finding {
    let refused: Vec<String> = SAVED_INTO
        .iter()
        .filter_map(|name| {
            let folder = inside(site, name);
            let writable = !folder.is_dir()
                || tempfile::Builder::new()
                    .prefix(".khalid-doctor-")
                    .tempfile_in(&folder)
                    .is_ok();
            (!writable).then(|| shown(site, name))
        })
        .collect();
    if refused.is_empty() {
        return Finding::ok(
            Check::Permissions,
            "Khalid can write to every folder it saves into.",
        );
    }
    Finding::problem(
        Check::Permissions,
        UserMessage::new(
            format!("Khalid cannot write to {}.", refused.join(", ")),
            "Khalid saves posts, the database, the theme and built pages there, so those commands would fail.",
            "Give the user who runs Khalid write permission there, for example with `chmod -R u+w`.",
        ),
    )
}

/// `name` inside `site`. Every name here is a constant of this module.
fn inside(site: &Path, name: &str) -> PathBuf {
    let mut path = site.to_path_buf();
    if !name.is_empty() {
        path.push(name);
    }
    path
}

/// How a folder reads in a message: `content/`, or the site folder's path.
fn shown(site: &Path, name: &str) -> String {
    if name.is_empty() {
        site.display().to_string()
    } else {
        format!("{name}/")
    }
}
