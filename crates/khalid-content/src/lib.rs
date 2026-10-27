//! The files that are Khalid's source of truth (RULES.md BL-FS-1).
//!
//! This crate owns reading content: front matter, markdown and the content
//! tree of language folders, collections and posts. It must not know about
//! templates, HTTP or the database; it hands typed content to the crates that
//! render, serve and index it, and never writes derived data under
//! `content/` (BL-FS-2).

mod block;
mod body_check;
mod content_error;
mod content_hash;
mod directive;
mod embed;
mod form;
mod front_matter;
mod maths;
mod post;
mod render;
mod schemas;
mod site_content;
mod structured;
mod tree;
mod wikilink;

pub use block::{
    Block, BlockError, BlockKind, CalloutKind, GalleryImage, Visibility, parse_blocks, print_blocks,
};
pub use body_check::check_bodies;
pub use content_error::ContentError;
pub use content_hash::ContentHash;
pub use embed::{EMBED_PROVIDERS, EmbedProvider};
pub use form::{FieldKind, FormField};
pub use post::{MAX_RELATED, Post, PostFront, parse_post};
pub use render::{Heading, Image, Link, Rendered, render, render_linked};
pub use schemas::{
    AuthorFront, CaseStudyFront, ChangelogFront, GlossaryFront, IndexFront, PageFront, SnippetFront,
};
pub use site_content::{Document, SiteContent, read_content};
pub use structured::{
    BusinessBlock, HowToBlock, ProductBlock, Question, REVIEW_TYPES, RecipeBlock, ReviewBlock,
    Step, StructuredBlock, VideoBlock,
};
pub use tree::{Collection, ContentTree, Entry, Language};
pub use wikilink::{WikiTarget, Wikilinks};
