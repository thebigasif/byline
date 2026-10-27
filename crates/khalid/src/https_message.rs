//! What a person reads when `khalid serve --domain` cannot start HTTPS
//! (RULES.md BL-CLI-3). A certificate is the file that lets browsers show
//! a site as secure; Let's Encrypt gives them for free.

use std::io;
use std::net::SocketAddr;

use khalid_core::UserMessage;
use khalid_https::{Domain, HttpsError, LETS_ENCRYPT_TERMS};

/// The message when a yes to Let's Encrypt's terms is needed and no one is
/// at a terminal to give it.
pub(crate) fn terms_needed(domain: &Domain) -> UserMessage {
    UserMessage::new(
        format!(
            "Khalid needs your yes to Let's Encrypt's subscriber agreement to get a certificate for {domain}."
        ),
        "Let's Encrypt gives free certificates only to site owners who agree to it, and no one is at a terminal to ask.",
        format!("Read it at {LETS_ENCRYPT_TERMS}, then run the command again with --accept-terms."),
    )
}

/// The message when the owner did not agree to Let's Encrypt's terms.
pub(crate) fn terms_declined() -> UserMessage {
    UserMessage::new(
        "Khalid did not get a certificate, because you did not agree to Let's Encrypt's subscriber agreement.",
        "Let's Encrypt gives free certificates only to site owners who agree to it.",
        "Run the command again and type yes, or serve plain HTTP behind your own HTTPS proxy with --listen.",
    )
}

/// The message when `--domain` is given for a static site.
pub(crate) fn static_site() -> UserMessage {
    UserMessage::new(
        "HTTPS with Khalid's own certificate is off for a static site.",
        "A static site is plain files on a host, and the host gives it HTTPS.",
        "Turn HTTPS on at your host, or set mode = \"server\" in site.toml and run `khalid serve --domain` again.",
    )
}

/// The message when port 443 or 80 cannot be opened.
pub(crate) fn listen(address: SocketAddr, error: &io::Error) -> UserMessage {
    let why = match error.kind() {
        io::ErrorKind::AddrInUse => {
            "Another program, such as another web server, is already using that port."
        }
        io::ErrorKind::PermissionDenied => {
            "Ports below 1024 need extra permission on this computer."
        }
        _ => "The computer refused to open that port for Khalid.",
    };
    let fix = match error.kind() {
        io::ErrorKind::PermissionDenied => {
            "Run Khalid as a service with the systemd unit, or give it the permission once: sudo setcap cap_net_bind_service=+ep $(command -v khalid)."
        }
        _ => "Stop the other program, or serve plain HTTP behind it with --listen 127.0.0.1:8080.",
    };
    UserMessage::new(
        format!("Khalid could not listen on {address}: {error}."),
        why,
        fix,
    )
}

/// The message when HTTPS cannot start for `domain`.
pub(crate) fn start(domain: &Domain, error: &HttpsError) -> UserMessage {
    match error {
        HttpsError::TermsNotAgreed => terms_needed(domain),
        HttpsError::Refused { detail } => UserMessage::new(
            format!("Let's Encrypt could not confirm that {domain} leads to this server."),
            detail.clone().unwrap_or_else(|| {
                "Its check did not reach Khalid on port 443 or port 80.".to_owned()
            }),
            format!(
                "Point the A and AAAA records of {domain} to this server, open ports 80 and 443 in its firewall, and run the command again."
            ),
        ),
        HttpsError::Acme { .. } | HttpsError::NoCheck => UserMessage::new(
            "Khalid could not get a certificate from Let's Encrypt.",
            "This server cannot reach Let's Encrypt, or Let's Encrypt refused the request.",
            "Check that this server can reach the internet, wait a few minutes, and run the command again.",
        ),
        HttpsError::Folder { path, .. } => UserMessage::new(
            format!("Khalid could not read or write {}.", path.display()),
            "It keeps the site's certificate and its Let's Encrypt account there.",
            "Make sure the user running Khalid owns the site folder, then run the command again.",
        ),
        _ => UserMessage::new(
            "The saved certificate or Let's Encrypt account cannot be read.",
            "A file in data/https in the site folder is damaged or was changed by hand.",
            "Delete the folder data/https in the site folder. Khalid gets a new certificate at the next start.",
        ),
    }
}
