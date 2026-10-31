//! The Model Context Protocol server behind `khalid mcp` (KH-API-5).
//!
//! This crate speaks the protocol: one JSON-RPC 2.0 message per line on
//! stdin and one answer per line on stdout, the lifecycle, `ping`, and the
//! tool methods. What each tool does comes from a [`Tools`] the caller
//! gives it, so the protocol knows nothing about posts, keys or files.

mod api_tools;
mod call;
mod operations;
mod server;
mod session_keys;
mod tool;

pub use api_tools::ApiTools;
pub use server::{MAX_MESSAGE, ServeError, Server};
pub use session_keys::SessionKeys;
pub use tool::{BoxFuture, Tool, ToolResult, Tools};
