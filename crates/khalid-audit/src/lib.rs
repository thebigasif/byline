//! The site audit (`docs/plan.md` section 5.10, KH-LNT-5).
//!
//! This crate crawls a blog the way Ahrefs and Semrush do: from the home
//! page, following links, staying inside the blog and checking each link
//! that leaves it. It reads every page once into what the checks need.
//!
//! It must not know where the answers come from: a built folder, a running
//! server or a test table all implement [`Source`]. It must not know about
//! content files, rendering or storage either; it sees the site only as a
//! crawler does.

mod answer;
mod audit;
mod body_checks;
mod check;
mod crawl;
mod issue;
mod link;
mod page;
mod page_checks;
mod robots;
mod thresholds;
mod visit;

pub use answer::{Answer, Source, Unreached};
pub use audit::audit;
pub use check::{Category, Check, Severity};
pub use crawl::{Crawl, Limit, crawl};
pub use issue::Issue;
pub use link::{Alternate, Image, Link};
pub use page::Page;
pub use robots::Robots;
pub use thresholds::Thresholds;
pub use visit::{Outcome, Visit};
