//! The requests-per-second measure reads whole keep-alive responses and
//! counts only good ones.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::thread;
use std::time::Duration;

use khalid_core::SystemClock;
use khalid_perf::{RequestRate, read_response};

const PAGE: &[u8] = b"HTTP/1.1 200 OK\r\ncontent-length: 5\r\ncontent-type: text/html\r\n\r\nhello";

#[test]
fn a_response_is_read_to_the_end_of_its_body() {
    let two = [PAGE, PAGE].concat();
    let mut reader = &two[..];
    read_response(&mut reader).unwrap();
    read_response(&mut reader).unwrap();
    assert!(reader.is_empty(), "both bodies read, nothing left over");
}

#[test]
fn a_status_other_than_200_is_an_error() {
    let mut reader = &b"HTTP/1.1 404 Not Found\r\ncontent-length: 0\r\n\r\n"[..];
    let error = read_response(&mut reader).unwrap_err();
    assert!(error.to_string().contains("404"), "{error}");
}

#[test]
fn a_response_without_a_length_is_an_error() {
    let mut reader =
        &b"HTTP/1.1 200 OK\r\ntransfer-encoding: chunked\r\n\r\n5\r\nhello\r\n0\r\n\r\n"[..];
    assert!(read_response(&mut reader).is_err());
}

#[test]
fn the_json_names_the_baseline_measure() {
    let json = RequestRate::new(25_000, 2.5).to_json();
    assert_eq!(json, "{\"cached_post_requests_per_second\": 10000.00}\n");
    assert_eq!(RequestRate::new(7, 0.0).per_second(), 0.0);
}

#[test]
fn every_answered_request_is_counted_over_the_run() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let host = listener.local_addr().unwrap().to_string();
    thread::spawn(move || {
        for stream in listener.incoming() {
            let stream = stream.unwrap();
            thread::spawn(move || answer_each_request(stream));
        }
    });
    let rate = RequestRate::measure(
        &host,
        "/blog/a-post/",
        (2, Duration::from_millis(300)),
        &SystemClock,
    )
    .unwrap();
    assert!(rate.answered() > 2, "{rate:?}");
    assert!(rate.per_second() > 0.0, "{rate:?}");
}

#[cfg(test)]
fn answer_each_request(stream: std::net::TcpStream) {
    let mut writer = stream.try_clone().unwrap();
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    loop {
        line.clear();
        if reader.read_line(&mut line).unwrap_or(0) == 0 {
            return;
        }
        if line == "\r\n" && writer.write_all(PAGE).is_err() {
            return;
        }
    }
}

#[test]
fn a_server_that_stops_answering_is_an_error() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let host = listener.local_addr().unwrap().to_string();
    thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = stream.unwrap();
            let mut request = [0_u8; 1024];
            let _ = stream.read(&mut request);
            // Closes the connection without an answer.
        }
    });
    let measured = RequestRate::measure(&host, "/", (1, Duration::from_millis(200)), &SystemClock);
    assert!(measured.is_err(), "{measured:?}");
}
