//! The security headers every response carries, and the Content Security
//! Policy with its nonce made fresh for each request.

use axum::http::{HeaderMap, HeaderName, HeaderValue, header};

use crate::ServerError;

/// How many random bytes a nonce holds: 128 bits, as the CSP
/// specification recommends at the least.
const NONCE_BYTES: usize = 16;

/// The policy up to the script sources. Nothing loads from anywhere unless
/// a directive allows it.
const HEAD: &str = "default-src 'none'; script-src ";

/// The policy after the script sources, up to the frame sources.
///
/// Styles, fonts and media come from the site itself. Images may also come
/// from any `https` address or a `data:` address, because writers and
/// bookmark cards show pictures hosted elsewhere, and an image cannot run
/// code. Forms post only to the site, the page may be framed only by the
/// site, and `base-uri 'none'` stops an injected `base` tag from moving
/// every relative link.
const REST: &str = "; style-src 'self'; img-src 'self' https: data:; font-src 'self'; \
     media-src 'self'; connect-src 'self'; manifest-src 'self'; object-src 'none'; \
     base-uri 'none'; form-action 'self'; frame-ancestors 'self'; frame-src ";

/// Room for the script sources with their nonce.
const SCRIPT_SOURCES: usize = 64;

/// The headers that are the same on every response.
const FIXED: [(HeaderName, HeaderValue); 5] = [
    (
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    ),
    (
        header::X_FRAME_OPTIONS,
        HeaderValue::from_static("SAMEORIGIN"),
    ),
    (
        header::REFERRER_POLICY,
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    ),
    (
        HeaderName::from_static("permissions-policy"),
        HeaderValue::from_static(
            "camera=(), microphone=(), geolocation=(), payment=(), usb=(), browsing-topics=()",
        ),
    ),
    (
        HeaderName::from_static("cross-origin-opener-policy"),
        HeaderValue::from_static("same-origin"),
    ),
];

/// The security headers for every response (RULES.md BL-SEC-2), with the
/// players an embed may show in a frame.
///
/// The default allows no frames at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityHeaders {
    /// The `frame-src` sources, the end of the policy.
    frames: String,
}

impl SecurityHeaders {
    /// The headers for a site whose embeds load players from
    /// `frame_origins`, such as `https://player.vimeo.com`.
    ///
    /// # Errors
    ///
    /// Returns [`ServerError::FrameOrigin`] for an origin that is not
    /// `https://` and a host, with an optional port, because anything else
    /// could widen or break the policy.
    pub fn new(frame_origins: &[&str]) -> Result<Self, ServerError> {
        if let Some(bad) = frame_origins.iter().find(|origin| !is_origin(origin)) {
            return Err(ServerError::FrameOrigin {
                origin: (*bad).to_owned(),
            });
        }
        Ok(match frame_origins {
            [] => Self::default(),
            origins => Self {
                frames: origins.join(" "),
            },
        })
    }

    /// Sets every security header on `headers`, with a new nonce.
    pub(crate) fn apply(&self, headers: &mut HeaderMap) {
        for (name, value) in &FIXED {
            headers.insert(name.clone(), value.clone());
        }
        if let Some(policy) = self.policy() {
            headers.insert(header::CONTENT_SECURITY_POLICY, policy);
        }
    }

    /// The policy with a nonce no other response has had. If the system
    /// cannot give random bytes, the script sources become `'none'`, which
    /// is stricter, never looser.
    fn policy(&self) -> Option<HeaderValue> {
        let length = HEAD.len() + SCRIPT_SOURCES + REST.len() + self.frames.len();
        let mut policy = String::with_capacity(length);
        policy.push_str(HEAD);
        match nonce() {
            Some(bytes) => {
                policy.push_str("'nonce-");
                let digits = bytes.iter().flat_map(|byte| [byte >> 4, byte & 0x0f]);
                policy.extend(digits.filter_map(|digit| char::from_digit(u32::from(digit), 16)));
                policy.push_str("' 'strict-dynamic'");
            }
            None => policy.push_str("'none'"),
        }
        policy.push_str(REST);
        policy.push_str(&self.frames);
        HeaderValue::try_from(policy).ok()
    }
}

impl Default for SecurityHeaders {
    fn default() -> Self {
        Self {
            frames: "'none'".to_owned(),
        }
    }
}

/// 128 random bits from the operating system.
fn nonce() -> Option<[u8; NONCE_BYTES]> {
    let mut bytes = [0_u8; NONCE_BYTES];
    getrandom::fill(&mut bytes).ok()?;
    Some(bytes)
}

/// Whether `origin` is `https://` and a host of letters, digits, `-` and
/// `.`, with an optional `:port`, and nothing else.
fn is_origin(origin: &str) -> bool {
    let Some(rest) = origin.strip_prefix("https://") else {
        return false;
    };
    let (host, port) = rest.split_once(':').unwrap_or((rest, "443"));
    let host_ok = !host.is_empty()
        && host
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'.');
    let port_ok = !port.is_empty() && port.bytes().all(|b| b.is_ascii_digit());
    host_ok && port_ok
}
