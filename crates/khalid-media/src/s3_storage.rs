//! Files kept in an S3-compatible bucket, such as Cloudflare R2.

use std::sync::Arc;

use khalid_core::{Clock, S3Keys, S3Settings};
use khalid_outbound::Outbound;

use crate::sig_v4::payload_hash;
use crate::storage::stored_as;
use crate::{ContentKey, FileName, MediaError, MediaType, SigV4};

/// The service name bucket requests are signed for.
const SERVICE: &str = "s3";

/// A bucket, reached through the one outbound client (RULES.md BL-RUN-8),
/// so its address must be public, and every request is signed.
#[derive(Debug)]
pub struct S3Storage {
    /// `<endpoint>/<bucket>/<prefix>`, path style, which R2, S3 and
    /// `MinIO` all accept.
    base: String,
    /// The public address with its prefix, ending in `/`.
    public: String,
    signer: SigV4,
    outbound: Outbound,
    clock: Arc<dyn Clock>,
}

impl S3Storage {
    /// The bucket `settings` names, signed with `keys` at `clock`'s time.
    pub(crate) fn new(
        settings: &S3Settings,
        keys: S3Keys,
        outbound: Outbound,
        clock: Arc<dyn Clock>,
    ) -> Self {
        let endpoint = settings.endpoint.as_str().trim_end_matches('/');
        let public = settings.public_address.as_str().trim_end_matches('/');
        Self {
            base: format!("{endpoint}/{}/{}", settings.bucket, settings.prefix),
            public: format!("{public}/{}", settings.prefix),
            signer: SigV4::new(keys, &settings.region, SERVICE),
            outbound,
            clock,
        }
    }

    /// Stores `bytes` as the object for `name` of `key`.
    pub(crate) async fn put(
        &self,
        key: &ContentKey,
        name: &FileName,
        bytes: Vec<u8>,
        media_type: MediaType,
    ) -> Result<(), MediaError> {
        let hash = payload_hash(&bytes);
        let headers = [
            ("content-type", media_type.mime()),
            ("x-amz-content-sha256", hash.as_str()),
        ];
        let answer = self.send("PUT", key, name, &headers, bytes).await?;
        match answer.status().as_u16() {
            200..=299 => Ok(()),
            status => Err(MediaError::Bucket { status }),
        }
    }

    /// The object for `name` of `key`, or `None` when the bucket has none.
    pub(crate) async fn get(
        &self,
        key: &ContentKey,
        name: &FileName,
    ) -> Result<Option<Vec<u8>>, MediaError> {
        let empty = payload_hash(b"");
        let headers = [("x-amz-content-sha256", empty.as_str())];
        let answer = self.send("GET", key, name, &headers, Vec::new()).await?;
        match answer.status().as_u16() {
            200 => Ok(Some(answer.into_body())),
            404 => Ok(None),
            status => Err(MediaError::Bucket { status }),
        }
    }

    /// Where readers fetch the object for `name` of `key`.
    pub(crate) fn address(&self, key: &ContentKey, name: &FileName) -> String {
        format!("{}{}", self.public, stored_as("", key, name))
    }

    /// Signs and sends one request for the object of `name` of `key`.
    async fn send(
        &self,
        method: &str,
        key: &ContentKey,
        name: &FileName,
        headers: &[(&str, &str)],
        body: Vec<u8>,
    ) -> Result<http::Response<Vec<u8>>, MediaError> {
        let url = format!("{}{}", self.base, stored_as("", key, name));
        let uri: http::Uri = url.parse().map_err(|_| MediaError::BucketAddress)?;
        let hash = headers
            .iter()
            .find(|(header, _)| *header == "x-amz-content-sha256")
            .map_or("", |(_, value)| value);
        let signed = self
            .signer
            .sign(method, &uri, headers, hash, self.clock.now())?;
        let mut request = http::Request::builder().method(method).uri(uri);
        for &(header, value) in headers {
            request = request.header(header, value);
        }
        let request = request
            .header("x-amz-date", signed.amz_date)
            .header("authorization", signed.authorization)
            .body(body)
            .map_err(|_| MediaError::BucketAddress)?;
        self.outbound
            .exchange(request)
            .await
            .map_err(|source| MediaError::Reach { source })
    }
}
