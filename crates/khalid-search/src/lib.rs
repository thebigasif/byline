//! Site search: how text becomes search terms, per language.
//!
//! A post's title, headings and body are split into words, and each word
//! becomes a [`Term`]: lower case, and cut to its stem in English and
//! German, so "linking" finds "links" and "Häuser" finds "Haus". The query
//! a reader types goes through the same [`Analyzer`], so both sides meet on
//! the same terms. The index itself lives in the database, built at publish
//! time (plan section 7, KH-WEB-2).
//!
//! This crate must not know about the database, HTTP, templates or files.
//! It works on the text it is given.

mod analyzer;
mod language;
mod term;

pub use analyzer::{Analyzer, Word};
pub use language::Language;
pub use term::Term;
