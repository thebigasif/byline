//! What a person reads when a content file is wrong (RULES.md BL-ERR-4).

use std::path::Path;

use khalid_content::ContentError;
use khalid_core::UserMessage;

use crate::config_message::located;

/// Why a front matter mistake stops the run.
const WHY: &str =
    "Khalid reads every post before it runs, so nothing is built from a file it cannot read.";

/// Turns a content error into what happened, why it matters and what to do.
/// Paths in content errors are relative to `site`, so messages join the two.
pub(crate) fn of(site: &Path, error: &ContentError) -> UserMessage {
    let at = |path: &Path| format!("{}/{}", site.display(), path.display());
    match error {
        ContentError::Field {
            path,
            line,
            field,
            problem,
            ..
        } => located(&at(path), *line, field, problem, WHY),
        ContentError::NoFrontMatter { path } => UserMessage::new(
            format!("{} does not start with front matter.", at(path)),
            "Front matter between two --- lines holds the title, date and author that readers and search engines need.",
            format!(
                "Add a --- line, the post's fields and another --- line at the top of {}.",
                at(path)
            ),
        ),
        ContentError::UnclosedFrontMatter { path } => UserMessage::new(
            format!(
                "{} opens its front matter with --- but never closes it.",
                at(path)
            ),
            "Without the closing line, Khalid cannot tell where the fields end and the post begins.",
            format!("Add a --- line after the last field in {}.", at(path)),
        ),
        ContentError::FolderWithoutIndex { path } => UserMessage::new(
            format!("{} is a post folder with no index.md.", at(path)),
            "A post folder holds the post in index.md and its images beside it.",
            format!(
                "Add index.md to {}, or move its files out of the folder.",
                at(path)
            ),
        ),
        ContentError::NotALanguage { path, .. } => UserMessage::new(
            format!("{} is not named after a language.", at(path)),
            "Each folder in content/ holds one language, named with its code, such as en or de.",
            format!(
                "Rename {} to a language code such as en, or move it out of content/.",
                at(path)
            ),
        ),
        ContentError::Read { path, .. } => UserMessage::new(
            format!("Khalid could not read {}.", at(path)),
            "Every content file has to be readable for the site to be built.",
            format!("Check that {} exists and that you can open it.", at(path)),
        ),
        _ => UserMessage::new(
            "Khalid could not read the site's content.",
            WHY,
            "Check the files under content/ in the folder given to --site.",
        ),
    }
}
