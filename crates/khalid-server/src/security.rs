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

/// The policy after the script, style and connection sources, up to the
/// frame sources. Styles and connections come from the site itself, except
/// on a page with head code.
///
/// Fonts and media come from the site itself. Images may also come
/// from any `https` address or a `data:` address, because writers and
/// bookmark cards show pictures hosted elsewhere, and an image cannot run
/// code. Forms post only to the site, the page may be framed only by the
/// site, and `base-uri 'none'` stops an injected `base` tag from moving
/// every relative link.
const REST: &str = "; img-src 'self' https: data:; font-src 'self'; \
     media-src 'self'; manifest-src 'self'; object-src 'none'; \
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

    /// Sets every security header on `headers`, with a new nonce, for a
    /// response made outside [`router`](crate::router), such as a page of
    /// search results.
    pub fn apply(&self, headers: &mut HeaderMap) {
        for (name, value) in &FIXED {
            headers.insert(name.clone(), value.clone());
        }
        if let Some(policy) = self.policy() {
            headers.insert(header::CONTENT_SECURITY_POLICY, policy);
        }
    }

    /// Sets every security header on `headers` for a page whose head code
    /// was given `nonce`: its scripts and styles may run under that nonce,
    /// and scripts may send data to any `https` address, as analytics
    /// tags do (ADR "head code gets the nonce on each request").
    pub(crate) fn apply_with_head_code(&self, headers: &mut HeaderMap, nonce: &str) {
        for (name, value) in &FIXED {
            headers.insert(name.clone(), value.clone());
        }
        if let Some(policy) = self.policy_with(Some(nonce), Sources::HeadCode) {
            headers.insert(header::CONTENT_SECURITY_POLICY, policy);
        }
    }

    /// The policy with a nonce no other response has had. If the system
    /// cannot give random bytes, the script sources become `'none'`, which
    /// is stricter, never looser.
    fn policy(&self) -> Option<HeaderValue> {
        self.policy_with(fresh_nonce().as_deref(), Sources::Site)
    }

    /// The policy with `nonce`, or with no script at all when there is none.
    fn policy_with(&self, nonce: Option<&str>, sources: Sources) -> Option<HeaderValue> {
        let length = HEAD.len() + SCRIPT_SOURCES * 2 + REST.len() + self.frames.len();
        let mut policy = String::with_capacity(length);
        policy.push_str(HEAD);
        match nonce {
            Some(nonce) => {
                policy.push_str("'nonce-");
                policy.push_str(nonce);
                policy.push_str("' 'strict-dynamic'");
            }
            None => policy.push_str("'none'"),
        }
        match (sources, nonce) {
            (Sources::HeadCode, Some(nonce)) => {
                policy.push_str("; style-src 'self' 'nonce-");
                policy.push_str(nonce);
                policy.push_str("'; connect-src 'self' https:");
            }
            _ => policy.push_str("; style-src 'self'; connect-src 'self'"),
        }
        policy.push_str(REST);
        policy.push_str(&self.frames);
        HeaderValue::try_from(policy).ok()
    }
}

/// Which pages a policy is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Sources {
    /// Any page: no inline style, connections to the site only.
    Site,
    /// A page with a post's head code, whose styles carry the nonce too and
    /// whose scripts may report to analytics services.
    HeadCode,
}

/// A nonce no other response has had, as 32 hex digits, or `None` when the
/// system cannot give random bytes.
pub(crate) fn fresh_nonce() -> Option<String> {
    let bytes = nonce()?;
    let digits = bytes.iter().flat_map(|byte| [byte >> 4, byte & 0x0f]);
    Some(
        digits
            .filter_map(|digit| char::from_digit(u32::from(digit), 16))
            .collect(),
    )
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
