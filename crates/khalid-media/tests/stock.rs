//! Photo search: Unsplash stood in for by `wiremock` (RULES.md BL-TST-3).
//! Every key here is made up for the test.

use khalid_core::Secret;
use khalid_media::{Access, PictureSource, StockApis, StockError, StockSearch};
use khalid_outbound::{Limits, Outbound, Reach, Resolver};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const UNSPLASH_KEY: &str = "made-for-this-test-unsplash";

/// Search asking `base`.
#[cfg(test)]
fn search_at(base: &str, unsplash: Access) -> StockSearch {
    let resolver = Resolver::for_tests(Vec::new(), Reach::PublicAndLoopback);
    let outbound = Outbound::with_resolver(Limits::default(), resolver).unwrap();
    let apis = StockApis {
        unsplash: base.to_owned(),
    };
    StockSearch::new(outbound, unsplash).at(apis)
}

#[cfg(test)]
fn search(server: &MockServer, unsplash: Access) -> StockSearch {
    search_at(&format!("{}/", server.uri()), unsplash)
}

#[cfg(test)]
fn keyed(key: &str) -> Access {
    Access::on_with(Some(Secret::new(key.to_owned())))
}

#[cfg(test)]
fn both(server: &MockServer) -> StockSearch {
    search(server, keyed(UNSPLASH_KEY))
}

#[tokio::test]
async fn asks_nothing_when_off_or_without_a_key() {
    let server = MockServer::start().await;
    let off = search(&server, Access::Off);
    assert!(!off.is_on(PictureSource::Unsplash));
    let unused = off.used("Ab-12_c").await;
    assert!(matches!(unused, Err(StockError::Off)), "{unused:?}");
    let keyless = search(&server, Access::on_with(None));
    assert!(keyless.is_on(PictureSource::Unsplash));
    let missing = keyless.used("Ab-12_c").await;
    assert!(
        matches!(
            missing,
            Err(StockError::NoKey {
                variable: "KHALID_UNSPLASH_ACCESS_KEY"
            })
        ),
        "{missing:?}"
    );
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn reads_a_refused_key_and_a_used_up_hour() {
    for (status, body, refused) in [
        (401, "OAuth error: The access token is invalid", true),
        (403, "Rate Limit Exceeded", false),
        (429, "", false),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(status).set_body_string(body))
            .mount(&server)
            .await;
        let answer = both(&server).used("Ab-12_c").await;
        if refused {
            assert!(matches!(answer, Err(StockError::KeyRefused)), "{answer:?}");
        } else {
            assert!(matches!(answer, Err(StockError::Busy)), "{answer:?}");
        }
    }
}

#[tokio::test]
async fn a_failure_never_shows_the_key() {
    // wiremock keeps a dropped server answering, so ask a port nobody holds.
    let closed = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}/", closed.local_addr().unwrap());
    drop(closed);
    let stock = search_at(&base, keyed(UNSPLASH_KEY));
    let answer = stock.used("Ab-12_c").await;
    let Err(error @ StockError::Unreachable { .. }) = answer else {
        panic!("unreachable, found {answer:?}");
    };
    let shown = format!("{error} {error:?}");
    assert!(!shown.contains(UNSPLASH_KEY), "{shown}");
}

#[tokio::test]
async fn tells_unsplash_a_photo_was_used() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/photos/Ab-12_c/download"))
        .and(header(
            "authorization",
            "Client-ID made-for-this-test-unsplash",
        ))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;
    both(&server).used("Ab-12_c").await.unwrap();
}

#[tokio::test]
async fn refuses_an_identity_unsplash_never_makes_without_asking() {
    let server = MockServer::start().await;
    let stock = both(&server);
    for id in ["", "../search", "a b", "a?b"] {
        let answer = stock.used(id).await;
        assert!(
            matches!(answer, Err(StockError::NotFound)),
            "{id}: {answer:?}"
        );
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}
