//! Git mode's push hook on `khalid serve`: a git host that was pushed to
//! asks the blog to pull, and the server's watch shows what came in (plan
//! section 4.6, ADR "git mode: a branch per draft").

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use axum::Router;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::post;
use hmac::{Hmac, KeyInit, Mac};
use khalid_core::Secret;
use sha2::Sha256;
use subtle::ConstantTimeEq;

use crate::pull;

/// Where the hook answers, beside the APIs.
pub(crate) const HOOK_PATH: &str = "/api/hooks/git";

/// GitHub's header: `sha256=` and the HMAC-SHA256 of the body, in hex.
const GITHUB_SIGNATURE: &str = "x-hub-signature-256";

/// GitLab's header: the secret itself.
const GITLAB_TOKEN: &str = "x-gitlab-token";

/// What the hook needs: the site folder, the secret, and a lock so two
/// pulls never run at once.
#[derive(Clone)]
struct Hook {
    site: PathBuf,
    secret: Arc<Secret<String>>,
    pulling: Arc<Mutex<()>>,
}

/// The push hook for the site in `site`, or no route at all when the
/// environment holds no `secret`.
pub(crate) fn router(site: &Path, secret: Option<Secret<String>>) -> Router {
    let Some(secret) = secret else {
        return Router::new();
    };
    let hook = Hook {
        site: site.to_path_buf(),
        secret: Arc::new(secret),
        pulling: Arc::new(Mutex::new(())),
    };
    Router::new()
        .route(HOOK_PATH, post(pushed))
        .with_state(hook)
}

/// Answers a signed push with 202 and pulls in the background, since a host
/// waits about ten seconds for an answer. Anything unsigned is a 401.
async fn pushed(
    State(hook): State<Hook>,
    headers: HeaderMap,
    body: Bytes,
) -> (StatusCode, &'static str) {
    if !signed(&headers, &body, hook.secret.expose()) {
        return (
            StatusCode::UNAUTHORIZED,
            "The push was not signed with KHALID_GIT_HOOK_SECRET. Set the same secret on the git host's webhook.\n",
        );
    }
    tokio::task::spawn_blocking(move || {
        let _one_at_a_time = hook.pulling.lock().unwrap_or_else(PoisonError::into_inner);
        match pull::pulled(&hook.site) {
            Ok(moved) => tracing::info!(files = moved.files, "the push hook pulled"),
            Err(message) => tracing::warn!(what = message.what(), "the push hook could not pull"),
        }
    });
    (StatusCode::ACCEPTED, "Pulling.\n")
}

/// Whether `headers` sign `body` with `secret`, the way GitHub or GitLab
/// does, compared in constant time.
fn signed(headers: &HeaderMap, body: &[u8], secret: &str) -> bool {
    let header = |name| headers.get(name).and_then(|value| value.to_str().ok());
    if let Some(signature) = header(GITHUB_SIGNATURE) {
        let Some(hex) = signature.strip_prefix("sha256=") else {
            return false;
        };
        let (Some(sent), Ok(mut mac)) = (
            bytes(hex),
            Hmac::<Sha256>::new_from_slice(secret.as_bytes()),
        ) else {
            return false;
        };
        mac.update(body);
        return mac.verify_slice(&sent).is_ok();
    }
    header(GITLAB_TOKEN).is_some_and(|token| bool::from(token.as_bytes().ct_eq(secret.as_bytes())))
}

/// The bytes `hex` spells, or `None` when it is not hex.
fn bytes(hex: &str) -> Option<Vec<u8>> {
    let digits = hex.as_bytes();
    if !digits.len().is_multiple_of(2) {
        return None;
    }
    digits
        .chunks(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn github(body: &[u8], secret: &str) -> HeaderMap {
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body);
        let hex = mac
            .finalize()
            .into_bytes()
            .iter()
            .fold(String::new(), |mut hex, b| {
                let _ = std::fmt::Write::write_fmt(&mut hex, format_args!("{b:02x}"));
                hex
            });
        let mut headers = HeaderMap::new();
        headers.insert(GITHUB_SIGNATURE, format!("sha256={hex}").parse().unwrap());
        headers
    }

    #[test]
    fn a_github_signature_counts_only_for_its_body_and_secret() {
        let secret = "made-for-this-test";
        assert!(signed(&github(b"{}", secret), b"{}", secret));
        assert!(!signed(&github(b"{}", secret), b"{\"a\":1}", secret));
        assert!(!signed(&github(b"{}", "another"), b"{}", secret));
        assert!(!signed(&HeaderMap::new(), b"{}", secret));
        let mut odd = HeaderMap::new();
        odd.insert(GITHUB_SIGNATURE, "sha256=zz".parse().unwrap());
        assert!(!signed(&odd, b"{}", secret));
    }

    #[test]
    fn a_gitlab_token_counts_only_when_it_is_the_secret() {
        let secret = "made-for-this-test";
        let mut headers = HeaderMap::new();
        headers.insert(GITLAB_TOKEN, secret.parse().unwrap());
        assert!(signed(&headers, b"{}", secret));
        headers.insert(GITLAB_TOKEN, "made-for-this-tesT".parse().unwrap());
        assert!(!signed(&headers, b"{}", secret));
    }
}
