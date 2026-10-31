//! What a person reads when the sites folder cannot be used (RULES.md
//! BL-ERR-4).

use std::path::Path;

use khalid_core::UserMessage;

/// Why each sites.toml problem matters: Khalid serves from it.
const SERVED_FROM: &str = "Khalid reads this file to know which sites to serve and who adds them.";

pub(crate) fn unreadable(path: &Path) -> UserMessage {
    UserMessage::new(
        format!("{} could not be read.", path.display()),
        SERVED_FROM,
        "Check that the file exists and that Khalid may read it.",
    )
}

pub(crate) fn invalid(path: &Path, said: &str) -> UserMessage {
    UserMessage::new(
        format!("{} is not a list of sites: {said}.", path.display()),
        SERVED_FROM,
        "Give it super_admins = [...] and one [[site]] table with a domain per site.",
    )
}

pub(crate) fn bad_domain(path: &Path, given: &str) -> UserMessage {
    UserMessage::new(
        format!("{given} in {} is not a domain.", path.display()),
        "Each site answers on its domain, and its folder is named after it.",
        "Write the domain with letters, digits, dots and dashes only, such as news.example.com.",
    )
}

pub(crate) fn twice(path: &Path, domain: &str) -> UserMessage {
    UserMessage::new(
        format!("{domain} is listed twice in {}.", path.display()),
        "Each domain leads to one site, so readers always reach the same one.",
        "Remove one of the two [[site]] tables.",
    )
}

pub(crate) fn no_site(path: &Path) -> UserMessage {
    UserMessage::new(
        format!("{} lists no site.", path.display()),
        "The first site is the home site, where the super-admin panel lives.",
        "Add a site with khalid sites add followed by its domain.",
    )
}

pub(crate) fn not_a_sites_folder(root: &Path) -> UserMessage {
    UserMessage::new(
        format!("{} serves one site, not many.", root.display()),
        "A folder serves many sites once it has a sites.toml listing them.",
        "Run khalid sites add with your first domain in this folder to start one.",
    )
}

pub(crate) fn taken(domain: &str) -> UserMessage {
    UserMessage::new(
        format!("A site here already answers on {domain}."),
        "Each domain leads to one site, so readers always reach the same one.",
        "Choose another domain, or work on the site that has it.",
    )
}

pub(crate) fn not_saved(path: &Path) -> UserMessage {
    UserMessage::new(
        format!("{} could not be saved.", path.display()),
        SERVED_FROM,
        "Check there is free disk space and that Khalid may write there, then try again.",
    )
}

pub(crate) fn no_panel() -> UserMessage {
    UserMessage::new(
        "This site has no super-admin panel.",
        "Only the home site of a sites folder adds sites.",
        "Sign in on the home site, or run khalid sites add on the server.",
    )
}

pub(crate) fn not_made(folder: &Path) -> UserMessage {
    UserMessage::new(
        format!("The site in {} could not be made.", folder.display()),
        "Each site keeps its settings, posts and database in its own folder.",
        "Check there is free disk space and that Khalid may write there, then try again.",
    )
}

pub(crate) fn bad_email(path: &Path, given: &str) -> UserMessage {
    UserMessage::new(
        format!("{given} in {} is not an email address.", path.display()),
        "A super admin signs in on the home site with this address.",
        "Write an address such as owner@example.com in super_admins.",
    )
}
