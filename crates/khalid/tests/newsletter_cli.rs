//! `khalid newsletter label` and `segment`: labels given and taken away
//! from the command line, and segments counted as totals, never as a list
//! of people (G39).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use khalid_core::{Clock, Slug, SystemClock};
use khalid_db::{Database, SubscriberTable};
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
