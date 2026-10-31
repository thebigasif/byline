//! Photo and GIF search: Unsplash and GIPHY stood in for by `wiremock`
//! (RULES.md BL-TST-3). Every key here is made up for the test.

use khalid_core::Secret;
use khalid_media::{Access, PictureSource, StockApis, StockError, StockSearch};
use khalid_outbound::{Limits, Outbound, Reach, Resolver};
use serde_json::json;
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

const UNSPLASH_KEY: &str = "made-for-this-test-unsplash";
const GIPHY_KEY: &str = "made-for-this-test-giphy";

/// Search asking `base` for both sources.
#[cfg(test)]
fn search_at(base: &str, unsplash: Access, giphy: Access) -> StockSearch {
    let resolver = Resolver::for_tests(Vec::new(), Reach::PublicAndLoopback);
    let outbound = Outbound::with_resolver(Limits::default(), resolver).unwrap();
    let apis = StockApis {
        unsplash: base.to_owned(),
        giphy: base.to_owned(),
    };
    StockSearch::new(outbound, unsplash)
        .with_gifs(giphy)
        .at(apis)
}

#[cfg(test)]
fn search(server: &MockServer, unsplash: Access, giphy: Access) -> StockSearch {
    search_at(&format!("{}/", server.uri()), unsplash, giphy)
}

#[cfg(test)]
fn keyed(key: &str) -> Access {
    Access::on_with(Some(Secret::new(key.to_owned())))
}

#[cfg(test)]
fn both(server: &MockServer) -> StockSearch {
    search(server, keyed(UNSPLASH_KEY), keyed(GIPHY_KEY))
}

#[cfg(test)]
fn lake() -> serde_json::Value {
    json!({
        "total": 1,
        "results": [{
            "id": "Ab-12_c",
            "width": 4000,
            "height": 3000,
            "description": null,
            "alt_description": "a boat on a still lake at dawn",
            "urls": {
                "regular": "https://images.unsplash.com/photo-1?w=1080",
                "small": "https://images.unsplash.com/photo-1?w=400"
            },
            "links": { "html": "https://unsplash.com/photos/Ab-12_c" },
            "user": {
                "name": "Ana Silva",
                "links": { "html": "https://unsplash.com/@anasilva" }
            }
        }]
    })
}

#[tokio::test]
async fn finds_photos_on_unsplash_with_the_key_in_a_header() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/search/photos"))
        .and(query_param("query", "mountain lake"))
        .and(query_param("page", "2"))
        .and(query_param("content_filter", "high"))
        .and(header(
            "authorization",
            "Client-ID made-for-this-test-unsplash",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(lake()))
        .expect(1)
        .mount(&server)
        .await;
    let found = both(&server)
        .search(PictureSource::Unsplash, " mountain lake ", 2)
        .await
        .unwrap();
    let [photo] = found.as_slice() else {
        panic!("one photo, found {found:?}");
    };
    assert_eq!(photo.source, PictureSource::Unsplash);
    assert_eq!(photo.id, "Ab-12_c");
    assert_eq!(photo.description, "a boat on a still lake at dawn");
    assert_eq!(photo.address, "https://images.unsplash.com/photo-1?w=1080");
    assert_eq!(photo.preview, "https://images.unsplash.com/photo-1?w=400");
    assert_eq!((photo.width, photo.height), (1080, 810));
    assert_eq!(photo.credit, "Ana Silva");
    assert_eq!(
        photo.credit_link,
        "https://unsplash.com/@anasilva?utm_source=khalid&utm_medium=referral"
    );
    assert_eq!(
        photo.page,
        "https://unsplash.com/photos/Ab-12_c?utm_source=khalid&utm_medium=referral"
    );
}

#[tokio::test]
async fn finds_gifs_on_giphy_rated_g_with_sizes_written_as_text() {
    let server = MockServer::start().await;
    let page = json!({
        "data": [{
            "id": "xT0x",
            "title": "Happy Dance GIF",
            "url": "https://giphy.com/gifs/xT0x",
            "images": {
                "fixed_width": { "url": "https://media.giphy.com/xT0x/200w.gif", "width": "200", "height": "150" },
                "downsized_medium": { "url": "https://media.giphy.com/xT0x/giphy.gif", "width": "480", "height": "360" },
                "original": { "url": "https://media.giphy.com/xT0x/original.gif", "width": "960", "height": "720" }
            }
        }]
    });
    Mock::given(method("GET"))
        .and(path("/v1/gifs/search"))
        .and(query_param("api_key", GIPHY_KEY))
        .and(query_param("q", "happy dance"))
        .and(query_param("offset", "24"))
        .and(query_param("rating", "g"))
        .respond_with(ResponseTemplate::new(200).set_body_json(page))
        .expect(1)
        .mount(&server)
        .await;
    let found = both(&server)
        .search(PictureSource::Giphy, "happy dance", 2)
        .await
        .unwrap();
    let [gif] = found.as_slice() else {
        panic!("one GIF, found {found:?}");
    };
    assert_eq!(gif.address, "https://media.giphy.com/xT0x/giphy.gif");
    assert_eq!((gif.width, gif.height), (480, 360));
    assert_eq!(gif.preview, "https://media.giphy.com/xT0x/200w.gif");
    assert_eq!(gif.credit, "GIPHY");
    assert_eq!(gif.credit_link, "https://giphy.com/gifs/xT0x");
    assert_eq!(gif.description, "Happy Dance GIF");
}

#[tokio::test]
async fn asks_nothing_when_off_or_without_a_key() {
    let server = MockServer::start().await;
    let stock = search(&server, Access::Off, Access::on_with(None));
    assert!(!stock.is_on(PictureSource::Unsplash));
    assert!(stock.is_on(PictureSource::Giphy));
    let unused = stock.used("Ab-12_c").await;
    assert!(matches!(unused, Err(StockError::Off)), "{unused:?}");
    let off = stock.search(PictureSource::Unsplash, "lake", 1).await;
    assert!(matches!(off, Err(StockError::Off)), "{off:?}");
    let missing = stock.search(PictureSource::Giphy, "lake", 1).await;
    assert!(
        matches!(
            missing,
            Err(StockError::NoKey {
                variable: "KHALID_GIPHY_API_KEY"
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
    let stock = search_at(&base, keyed(UNSPLASH_KEY), keyed(GIPHY_KEY));
    let answer = stock.search(PictureSource::Giphy, "lake", 1).await;
    let Err(error @ StockError::Unreachable { .. }) = answer else {
        panic!("unreachable, found {answer:?}");
    };
    let shown = format!("{error} {error:?}");
    assert!(!shown.contains(GIPHY_KEY), "{shown}");
}

#[tokio::test]
async fn an_answer_that_is_not_json_is_unreadable() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string("<html>"))
        .mount(&server)
        .await;
    let answer = both(&server)
        .search(PictureSource::Unsplash, "lake", 1)
        .await;
    assert!(matches!(answer, Err(StockError::Unreadable)), "{answer:?}");
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
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "url": "x" })))
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
