//! The answers people send through a post's form block (KH-GRO-1).
//!
//! A `:::form` block posts to `<mount>forms/<id>`. The route that answers
//! it on `khalid serve` drops what robots send through the spam trap, reads
//! the answers into a [`Submission`] named by a [`FormId`] and keeps it in
//! a [`FormStore`]. Every answer is a plain page, so the form works with
//! scripts off.
//!
//! This crate must not know about SQL, templates or content files.
//! `khalid-db` implements the store. Sending each answer on by email or
//! webhook comes later (G40).

mod answer_page;
mod form_id;
mod form_route;
mod form_store;
mod submission;

pub use form_id::{FormId, FormIdError};
pub use form_route::{FormRoute, SiteForms, form_router};
pub use form_store::{FormStore, FormStoreError};
pub use submission::Submission;
