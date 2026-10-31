//! Reading a Substack export: `posts.csv` and the `posts/` folder.

use chrono::NaiveDate;
use khalid_import::substack::{Audience, PostKind, RowError, read_export};

const POSTS_CSV: &str = "post_id,post_date,is_published,email_sent_at,inbox_sent_at,type,audience,title,subtitle,podcast_url\n\
    140062712.why-rye,2024-03-12T09:30:44.123Z,true,2024-03-12T09:31:00.000Z,,newsletter,everyone,Why rye,\"A grain that waits, and wins\",\n\
    140062713.members-loaf,2024-04-02T08:00:00.000Z,true,,,newsletter,only_paid,The members loaf,,\n\
    140062714.spring-flour,,false,,,newsletter,everyone,Spring flour,,\n\
    140062715.ask-me,2024-05-01T10:00:00.000Z,true,,,thread,everyone,\"Ask me \"\"anything\"\"\",,\n\
    not-a-number,2024-05-02T10:00:00.000Z,true,,,newsletter,everyone,Broken,,\n\
    140062716.late,yesterday,true,,,newsletter,everyone,Late,,\n";

const FILES: [(&str, &str); 3] = [
    ("140062712.why-rye.html", "<p>Rye needs time.</p>"),
    ("140062713.members-loaf.html", "<p>For members.</p>"),
    ("140062712.why-rye.opens.csv", "email,opened\n"),
];

#[test]
fn each_row_becomes_a_post_with_its_body_and_slug() {
    let export = read_export(POSTS_CSV, FILES).unwrap();
    let rye = &export.posts[0];
    let read = (rye.id.as_str(), rye.slug.as_str(), rye.title.as_str());
    assert_eq!(read, ("140062712", "why-rye", "Why rye"));
    let subtitle = Some("A grain that waits, and wins");
    assert_eq!(rye.subtitle.as_deref(), subtitle);
    let date = NaiveDate::from_ymd_opt(2024, 3, 12).unwrap();
    assert_eq!(rye.date.map(|at| at.date()), Some(date));
    assert_eq!((rye.published, &rye.kind), (true, &PostKind::Newsletter));
    assert_eq!(rye.audience, Audience::Everyone);
    assert_eq!(rye.body.as_deref(), Some("<p>Rye needs time.</p>"));
}

#[test]
fn audiences_drafts_threads_and_missing_bodies_are_read_as_written() {
    let export = read_export(POSTS_CSV, FILES).unwrap();
    let [_, loaf, spring, thread] = &export.posts[..] else {
        panic!("{:?}", export.posts);
    };
    assert_eq!(loaf.audience, Audience::Paid);
    assert!(!spring.published);
    assert_eq!((spring.date, spring.body.as_deref()), (None, None));
    assert_eq!(thread.kind, PostKind::Thread);
    assert_eq!(thread.title, "Ask me \"anything\"");
}

#[test]
fn rows_that_cannot_be_read_are_listed_with_their_line() {
    let export = read_export(POSTS_CSV, FILES).unwrap();
    let [id, date] = &export.unread[..] else {
        panic!("{:?}", export.unread);
    };
    assert_eq!(
        (id.line, &id.error),
        (6, &RowError::Id("not-a-number".to_owned()))
    );
    assert_eq!(
        (date.line, &date.error),
        (7, &RowError::Date("yesterday".to_owned()))
    );
}

#[test]
fn another_file_of_the_export_is_refused() {
    let subscribers = "email,active_subscription,created_at\nmara@example.com,true,2024-01-01\n";
    let refused = read_export(subscribers, FILES).unwrap_err();
    assert_eq!(
        refused.to_string(),
        "posts.csv has no post_id column, so it is not the post list of a Substack export"
    );
}

#[test]
fn a_quote_never_closed_is_refused() {
    let broken =
        format!("{POSTS_CSV}140062717.open,2024-06-01T10:00:00Z,true,,,newsletter,everyone,\"Open");
    let refused = read_export(&broken, FILES).unwrap_err();
    assert_eq!(
        refused.to_string(),
        "posts.csv cannot be read: a quote opened on line 8 is never closed"
    );
}
