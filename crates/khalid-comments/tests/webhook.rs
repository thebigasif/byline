//! Each stored comment fires `comment.added` once, signed, without the
//! reader's email. The receiver is a wiremock server on loopback.

use std::sync::Arc;
use std::time::Duration;

use chrono::{TimeZone, Utc};
use khalid_comments::{
    AuthorName, CommentAuthor, CommentEvents, CommentText, Comments, MemoryCommentStore,
    NewComment, PostFile, ReaderEmail, WebhookEvents,
};
use khalid_core::{AbsoluteUrl, FixedClock, Secret};
use khalid_outbound::{Limits, Outbound, Reach, Resolver};
use khalid_webhooks::{Dispatcher, Event, MemoryDeliveryStore, SigningKey, Webhook, WebhookId};
use serde_json::{Value, json};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// A key made up for this test. It protects nothing.
const KEY: &str = "test-key-generated-for-comment-webhook-tests";

#[tokio::test]
async fn a_readers_comment_reaches_the_webhook_once_without_the_email() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/hooks"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 10, 27, 9, 0, 0).unwrap());
    let resolver = Resolver::for_tests(vec![], Reach::PublicAndLoopback);
    let outbound = Outbound::with_resolver(Limits::default(), resolver).unwrap();
    let site = AbsoluteUrl::parse("https://example.com/blog/").unwrap();
    let store = MemoryDeliveryStore::default();
    let dispatcher = Arc::new(Dispatcher::new(outbound, store, clock, site));
    let key = SigningKey::try_new(&Secret::new(KEY.to_owned())).unwrap();
    let url = AbsoluteUrl::parse(&format!("{}/hooks", server.uri())).unwrap();
    let hook = Webhook::new(WebhookId::new(1), url, key, vec![Event::CommentAdded]);
    let events: Arc<dyn CommentEvents> = Arc::new(WebhookEvents::new(dispatcher, vec![hook]));
    let comments = Comments::new(MemoryCommentStore::default(), clock).with_events(events);
    let new = NewComment {
        post: PostFile::new("content/en/posts/why-links-matter.md"),
        parent: None,
        author: CommentAuthor::Reader {
            name: AuthorName::parse("Lena").unwrap(),
            email: Some(ReaderEmail::parse("lena@example.com").unwrap()),
        },
        text: CommentText::parse("Great post.").unwrap(),
    };
    comments.submit(new).await.unwrap();
    let mut received = Vec::new();
    for _ in 0..100 {
        received = server.received_requests().await.unwrap();
        if !received.is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert_eq!(received.len(), 1);
    let header = received[0].headers.get("khalid-event").unwrap();
    assert_eq!(header.to_str().unwrap(), "comment.added");
    let body: Value = serde_json::from_slice(&received[0].body).unwrap();
    assert_eq!(
        body,
        json!({
            "event": "comment.added",
            "occurred_at": "2026-10-27T09:00:00Z",
            "site": "https://example.com/blog/",
            "comment": {
                "id": 1,
                "post": "content/en/posts/why-links-matter.md",
                "parent": null,
                "author_name": "Lena",
                "author_kind": "reader",
                "text": "Great post.",
                "status": "pending",
                "created_at": "2026-10-27T09:00:00Z"
            }
        })
    );
}
