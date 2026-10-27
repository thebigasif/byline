//! `WordPress`: the WXR export file that Tools, Export writes.

mod export;
mod item;
mod item_file;
mod names;
mod php;
mod plan;

mod post_slug;
mod redirect_map;
mod redirection;
mod report;
mod seo_meta;
mod seo_plugins;
mod seo_text;
mod site_files;
mod wxr;
mod wxr_error;

pub use export::{Author, Export, Site, Term};
pub use item::{Item, ItemKind, ItemStatus, Meta};
pub use plan::{
    FailedItem, Held, ImportedItem, ItemError, LeftItem, LeftReason, Place, Plan, PlannedFile,
    REDIRECTS_CSV, plan,
};
pub use post_slug::SlugOrigin;
pub use redirect_map::{MapEntry, MapSource, MapTarget, Matching};
pub use redirection::{
    MoveStatus, Redirection, RedirectionError, RedirectionRule, RuleSource, RuleTarget, SkipReason,
    SkippedRule, read_redirection,
};
pub use report::Report;
pub use seo_meta::{SeoMeta, SeoPlugin, UnreadField};
pub use wxr::read_wxr;
pub use wxr_error::WxrError;
