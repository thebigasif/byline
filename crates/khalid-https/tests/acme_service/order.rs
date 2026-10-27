//! The order part of the test ACME service: one order for one domain, its
//! check made for real against the server under test, and its certificate
//! request signed by a test authority.

use axum::extract::State;
use axum::http::StatusCode;
use axum::http::header::CONTENT_TYPE;
use axum::response::{IntoResponse, Response};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use rcgen::{CertificateSigningRequestParams, Issuer};
use rustls_pki_types::CertificateSigningRequestDer;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::{Book, Shared, answer, opened};
use crate::common::tls_handshake;

/// The order as JSON, with its certificate's address once there is one.
fn order_body(book: &Book) -> Value {
    let base = &book.base;
    let mut body = json!({
        "status": book.order,
        "identifiers": [{ "type": "dns", "value": book.domain }],
        "authorizations": [format!("{base}/authz/1")],
        "finalize": format!("{base}/finalize/1"),
    });
    if book.chain.is_some() {
        body["certificate"] = json!(format!("{base}/cert/1"));
    }
    body
}

pub(crate) async fn new_order(State(book): State<Shared>) -> Response {
    let book = book.lock().unwrap();
    let location = format!("{}/order/1", book.base);
    answer(StatusCode::CREATED, Some(location), &order_body(&book))
}

pub(crate) async fn order(State(book): State<Shared>) -> Response {
    answer(StatusCode::OK, None, &order_body(&book.lock().unwrap()))
}

pub(crate) async fn authorization(State(book): State<Shared>) -> Response {
    let book = book.lock().unwrap();
    let body = json!({
        "status": "pending",
        "identifier": { "type": "dns", "value": book.domain },
        "challenges": [{
            "type": "tls-alpn-01",
            "url": format!("{}/challenge/1", book.base),
            "token": "token-1",
            "status": "pending",
        }],
    });
    answer(StatusCode::OK, None, &body)
}

/// The key authorization the server under test must show.
fn expected(book: &Book) -> String {
    let jwk = book.account_key.as_ref().unwrap();
    let canonical = format!(
        r#"{{"crv":{},"kty":{},"x":{},"y":{}}}"#,
        jwk["crv"], jwk["kty"], jwk["x"], jwk["y"]
    );
    let thumbprint = URL_SAFE_NO_PAD.encode(Sha256::digest(canonical.as_bytes()));
    format!("token-1.{thumbprint}")
}

/// Makes the check for real, then marks the order ready or invalid.
pub(crate) async fn challenge(State(book): State<Shared>) -> Response {
    let (domain, port, expected) = {
        let book = book.lock().unwrap();
        (book.domain.clone(), book.tls_port, expected(&book))
    };
    let mut extension = vec![0x04, 0x20];
    extension.extend_from_slice(&Sha256::digest(expected.as_bytes()));
    let passed = tls_handshake(port, &domain, b"acme-tls/1")
        .await
        .is_some_and(|(certificate, _)| {
            certificate.windows(extension.len()).any(|w| w == extension)
        });
    let mut book = book.lock().unwrap();
    book.order = if passed { "ready" } else { "invalid" };
    let body = json!({
        "type": "tls-alpn-01",
        "url": format!("{}/challenge/1", book.base),
        "token": "token-1",
        "status": if passed { "valid" } else { "invalid" },
    });
    answer(StatusCode::OK, None, &body)
}

/// Signs the certificate request with the test authority.
pub(crate) async fn finalize(State(book): State<Shared>, body: String) -> Response {
    let (_, payload) = opened(&body);
    let der = URL_SAFE_NO_PAD
        .decode(payload["csr"].as_str().unwrap())
        .unwrap();
    let request =
        CertificateSigningRequestParams::from_der(&CertificateSigningRequestDer::from(der))
            .unwrap();
    let mut book = book.lock().unwrap();
    let chain = {
        let (params, key) = book.authority.as_ref().unwrap();
        let issuer = Issuer::from_params(params, key);
        request.signed_by(&issuer).unwrap().pem()
    };
    book.chain = Some(chain);
    book.order = "valid";
    book.issued += 1;
    answer(StatusCode::OK, None, &order_body(&book))
}

pub(crate) async fn certificate(State(book): State<Shared>) -> Response {
    let chain = book.lock().unwrap().chain.clone().unwrap();
    let mut response = (StatusCode::OK, chain).into_response();
    let headers = response.headers_mut();
    headers.insert("replay-nonce", "bm9uY2U".parse().unwrap());
    let pem = "application/pem-certificate-chain".parse().unwrap();
    headers.insert(CONTENT_TYPE, pem);
    response
}
