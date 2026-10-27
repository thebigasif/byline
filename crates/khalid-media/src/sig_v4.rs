//! AWS Signature Version 4, which S3-compatible buckets such as Cloudflare
//! R2 require on every request.

use std::fmt::Write as _;

use chrono::{DateTime, Utc};
use hmac::{Hmac, KeyInit, Mac};
use http::Uri;
use khalid_core::{S3Keys, Secret};
use sha2::{Digest, Sha256};

use crate::MediaError;

/// Signs requests with one key for one region and service, such as `s3`.
/// `Debug` hides both keys.
#[derive(Debug)]
pub struct SigV4 {
    access_key_id: Secret<String>,
    secret_access_key: Secret<String>,
    region: String,
    service: String,
}

/// The two headers a signed request sends.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Signed {
    /// `X-Amz-Date`, such as `20261027T080000Z`.
    pub amz_date: String,
    /// `Authorization`.
    pub authorization: String,
}

impl SigV4 {
    /// A signer with `keys`, for buckets in `region` of `service`.
    pub fn new(keys: S3Keys, region: &str, service: &str) -> Self {
        let S3Keys {
            access_key_id,
            secret_access_key,
            ..
        } = keys;
        Self {
            access_key_id,
            secret_access_key,
            region: region.to_owned(),
            service: service.to_owned(),
        }
    }

    /// Signs a `method` request for `uri` at the time `at`. The request's
    /// host, with its port, and `X-Amz-Date` are always signed, and so is
    /// each of `headers`, whose names must be lowercase. `payload` is the
    /// SHA-256 of the body in hex. The caller sends `headers` and both
    /// headers of the answer exactly as signed.
    ///
    /// # Errors
    ///
    /// Returns [`MediaError::Sign`] when the MAC refuses a key, which HMAC
    /// never does.
    pub fn sign(
        &self,
        method: &str,
        uri: &Uri,
        headers: &[(&str, &str)],
        payload: &str,
        at: DateTime<Utc>,
    ) -> Result<Signed, MediaError> {
        let amz_date = at.format("%Y%m%dT%H%M%SZ").to_string();
        let day = at.format("%Y%m%d").to_string();
        let host = uri.authority().map_or("", |authority| authority.as_str());
        let mut signed: Vec<(&str, &str)> = vec![("host", host), ("x-amz-date", &amz_date)];
        signed.extend_from_slice(headers);
        signed.sort_unstable();
        let names = signed
            .iter()
            .map(|(name, _)| *name)
            .collect::<Vec<_>>()
            .join(";");
        let mut canonical = format!("{method}\n{}\n", path_of(uri));
        canonical.push_str(&query_of(uri));
        canonical.push('\n');
        for (name, value) in &signed {
            let _ = writeln!(canonical, "{name}:{}", value.trim());
        }
        let _ = write!(canonical, "\n{names}\n{payload}");
        let scope = format!("{day}/{}/{}/aws4_request", self.region, self.service);
        let to_sign = format!(
            "AWS4-HMAC-SHA256\n{amz_date}\n{scope}\n{}",
            hex(&Sha256::digest(canonical.as_bytes()))
        );
        let secret = format!("AWS4{}", self.secret_access_key.expose());
        let mut key = mac(secret.as_bytes(), day.as_bytes())?;
        for part in [self.region.as_str(), self.service.as_str(), "aws4_request"] {
            key = mac(&key, part.as_bytes())?;
        }
        let signature = hex(&mac(&key, to_sign.as_bytes())?);
        let authorization = format!(
            "AWS4-HMAC-SHA256 Credential={}/{scope}, SignedHeaders={names}, Signature={signature}",
            self.access_key_id.expose()
        );
        Ok(Signed {
            amz_date,
            authorization,
        })
    }
}

/// SHA-256 of `bytes` in lowercase hex, as `X-Amz-Content-Sha256` wants.
pub(crate) fn payload_hash(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

/// The path as sent, already percent-encoded, or `/` for none. S3 signs it
/// without encoding it again.
fn path_of(uri: &Uri) -> &str {
    match uri.path() {
        "" => "/",
        path => path,
    }
}

/// The query's pairs, sorted, each written `name=value`.
fn query_of(uri: &Uri) -> String {
    let mut pairs: Vec<String> = uri
        .query()
        .unwrap_or("")
        .split('&')
        .filter(|pair| !pair.is_empty())
        .map(|pair| match pair.split_once('=') {
            Some(_) => pair.to_owned(),
            None => format!("{pair}="),
        })
        .collect();
    pairs.sort_unstable();
    pairs.join("&")
}

fn mac(key: &[u8], data: &[u8]) -> Result<Vec<u8>, MediaError> {
    let mut mac =
        Hmac::<Sha256>::new_from_slice(key).map_err(|source| MediaError::Sign { source })?;
    mac.update(data);
    Ok(mac.finalize().into_bytes().to_vec())
}

fn hex(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(text, "{byte:02x}");
    }
    text
}
