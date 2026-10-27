//! The public site's templates.
//!
//! This crate owns the sandboxed `MiniJinja` environment, the default templates
//! built into the binary, a theme's replacements for them from
//! `theme/templates/`, and the words the templates show, one strings file per
//! language. It takes plain input records ([`RenderSite`], [`PageMeta`],
//! [`PageBody`]) and returns HTML.
//!
//! It must not know about markdown, the content tree, SQL or HTTP. The wiring
//! code turns content into these records; the server decides when to render
//! and caches the result.

mod asset;
mod author;
mod comments;
mod contents;
mod dates;
mod direction;
mod enlarge;
mod escape;
mod features;
mod font_fallback;
mod glossary;
mod heading_links;
mod home;
mod image;
mod listing;
mod page_body;
mod page_meta;
mod person;
mod post;
mod reading_time;
mod related;
mod render_error;
mod renderer;
mod script;
mod serde_display;
mod site;
mod strings;
mod stylesheet;
mod templates;
mod token_values;
mod tokens;

pub use asset::Asset;
pub use author::{AuthorEntry, RenderAuthor, RenderAuthors, RenderPage};
pub use comments::{CommentWriter, RenderComment, RenderComments, RenderThread};
pub use contents::{Contents, ContentsHeading, HeadingLevel};
pub use features::{Feature, Features};
pub use glossary::{
    GlossaryGroup, GlossaryTerm, Highlight, RenderGlossary, RenderSearch, SearchMode, SearchResult,
};
pub use home::{HomeSectionPosts, RenderHome, TopicGroup};
pub use image::{ImageCandidate, ImageSet, Loading, RenderImage};
pub use listing::{Pagination, PostCard, RenderListing};
pub use page_body::PageBody;
pub use page_meta::{Alternate, HeadHtml, PageKind, PageMeta, Robots, ShareImage};
pub use person::Person;
pub use post::{BodyHtml, Link, RenderPost};
pub use reading_time::ReadingTime;
pub use related::{PostTopics, related};
pub use render_error::RenderError;
pub use renderer::Renderer;
pub use script::{ScriptTag, reader_script};
pub use site::{Nav, NavLink, RenderSite};
pub use stylesheet::stylesheet;
pub use token_values::{Color, FontStack, Length, ParseTokenError};
pub use tokens::{Colors, ContrastFailure, Fonts, Shape, TokenName, Tokens};
