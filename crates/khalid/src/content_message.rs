//! What a person reads when a content file is wrong (RULES.md BL-ERR-4).

use std::path::Path;

use khalid_content::ContentError;
use khalid_core::{Locale, UserMessage};

use crate::config_message::located;

/// Why a front matter mistake stops the run.
const WHY: &str =
    "Khalid reads every post before it runs, so nothing is built from a file it cannot read.";

/// Why a block mistake stops the run.
const BLOCK_WHY: &str =
    "Khalid checks every block before it runs, so no page shows a block it cannot draw.";

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
        ContentError::Block { path, line, source } => {
            located(&at(path), *line, "", &source.problem(), BLOCK_WHY)
        }
        ContentError::Wikilink {
            path,
            line,
            target,
            language,
        } => wikilink(&at(path), *line, target, language),
        ContentError::Snippet {
            path,
            line,
            slug,
            language,
        } => snippet(&at(path), *line, slug, language),
        ContentError::Maths {
            path,
            line,
            problem,
        } => maths(&at(path), *line, problem),
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

/// The message for a `[[target]]` on `line` of the file at `path` that
/// matches no page in `language`.
fn wikilink(path: &str, line: usize, target: &str, language: &Locale) -> UserMessage {
    UserMessage::new(
        format!(
            "{path}, line {line}: [[{target}]] matches no post, page or glossary term in {language}."
        ),
        "A wikilink names a page by its slug, the last part of its address, in the post's own language.",
        format!(
            "Change [[{target}]] on line {line} to the slug of a post, page or glossary term in {language}, or remove the brackets."
        ),
    )
}

/// The message for a snippet block on `line` of the file at `path` that
/// names no snippet in `language`.
fn snippet(path: &str, line: usize, slug: &str, language: &Locale) -> UserMessage {
    UserMessage::new(
        format!("{path}, line {line}: there is no snippet called {slug} in {language}."),
        "A snippet block shows a file from snippets/ in the post's own language, so the post would have a gap.",
        format!(
            "Add content/{language}/snippets/{slug}.md, or change the slug on line {line} to one that exists."
        ),
    )
}

/// The message for a formula on `line` of the file at `path` that cannot be
/// drawn, because of `problem`.
fn maths(path: &str, line: usize, problem: &str) -> UserMessage {
    UserMessage::new(
        format!("{path}, line {line}: the maths cannot be drawn: {problem}."),
        "Khalid draws every formula between $ signs when it builds, so readers see it with scripts off.",
        format!(
            "Correct the formula on line {line}, or write \\$ for a dollar sign that is not maths."
        ),
    )
}
