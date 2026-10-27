//! Bringing a blog from another system into Khalid (plan section 5.7, PRD KH-API-7).
//!
//! This crate owns reading other systems' exports and turning them into
//! Khalid content files, with kept slugs, a redirect map and a report of what
//! came in and what did not. One module per source; `WordPress` comes first.
//! It must not know about rendering, HTTP or the database. It reads text it
//! is given and never fetches anything itself.

#[cfg_attr(
    not(test),
    expect(dead_code, reason = "the post files of task 6.1 part 4b use it")
)]
mod front_matter;
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "the post files of task 6.1 part 4c use it")
)]
mod markdown;
pub mod wordpress;
