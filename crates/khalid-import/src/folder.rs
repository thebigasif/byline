//! A folder of markdown files, as Hugo, Jekyll, Astro and Eleventy keep
//! a blog: one file per post with front matter on top.

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the folder plan converts bodies, in the next part"
    )
)]
mod body;
mod export;
mod read;

pub use body::BodyError;
pub use export::{Export, Front, SourceFile, UnreadFile};
pub use read::{FileError, FolderError, read_folder};
