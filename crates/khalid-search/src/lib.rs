//! Site search: how text becomes search terms, per language.
//!
//! A post's title, headings and body are split into words, and each word
//! becomes a [`Term`]: lower case, and cut to its stem in English and
//! German, so "linking" finds "links" and "Häuser" finds "Haus". The query
//! a reader types goes through the same [`Analyzer`], so both sides meet on
//! the same terms. [`Analyzer::index`] counts each term per field, which is
//! what the index in the database stores, built at publish time (plan
//! section 7, KH-WEB-2). A [`Query`] looks its terms up there, and [`rank`]
//! orders the pages found, the title counting most, then the headings, then
//! the body.
//!
//! This crate must not know about the database, HTTP, templates or files.
//! It works on the text it is given.

mod analyzer;
mod field_counts;
mod indexed_text;
mod language;
mod query;
mod rank;
mod term;

pub use analyzer::{Analyzer, Word};
pub use field_counts::FieldCounts;
pub use indexed_text::IndexedText;
pub use language::Language;
pub use query::{Completion, Query, QueryTerm};
pub use rank::{Corpus, Posting, rank};
pub use term::Term;
