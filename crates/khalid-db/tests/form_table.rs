//! Form answers on SQLite come back as they went in, in order, and each
//! site sees only its own (KH-GRO-1).

use std::path::PathBuf;

use chrono::{DateTime, TimeDelta, TimeZone, Utc};
use khalid_core::Slug;
use khalid_db::{Database, FormTable};
use khalid_forms::{FormId, FormStore, Submission};

#[cfg(test)]
fn at(minutes: i64) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 31, 9, 0, 0).unwrap() + TimeDelta::minutes(minutes)
}

/// A migrated database with the form answers of the sites `demo` and
/// `other`.
#[cfg(test)]
async fn opened() -> (tempfile::TempDir, FormTable, FormTable) {
    let dir = tempfile::tempdir().unwrap();
    let mut path = PathBuf::from(dir.path());
    path.push("site.db");
    let db = Database::open(&path).await.unwrap();
    db.migrate().await.unwrap();
    let demo = db.site(&Slug::try_new("demo").unwrap()).await.unwrap();
    let other = db.site(&Slug::try_new("other").unwrap()).await.unwrap();
    let tables = (FormTable::new(db.clone(), demo), FormTable::new(db, other));
    (dir, tables.0, tables.1)
}

#[cfg(test)]
fn sent(form: &str, fields: &[(&str, &str)]) -> Submission {
    Submission {
        form: FormId::try_new(form).unwrap(),
        page: "/blog/write-to-us/".to_owned(),
        fields: fields
            .iter()
            .map(|&(name, value)| (name.to_owned(), value.to_owned()))
            .collect(),
    }
}

#[tokio::test]
async fn answers_come_back_in_the_order_they_were_sent() {
    let (_dir, forms, _) = opened().await;
    let first = sent(
        "contact",
        &[
            ("name", "Lena Vogt"),
            ("message", "Quotes \" and 'ticks', a {brace} and ü."),
            ("reply_to", "lena@example.com"),
        ],
    );
    let second = sent("contact", &[("message", "A second note.")]);
    let survey = sent("survey", &[("answer", "Yes")]);
    forms.insert(&second, at(5)).await.unwrap();
    forms.insert(&first, at(0)).await.unwrap();
    forms.insert(&survey, at(1)).await.unwrap();

    let contact = FormId::try_new("contact").unwrap();
    let kept = forms.of_form(&contact).await.unwrap();
    assert_eq!(kept, [(first, at(0)), (second, at(5))]);
}

#[tokio::test]
async fn a_site_never_sees_another_site_s_answers() {
    let (_dir, demo, other) = opened().await;
    demo.insert(&sent("contact", &[("name", "Ada")]), at(0))
        .await
        .unwrap();
    let contact = FormId::try_new("contact").unwrap();
    assert_eq!(other.of_form(&contact).await.unwrap(), []);
    assert_eq!(demo.of_form(&contact).await.unwrap().len(), 1);
}
