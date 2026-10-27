//! The ACME client's requests, sent through the one outbound client.

use std::future::Future;
use std::pin::Pin;

use bytes::Bytes;
use http::Request;
use http_body_util::BodyExt;
use instant_acme::{BodyWrapper, BytesResponse, HttpClient};
use khalid_outbound::Outbound;

/// The ACME client's HTTP, through [`Outbound`] (RULES.md BL-RUN-8), so
/// its timeouts, size cap and private-network guard apply.
pub(crate) struct Through(Outbound);

impl Through {
    /// The client as the ACME library takes it.
    pub(crate) fn boxed(outbound: &Outbound) -> Box<dyn HttpClient> {
        Box::new(Self(outbound.clone()))
    }
}

impl HttpClient for Through {
    fn request(
        &self,
        request: Request<BodyWrapper<Bytes>>,
    ) -> Pin<Box<dyn Future<Output = Result<BytesResponse, instant_acme::Error>> + Send>> {
        let outbound = self.0.clone();
        Box::pin(async move {
            let (parts, body) = request.into_parts();
            let body = match body.collect().await {
                Ok(collected) => collected.to_bytes(),
                Err(never) => match never {},
            };
            let request = Request::from_parts(parts, body.to_vec());
            let response = outbound
                .exchange(request)
                .await
                .map_err(|error| instant_acme::Error::Other(Box::new(error)))?;
            let (parts, body) = response.into_parts();
            Ok(BytesResponse {
                parts,
                body: Box::new(Bytes::from(body)),
            })
        })
    }
}
