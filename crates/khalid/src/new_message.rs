//! What a person reads when `khalid new` cannot write a post (RULES.md BL-CLI-3).

use std::io;
use std::path::Path;

use khalid_core::{Locale, UserMessage};

use crate::new_template::Template;

/// The message when `--language` names a language the site does not list.
pub(crate) fn language(locale: &Locale, languages: &[Locale]) -> UserMessage {
    let listed: Vec<&str> = languages.iter().map(Locale::as_str).collect();
    UserMessage::new(
        format!("The site does not publish in {locale}."),
        "Each post goes in the folder of a language listed in site.toml.",
        format!(
            "Use one of {} with --language, or add {locale} to languages in site.toml.",
            listed.join(", ")
        ),
    )
}

/// The message when the title has no letter or digit to make a slug from.
pub(crate) fn no_slug(title: &str) -> UserMessage {
    UserMessage::new(
        format!("Khalid cannot make an address from the title \"{title}\"."),
        "A post's address is made from the letters and digits in its title.",
        "Give a title with at least one letter or digit.",
    )
}

/// The message when `--template` asks for a kind of post the title does
/// not read as.
pub(crate) fn not_kind(title: &str, template: Template) -> UserMessage {
    let name = template.name();
    UserMessage::new(
        format!("The title \"{title}\" does not read as a {name} post."),
        format!(
            "The linter checks the shape of a {name} post only when its title says what it is."
        ),
        format!(
            "Write a title like \"{}\", or leave out --template.",
            template.example()
        ),
    )
}

/// The message when the post's file is already there.
pub(crate) fn exists(relative: &str) -> UserMessage {
    UserMessage::new(
        format!("{relative} already exists."),
        "Khalid never overwrites a post, so nothing has changed.",
        "Choose a different title, or open the existing file to edit it.",
    )
}

/// The message when the post's file cannot be written.
pub(crate) fn write(relative: &str, error: &io::Error) -> UserMessage {
    UserMessage::new(
        format!("Khalid could not write {relative}: {error}."),
        "The post exists only once its file is written.",
        "Check that you may write to the site folder, then run the command again.",
    )
}

/// The message when `git` cannot be started for `--branch`.
pub(crate) fn no_git() -> UserMessage {
    UserMessage::new(
        "Khalid could not run git on this computer.",
        "With --branch, each draft is a git branch, and Khalid uses git to make it.",
        "Install git from https://git-scm.com, or leave out --branch.",
    )
}

/// The message when `--branch` is used outside a git repository.
pub(crate) fn not_a_repository(site: &Path) -> UserMessage {
    UserMessage::new(
        format!("{} is not in a git repository.", site.display()),
        "With --branch, each draft is a git branch, and a branch needs a repository.",
        format!(
            "Run `git init` in {} and commit the site, or leave out --branch.",
            site.display()
        ),
    )
}

/// The message when the draft's branch is there already.
pub(crate) fn branch_exists(branch: &str) -> UserMessage {
    UserMessage::new(
        format!("The branch {branch} already exists."),
        "Each draft has its own branch, named after the post, and Khalid never reuses one.",
        format!(
            "Run `git switch {branch}` to keep writing that draft, or choose a different title."
        ),
    )
}

/// The message when git refused a step of `--branch`, with the first line
/// it gave as `said`.
pub(crate) fn git_refused(site: &Path, said: &str) -> UserMessage {
    let said = said.trim().trim_end_matches('.');
    UserMessage::new(
        format!("Git could not make the draft's branch: {said}."),
        "With --branch, the new post is committed on its own branch.",
        format!(
            "Run `git -C {} status` to see why, sort it out, then run the command again.",
            site.display()
        ),
    )
}
