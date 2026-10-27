//! A small ACME service for tests, speaking enough of RFC 8555 to open
//! accounts. It does not check the requests' signatures.

use std::sync::{Arc, Mutex};

use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::http::header::{CONTENT_TYPE, LOCATION};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde_json::{Value, json};
use tokio::net::TcpListener;

/// The service, and where it answers.
pub(crate) struct AcmeService {
    /// The directory address to sign up at.
    pub(crate) directory: String,
    book: Shared,
}

impl AcmeService {
    /// How many accounts were opened.
    pub(crate) fn accounts(&self) -> usize {
        self.book.lock().unwrap().accounts
    }
}

/// What the service remembers.
#[derive(Default)]
pub(crate) struct Book {
    pub(crate) base: String,
    pub(crate) accounts: usize,
}

pub(crate) type Shared = Arc<Mutex<Book>>;

/// Starts the service on a free loopback port.
pub(crate) async fn start() -> AcmeService {
    let tcp = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://127.0.0.1:{}", tcp.local_addr().unwrap().port());
    let book = Arc::new(Mutex::new(Book {
        base: base.clone(),
        ..Book::default()
    }));
    let app = Router::new()
        .route("/dir", get(directory))
        .route("/nonce", get(nonce).head(nonce))
        .route("/account", post(account))
        .with_state(Arc::clone(&book));
    tokio::spawn(async move { axum::serve(tcp, app).await });
    AcmeService {
        directory: format!("{base}/dir"),
        book,
    }
}

/// An answer with a fresh nonce, as every ACME answer carries.
pub(crate) fn answer(status: StatusCode, location: Option<String>, body: &Value) -> Response {
    let mut response = (status, body.to_string()).into_response();
    let headers = response.headers_mut();
    headers.insert("replay-nonce", "bm9uY2U".parse().unwrap());
    headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
    if let Some(location) = location {
        headers.insert(LOCATION, location.parse().unwrap());
    }
    response
}

/// The protected header and the payload of a JWS request, as JSON.
pub(crate) fn opened(body: &str) -> (Value, Value) {
    let jws: Value = serde_json::from_str(body).unwrap();
    let part = |name: &str| -> Value {
        let bytes = URL_SAFE_NO_PAD.decode(jws[name].as_str().unwrap()).unwrap();
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };
    (part("protected"), part("payload"))
}

async fn directory(State(book): State<Shared>) -> Response {
    let base = book.lock().unwrap().base.clone();
    let body = json!({
        "newNonce": format!("{base}/nonce"),
        "newAccount": format!("{base}/account"),
        "newOrder": format!("{base}/order"),
    });
    answer(StatusCode::OK, None, &body)
}

async fn nonce() -> Response {
    answer(StatusCode::OK, None, &Value::Null)
}

async fn account(State(book): State<Shared>, body: String) -> Response {
    let (_, payload) = opened(&body);
    assert_eq!(payload["termsOfServiceAgreed"], true, "terms agreed");
    let mut book = book.lock().unwrap();
    book.accounts += 1;
    let location = format!("{}/account/1", book.base);
    answer(
        StatusCode::CREATED,
        Some(location),
        &json!({ "status": "valid" }),
    )
}
