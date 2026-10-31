//! `khalid newsletter label` and `segment`: labels given and taken away
//! from the command line, and segments counted as totals, never as a list
//! of people (G39). `send` emails a published post once through a stub
//! SMTP relay, each copy with an unsubscribe link and the postal address,
//! and sends nothing without a postal address (task 5.8).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use khalid_core::{Clock, Slug, SystemClock};
use khalid_db::{Database, SubscriberTable};
use khalid_mail::StubSmtp;
use khalid_newsletter::{EmailAddress, SignUp, SubscriberStore, SubscriberToken};
use serde_json::Value;

/// A copy of the demo site, so the database it makes stays out of the
/// fixture.
#[cfg(test)]
fn demo_copy() -> tempfile::TempDir {
    let mut demo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    demo.push("../../fixtures/demo-site");
    let temp = tempfile::tempdir().unwrap();
    copy_dir(&demo, temp.path());
    temp
}

#[cfg(test)]
fn copy_dir(from: &Path, into: &Path) {
    fs::create_dir_all(into).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let mut target = into.to_path_buf();
        target.push(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).unwrap();
        }
    }
}

/// Confirmed subscribers of `main` at `addresses`, in the site's database.
#[cfg(test)]
async fn subscribers(site: &Path, addresses: &[&str]) {
    let mut path = site.to_path_buf();
    path.push("data");
    fs::create_dir_all(&path).unwrap();
    path.push("site.db");
    let db = Database::open(&path).await.unwrap();
    db.migrate().await.unwrap();
    let id = db.site(&Slug::try_new("main").unwrap()).await.unwrap();
    let table = SubscriberTable::new(db.clone(), id);
    for address in addresses {
        let token = SubscriberToken::generate().unwrap();
        let email = EmailAddress::parse(address).unwrap();
        let main = Slug::try_new("main").unwrap();
        table
            .sign_up(&SignUp::new(email, main, token.hash(), SystemClock.now()))
            .await
            .unwrap();
        table
            .confirm(&token.hash(), SystemClock.now())
            .await
            .unwrap();
    }
    db.close().await;
}

/// Runs `khalid --site <site> newsletter` with `args`; returns the exit
/// code and stdout.
#[cfg(test)]
fn newsletter(site: &Path, args: &[&str]) -> (Option<i32>, String) {
    let Output { status, stdout, .. } = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .arg("--site")
        .arg(site)
        .arg("newsletter")
        .args(args)
        .output()
        .unwrap();
    (status.code(), String::from_utf8_lossy(&stdout).into_owned())
}

#[cfg(test)]
fn counted(site: &Path, labels: &[&str]) -> u64 {
    let mut args = vec!["segment", "--json"];
    for label in labels {
        args.extend(["--label", label]);
    }
    let (code, out) = newsletter(site, &args);
    assert_eq!(code, Some(0), "{out}");
    let json: Value = serde_json::from_str(&out).unwrap();
    json["subscribers"].as_u64().unwrap()
}

#[tokio::test]
async fn a_label_narrows_the_segment_and_taking_it_away_widens_it_again() {
    let site = demo_copy();
    subscribers(site.path(), &["grace@example.com", "ada@example.com"]).await;
    assert_eq!(counted(site.path(), &[]), 2);
    let (code, out) = newsletter(site.path(), &["label", "grace@example.com", "paid"]);
    assert_eq!(code, Some(0), "{out}");
    assert_eq!(out, "Added the label paid.\n");
    assert_eq!(counted(site.path(), &["paid"]), 1);
    let removing = ["label", "grace@example.com", "paid", "--remove"];
    let (code, out) = newsletter(site.path(), &removing);
    assert_eq!(code, Some(0), "{out}");
    assert_eq!(counted(site.path(), &["paid"]), 0);
    let (code, out) = newsletter(site.path(), &["segment"]);
    assert_eq!(code, Some(0));
    assert_eq!(out, "2 confirmed subscribers of main.\n");
}

#[tokio::test]
async fn a_stranger_cannot_be_labelled() {
    let site = demo_copy();
    subscribers(site.path(), &[]).await;
    let (code, _) = newsletter(site.path(), &["label", "nobody@example.com", "paid"]);
    assert_eq!(code, Some(1));
}

const POST: &str = "what-a-sitemap-does";

const POSTAL: &str = "Field Notes Ltd, 1 Park Row, Leeds LS1 5AB, UK";

/// Appends `[mail]` sending through `stub` to the site's settings, with
/// `postal_address` when one is given.
#[cfg(test)]
fn mail_through(site: &Path, stub: &StubSmtp, postal: Option<&str>) {
    let mut settings = site.to_path_buf();
    settings.push("site.toml");
    let text = fs::read_to_string(&settings).unwrap();
    let postal = postal.map_or_else(String::new, |p| format!("postal_address = \"{p}\"\n"));
    let text = format!(
        "{text}\n[mail]\nfrom = \"news@example.com\"\nname = \"Field Notes\"\n{postal}\n\
         [mail.smtp]\nhost = \"127.0.0.1\"\nport = {}\nsecurity = \"none\"\nprivate_host = true\n",
        stub.port()
    );
    fs::write(&settings, text).unwrap();
}

