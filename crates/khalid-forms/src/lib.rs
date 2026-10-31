//! The answers people send through a post's form block (KH-GRO-1).
//!
//! A `:::form` block posts to `<mount>forms/<id>`. The route that answers
//! it on `khalid serve` drops what robots send through the spam trap, reads
//! the answers into a [`Submission`] named by a [`FormId`] and keeps it in
//! a [`FormStore`]. Every answer is a plain page, so the form works with
//! scripts off. Each kept answer goes on to a [`FormDelivery`].
//!
//! This crate must not know about SQL, templates, content files, mail or
//! webhooks. `khalid-db` implements the store, and the binary the
//! delivery by email and webhook.

mod answer_page;
mod form_delivery;
mod form_id;
mod form_route;
mod form_store;
mod submission;

pub use form_delivery::FormDelivery;
pub use form_id::{FormId, FormIdError};
pub use form_route::{FormRoute, SiteForms, form_router};
pub use form_store::{FormStore, FormStoreError};
pub use submission::Submission;
