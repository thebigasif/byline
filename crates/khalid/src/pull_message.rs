//! What a person reads when `khalid pull` cannot finish (RULES.md
//! BL-ERR-4).

use std::path::Path;

use khalid_core::UserMessage;

/// The message when the folder holds no `site.toml`.
pub(crate) fn not_a_site(site: &Path) -> UserMessage {
    UserMessage::new(
        format!(
            "{} has no site.toml, so it is not a site folder.",
            site.display()
        ),
        "Khalid pulls the repository that holds one site, and site.toml marks its folder.",
        "Run `khalid --site <folder> pull` with the folder that holds site.toml.",
    )
}

/// The message when `git` cannot be started.
pub(crate) fn no_git() -> UserMessage {
    UserMessage::new(
        "Khalid could not run git on this computer.",
        "Git mode keeps the site in a git repository, and Khalid uses git to fetch it.",
        "Install git from https://git-scm.com, then run `khalid pull` again.",
    )
}

/// The message when the site is not in a git repository.
pub(crate) fn not_a_repository(site: &Path) -> UserMessage {
    UserMessage::new(
        format!(
            "{} is not in a git repository, so there is nothing to pull.",
            site.display()
        ),
        "Git mode keeps the site in a repository, and `khalid pull` brings in what was pushed to it.",
        "Clone the site's repository with `git clone`, and run `khalid pull` in that folder.",
    )
}

/// The message when git refused or failed to bring in the changes, with
/// the first line it gave as `said`.
pub(crate) fn refused(site: &Path, said: &str) -> UserMessage {
    let said = said.trim().trim_end_matches('.');
    UserMessage::new(
        format!("Git could not bring in the changes: {said}."),
        "Khalid only moves the site forward to what the repository holds, so it never merges or overwrites changes made here.",
        format!(
            "Run `git -C {} status` to see why, sort it out, then run `khalid pull` again.",
            site.display()
        ),
    )
}
