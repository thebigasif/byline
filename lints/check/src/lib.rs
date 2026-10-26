//! Proof that the lint table fires (RULES.md BL-LNT-3).
//!
//! This crate owns one deliberate violation per entry in `lints/lints.toml`, in
//! the `violations` module. The module compiles only when
//! `scripts/check-lints.sh` sets the `khalid_lint_check` cfg, so every other
//! build sees an empty crate. It must not know about the product: no crate
//! depends on it and it depends on nothing.

/// One deliberate violation per lint, each marked with the lint it must trip.
#[cfg(khalid_lint_check)]
pub mod violations;