/// `message` with the lines the mail encoding broke joined back up.
#[cfg(test)]
fn joined(message: &str) -> String {
    let message = message.replace("=\r\n", "").replace("=\n", "");
    message.replace("=3D", "=")
}

#[tokio::test]
async fn a_published_post_goes_out_once_with_a_link_out_and_the_postal_address() {
    let stub = StubSmtp::start().unwrap();
    let site = demo_copy();
    mail_through(site.path(), &stub, Some(POSTAL));
    subscribers(site.path(), &["grace@example.com", "ada@example.com"]).await;
    let (code, out) = newsletter(site.path(), &["send", POST]);
    assert_eq!(code, Some(0), "{out}");
    assert_eq!(out, format!("Sent en/{POST} to 2 subscribers of main.\n"));
    let received = stub.received();
    assert_eq!(received.len(), 2, "{received:?}");
    let message = joined(&received[0]);
    assert!(message.contains("To: grace@example.com"), "{message}");
    assert!(message.contains("A sitemap is a file"), "{message}");
    assert!(message.contains("/blog/unsubscribe?token="), "{message}");
    assert!(message.contains(POSTAL), "{message}");
    let (code, _) = newsletter(site.path(), &["send", POST]);
    assert_eq!(code, Some(1));
    assert_eq!(stub.received().len(), 2, "the post went out twice");
    let (code, out) = newsletter(site.path(), &["numbers", "--json"]);
    assert_eq!(code, Some(0), "{out}");
    let numbers: Value = serde_json::from_str(&out).unwrap();
    let send = &numbers["sends"][0];
    assert_eq!(send["key"], format!("post:en/{POST}"));
    assert_eq!(
        (send["sent"].as_u64(), send["failed"].as_u64()),
        (Some(2), Some(0))
    );
    assert!(!out.contains("grace@example.com"), "numbers name a reader");
}

#[tokio::test]
async fn without_a_postal_address_nothing_goes_out() {
    let stub = StubSmtp::start().unwrap();
    let site = demo_copy();
    mail_through(site.path(), &stub, None);
    subscribers(site.path(), &["grace@example.com"]).await;
    let (code, _) = newsletter(site.path(), &["send", POST]);
    assert_eq!(code, Some(1));
    assert_eq!(stub.received().len(), 0);
}

#[test]
fn embed_prints_a_form_that_posts_to_the_blog_for_a_newsletter_it_has() {
    let site = demo_copy();
    let mut weekly = site.path().to_path_buf();
    weekly.extend(["content", "newsletters", "weekly"]);
    fs::create_dir_all(&weekly).unwrap();
    let (code, form) = newsletter(site.path(), &["embed", "--list", "weekly"]);
    assert_eq!(code, Some(0), "{form}");
    let action = "action=\"https://www.example.com/blog/subscribe\"";
    assert!(form.contains(action), "{form}");
    assert!(form.contains("name=\"list\" value=\"weekly\""), "{form}");
    let (code, _) = newsletter(site.path(), &["embed", "--list", "monthly"]);
    assert_eq!(code, Some(1));
}

#[tokio::test]
async fn a_digest_lists_new_posts_once_and_then_sends_nothing() {
    let stub = StubSmtp::start().unwrap();
    let site = demo_copy();
    mail_through(site.path(), &stub, Some(POSTAL));
    subscribers(site.path(), &["grace@example.com"]).await;
    let today = SystemClock.now().date_naive();
    let mut post = site.path().to_path_buf();
    post.extend(["content", "en", "posts", "fresh-news.md"]);
    let front = format!(
        "---\ntitle: Fresh news on crawling\nslug: fresh-news\ndate: {today}\n\
         author: lena-vogt\n---\n\nSearch engines crawl new pages first.\n"
    );
    fs::write(&post, front).unwrap();
    let (code, out) = newsletter(site.path(), &["digest"]);
    assert_eq!(code, Some(0), "{out}");
    assert_eq!(out, "Sent the new posts to 1 subscribers of main.\n");
    let received = stub.received();
    assert_eq!(received.len(), 1, "{received:?}");
    let message = joined(&received[0]);
    assert!(message.contains("Subject: New on "), "{message}");
    assert!(message.contains("Fresh news on crawling"), "{message}");
    assert!(message.contains("/blog/fresh-news/"), "{message}");
    assert!(
        !message.contains("What a sitemap does"),
        "an old post: {message}"
    );
    let (code, out) = newsletter(site.path(), &["digest"]);
    assert_eq!(code, Some(0), "{out}");
    assert_eq!(
        out,
        "Nothing is new since the last digest, so nothing was sent.\n"
    );
    assert_eq!(stub.received().len(), 1);
}
