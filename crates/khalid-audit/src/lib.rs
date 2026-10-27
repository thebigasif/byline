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
mod link;
mod robots;

pub use answer::{Answer, Source, Unreached};
pub use link::{Alternate, Image, Link};
pub use robots::Robots;
