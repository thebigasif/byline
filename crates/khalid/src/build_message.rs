//! What a person reads when `khalid build` cannot render the site (RULES.md BL-CLI-3).

use std::io;
use std::path::Path;

use khalid_content::BlockError;
use khalid_core::UserMessage;
use khalid_render::RenderError;

/// The message when the output folder is inside `content/`.
pub(crate) fn into_content(out: &Path) -> UserMessage {
    UserMessage::new(
        format!("Khalid will not build the site into {}.", out.display()),
        "The content folder holds only what you write, never what Khalid makes from it.",
        "Choose an output folder outside content with --out, or drop --out to use public.",
    )
}

/// The message when a post's body has a block Khalid cannot draw.
pub(crate) fn block(path: &Path, error: &BlockError) -> UserMessage {
    UserMessage::new(
        format!("Khalid could not render {}: {error}.", path.display()),
        "A page is built only when every block in it can be drawn.",
        "Correct the block in that file, then build again.",
    )
}

/// The message when a post, page or author has no slug and nothing to make one from.
pub(crate) fn no_slug(path: &Path) -> UserMessage {
    UserMessage::new(
        format!(
            "{} has no slug, and no title or name to make one from.",
            path.display()
        ),
        "Every page needs an address, and its address ends in its slug.",
        "Add a slug to the front matter of that file.",
    )
}

/// The message when a page's address is not a valid web address.
pub(crate) fn address(address: &str) -> UserMessage {
    UserMessage::new(
        format!("Khalid could not make a valid web address from {address}."),
        "Every page needs its full address for search engines and shared links.",
        "Check address and mount in site.toml, then build again.",
    )
}

/// The message when a template cannot render a page.
pub(crate) fn render(error: &RenderError) -> UserMessage {
    UserMessage::new(
        format!("Khalid could not render a page: {error}."),
        "A site is built only when every page renders, so nothing broken goes live.",
        "Check the templates in theme/templates, or remove the one named, then build again.",
    )
}

/// The message when a page cannot be written.
pub(crate) fn write(file: &Path, error: &io::Error) -> UserMessage {
    UserMessage::new(
        format!("Khalid could not write {}: {error}.", file.display()),
        "The site is complete only when every page is written.",
        "Check that you may write to the output folder, then build again.",
    )
}
