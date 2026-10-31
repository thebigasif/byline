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
mod audit_error;
mod body_checks;
mod check;
mod crawl;
mod folder;
mod head_link_checks;
mod history;
mod issue;
mod link;
mod link_checks;
mod link_report;
mod outgoing;
mod outline;
mod page;
mod page_checks;
mod report;
mod robots;
mod server;
mod sitemap;
mod sitemap_checks;
mod thresholds;
mod visit;
mod writing_checks;

pub use answer::{Answer, Headers, Source, Unreached};
pub use audit::audit;
pub use audit_error::AuditError;
pub use check::{Category, Check, Severity};
pub use crawl::{Crawl, Limit, crawl};
pub use folder::Folder;
pub use history::{Run, history};
pub use issue::Issue;
pub use link::{Alternate, Image, Link};
pub use link_report::{LinkReport, PageLinks};
pub use outgoing::check_outside;
pub use outline::{Block, Paragraph};
pub use page::Page;
pub use report::Report;
pub use robots::Robots;
pub use server::Server;
pub use sitemap::{SiteFiles, Sitemap};
pub use thresholds::Thresholds;
pub use visit::{Outcome, Visit};
