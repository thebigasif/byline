//! Mounting the blog under a folder of a site built with something else.
//!
//! When the blog is at `example.com/blog` and `example.com` runs on another
//! stack, that stack passes `/blog` on to Khalid (plan 4.8, KH-INS-3). This
//! crate owns the exact rule per stack (ADR 0017). It must not know about
//! content, templates, HTTP clients or files.

mod mount_error;
mod mount_path;
mod proxy_rule;
mod stack;

pub use mount_error::MountError;
pub use mount_path::MountPath;
pub use proxy_rule::proxy_rule;
pub use stack::Stack;
