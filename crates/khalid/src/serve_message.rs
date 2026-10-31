//! What a person reads when `khalid serve` cannot start or keep serving
//! (RULES.md BL-CLI-3).

use std::error::Error;
use std::net::SocketAddr;
use std::{fmt, io};

use khalid_core::UserMessage;

/// Why the site could not be rendered after an edit, as a person reads it.
#[derive(Debug)]
pub(crate) struct Unrendered(pub(crate) UserMessage);

impl fmt::Display for Unrendered {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Error for Unrendered {}

/// The message when the server cannot listen on `address`.
pub(crate) fn listen(address: SocketAddr, error: &io::Error) -> UserMessage {
    let why = match error.kind() {
        io::ErrorKind::AddrInUse => "Another program is already using that port.",
        io::ErrorKind::AddrNotAvailable => "That address does not belong to this computer.",
        io::ErrorKind::PermissionDenied => "Ports below 1024 need extra permission.",
        _ => "The computer refused to open that address for Khalid.",
    };
    UserMessage::new(
        format!("Khalid could not listen on {address}: {error}."),
        why,
        "Choose another address with --listen, such as --listen 127.0.0.1:8081.",
    )
}

/// The message when the server stopped answering by itself.
pub(crate) fn stopped() -> UserMessage {
    UserMessage::new(
        "The blog stopped answering visitors.",
        "The computer closed the address Khalid was listening on.",
        "Run `khalid serve` again, and report it with the output of `khalid --version` if it happens again.",
    )
}

/// The message when the server cannot start for a reason inside Khalid or
/// the computer, which leaves nothing the person typed to blame.
pub(crate) fn internal() -> UserMessage {
    UserMessage::new(
        "Khalid could not start the server.",
        "The cause is inside Khalid or the computer, not in your site.",
        "Run the command again, and report it with the output of `khalid --version` if it fails again.",
    )
}
