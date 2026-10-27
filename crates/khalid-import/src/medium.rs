//! Medium: the archive that Settings, Security and apps, Download your
//! information gives, with one HTML file per post in its `posts/` folder.

mod export;
mod read;

pub use export::{Author, Export, Post, PostStatus, UnreadFile};
pub use read::{ExportError, PostFileError, read_export};
