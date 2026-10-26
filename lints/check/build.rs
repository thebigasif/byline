//! Declares the `khalid_lint_check` cfg.
//!
//! `scripts/check-lints.sh` sets it to compile the deliberate violations. Every
//! other build leaves it unset, so the workspace stays clean.

fn main() {
    println!("cargo::rustc-check-cfg=cfg(khalid_lint_check)");
}
