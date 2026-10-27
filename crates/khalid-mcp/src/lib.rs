//! The Model Context Protocol server behind `khalid mcp` (KH-API-5).
//!
//! This crate speaks the protocol: one JSON-RPC 2.0 message per line on
//! stdin and one answer per line on stdout, the lifecycle, `ping`, and the
//! tool methods. What each tool does comes from a [`Tools`] the caller
//! gives it, so the protocol knows nothing about posts, keys or files.

mod server;
mod tool;

pub use server::{MAX_MESSAGE, ServeError, Server};
pub use tool::{Tool, Tools};
