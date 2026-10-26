//! The server stops gracefully (RULES.md BL-RUN-10): it stops accepting,
//! finishes the requests it has, and gives up on the rest after the grace
//! period.

use std::time::Duration;

use khalid_server::{MediaType, Page, PagePath, Pages, Stopped, router, serve};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

/// Long enough for the server to read what a test has sent.
const SETTLE: Duration = Duration::from_millis(200);

/// A server on a free loopback port, and the switch that stops it.
struct Running {
    address: std::net::SocketAddr,
    stop: oneshot::Sender<()>,
    server: JoinHandle<Result<Stopped, khalid_server::ServerError>>,
}

#[cfg(test)]
async fn start(grace: Duration) -> Running {
    let pages = Pages::new(
        vec![Page::new(
            PagePath::try_new("/blog/").unwrap(),
            MediaType::Html,
            "<h1>The blog</h1>",
        )],
        "<h1>Page not found</h1>",
    )
    .unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (stop, stopped) = oneshot::channel::<()>();
    let shutdown = async move {
        let _ = stopped.await;
    };
    let server = tokio::spawn(serve(listener, router(pages), shutdown, grace));
    Running {
        address,
        stop,
        server,
    }
}

#[cfg(test)]
async fn read_response(stream: &mut TcpStream) -> String {
    let mut bytes = Vec::new();
    stream.read_to_end(&mut bytes).await.unwrap();
    String::from_utf8(bytes).unwrap()
}

#[tokio::test]
async fn finishes_a_request_in_flight_then_stops() {
    let running = start(Duration::from_secs(5)).await;
    let mut stream = TcpStream::connect(running.address).await.unwrap();
    stream
        .write_all(b"GET /blog/ HTTP/1.1\r\nHost: localhost\r\n")
        .await
        .unwrap();
    tokio::time::sleep(SETTLE).await;

    running.stop.send(()).unwrap();
    tokio::time::sleep(SETTLE).await;
    stream.write_all(b"\r\n").await.unwrap();

    let response = read_response(&mut stream).await;
    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
    assert!(response.ends_with("<h1>The blog</h1>"), "{response}");
    let stopped = running.server.await.unwrap().unwrap();
    assert_eq!(stopped, Stopped::Finished);
    assert!(TcpStream::connect(running.address).await.is_err());
}

#[tokio::test]
async fn gives_up_on_a_stalled_request_after_the_grace_period() {
    let running = start(Duration::from_millis(300)).await;
    let mut stream = TcpStream::connect(running.address).await.unwrap();
    stream.write_all(b"GET /blog/ HTTP/1.1\r\n").await.unwrap();
    tokio::time::sleep(SETTLE).await;

    running.stop.send(()).unwrap();
    let stopped = tokio::time::timeout(Duration::from_secs(5), running.server)
        .await
        .expect("the server stops within the grace period")
        .unwrap()
        .unwrap();
    assert_eq!(stopped, Stopped::GraceExpired);
}
