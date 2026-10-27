//! The server answers on any listener, not only a plain TCP one, so
//! `khalid serve --domain` can hand it a listener that speaks TLS.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use axum::serve::ListenerExt;
use khalid_server::{MediaType, Page, PagePath, Pages, SecurityHeaders, Stopped, router, serve};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;

#[tokio::test]
async fn answers_on_a_listener_that_wraps_each_connection() {
    let pages = Pages::new(
        vec![Page::new(
            PagePath::try_new("/blog/").unwrap(),
            MediaType::Html,
            "<h1>The blog</h1>",
        )],
        "<h1>Page not found</h1>",
    )
    .unwrap();
    let tcp = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = tcp.local_addr().unwrap();
    let accepted = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&accepted);
    let listener = tcp.tap_io(move |_| {
        counter.fetch_add(1, Ordering::SeqCst);
    });
    let (stop, stopped) = oneshot::channel::<()>();
    let shutdown = async move {
        let _ = stopped.await;
    };
    let app = router(pages, SecurityHeaders::default());
    let server = tokio::spawn(serve(listener, app, shutdown, Duration::from_secs(5)));

    let mut stream = TcpStream::connect(address).await.unwrap();
    stream
        .write_all(b"GET /blog/ HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .await
        .unwrap();
    let mut bytes = Vec::new();
    stream.read_to_end(&mut bytes).await.unwrap();
    let response = String::from_utf8(bytes).unwrap();

    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
    assert!(response.ends_with("<h1>The blog</h1>"), "{response}");
    assert_eq!(accepted.load(Ordering::SeqCst), 1);
    stop.send(()).unwrap();
    assert_eq!(server.await.unwrap().unwrap(), Stopped::Finished);
}
