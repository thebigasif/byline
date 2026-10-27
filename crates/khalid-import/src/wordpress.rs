//! `WordPress`: the WXR export file that Tools, Export writes.

mod export;
mod item;
mod php;
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "the post files of task 6.1 part 4b use it")
)]
mod post_slug;
mod redirection;
mod seo_meta;
mod seo_plugins;
mod seo_text;
mod wxr;
mod wxr_error;

pub use export::{Author, Export, Site, Term};
pub use item::{Item, ItemKind, ItemStatus, Meta};
pub use redirection::{
    MoveStatus, Redirection, RedirectionError, RedirectionRule, RuleSource, RuleTarget, SkipReason,
    SkippedRule, read_redirection,
};
pub use seo_meta::{SeoMeta, SeoPlugin, UnreadField};
pub use wxr::read_wxr;
pub use wxr_error::WxrError;
