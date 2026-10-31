//! Each form answer a running site keeps, sent on (G40, KH-GRO-1): by email
//! to the address `[mail]` names for form answers, and as `form.submitted`
//! to every webhook that asked for it. Both run on the announcing tasks, so
//! the reader's thank-you page never waits, and both are under way within
//! the 5 seconds plan section 10 allows.

use std::sync::Arc;

use chrono::{DateTime, SecondsFormat, Utc};
use khalid_core::{Config, UserMessage};
use khalid_forms::{FormDelivery, Submission};
use khalid_newsletter::{EmailAddress, Mail, Mailer};
use khalid_webhooks::Event;
use serde::ser::{Serialize, Serializer};

use crate::announce::SiteAnnounce;
use crate::serve_message;

/// Where a running site sends each kept answer.
#[derive(Debug)]
pub(crate) struct SiteDelivery {
    /// The way mail leaves and the owner's address, or `None` when the
    /// site sends no mail.
    mail: Option<(Arc<dyn Mailer>, EmailAddress)>,
    /// The site's address without its last `/`, which page paths follow.
    base: String,
    announce: SiteAnnounce,
}

impl SiteDelivery {
    /// The delivery for the site with `config`, emailing through `mailer`
    /// when it has one and firing webhooks through `announce`.
    ///
    /// # Errors
    ///
    /// Khalid's own message when the address `[mail]` gives is not one,
    /// which the settings check at start rules out.
    pub(crate) fn new(
        config: &Config,
        mailer: Option<Arc<dyn Mailer>>,
        announce: SiteAnnounce,
    ) -> Result<Self, UserMessage> {
        let mail = match (mailer, config.mail()) {
            (Some(mailer), Some(settings)) => {
                let to = EmailAddress::parse(settings.form_answers_to())
                    .map_err(|_| serve_message::internal())?;
                Some((mailer, to))
            }
            _ => None,
        };
        let base = config.address().as_str().trim_end_matches('/').to_owned();
        Ok(Self {
            mail,
            base,
            announce,
        })
    }
}

impl FormDelivery for SiteDelivery {
    fn kept(&self, submission: &Submission, at: DateTime<Utc>) {
        let page = format!("{}{}", self.base, submission.page);
        if let Some((mailer, to)) = &self.mail {
            let mail = email(to.clone(), submission, &page, at);
            let (mailer, form) = (Arc::clone(mailer), submission.form.clone());
            self.announce.send(async move {
                if let Err(error) = mailer.send(&mail).await {
                    tracing::warn!(%error, %form, "a form's answers could not be emailed");
                }
            });
        }
        let submission = Answer {
            form: submission.form.to_string(),
            page,
            fields: Fields(submission.fields.clone()),
        };
        self.announce
            .fire(Event::FormSubmitted, Submitted { submission });
    }
}

/// The email telling the owner, `to`, what `submission` on `page` said.
fn email(to: EmailAddress, submission: &Submission, page: &str, at: DateTime<Utc>) -> Mail {
    let form = &submission.form;
    let subject = format!("New answer to the {form} form");
    let when = at.to_rfc3339_opts(SecondsFormat::Secs, true);
    let fields: String = submission
        .fields
        .iter()
        .flat_map(|(name, value)| [name.as_str(), ": ", value.as_str(), "\n"])
        .collect();
    let text = format!(
        "Someone filled in the {form} form on {page} at {when}.\n\n{fields}\n\
         The answer is also kept in your site's database.\n"
    );
    Mail::new(to, subject, text)
}

/// The resource a `form.submitted` delivery carries.
#[derive(Debug, serde::Serialize)]
struct Submitted {
    submission: Answer,
}

/// One answer in API shape: the form, the full address of its page, and
/// each field by name, in the order the form sent them.
#[derive(Debug, serde::Serialize)]
struct Answer {
    form: String,
    page: String,
    fields: Fields,
}

/// A form's fields, written as one JSON object in the order they came.
#[derive(Debug)]
struct Fields(Vec<(String, String)>);

impl Serialize for Fields {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_map(self.0.iter().map(|(name, value)| (name, value)))
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use khalid_forms::FormId;

    use super::*;

    fn submission() -> Submission {
        Submission {
            form: FormId::try_new("contact").unwrap(),
            page: "/blog/write-to-us/".to_owned(),
            fields: vec![
                ("name".to_owned(), "Grace Hopper".to_owned()),
                ("reply_to".to_owned(), "grace@example.com".to_owned()),
            ],
        }
    }

    #[test]
    fn the_email_names_the_form_its_page_and_every_field_in_order() {
        let to = EmailAddress::parse("hello@example.com").unwrap();
        let at = Utc.with_ymd_and_hms(2026, 10, 31, 9, 0, 0).unwrap();
        let page = "https://www.example.com/blog/write-to-us/";
        let mail = email(to, &submission(), page, at);
        assert_eq!(mail.subject, "New answer to the contact form");
        assert_eq!(
            mail.text,
            "Someone filled in the contact form on https://www.example.com/blog/write-to-us/ \
             at 2026-10-31T09:00:00Z.\n\nname: Grace Hopper\nreply_to: grace@example.com\n\n\
             The answer is also kept in your site's database.\n"
        );
    }

    #[test]
    fn the_webhook_body_keeps_the_fields_in_their_order() {
        let answer = Answer {
            form: "contact".to_owned(),
            page: "https://www.example.com/blog/write-to-us/".to_owned(),
            fields: Fields(submission().fields),
        };
        let json = serde_json::to_string(&Submitted { submission: answer }).unwrap();
        assert_eq!(
            json,
            "{\"submission\":{\"form\":\"contact\",\
             \"page\":\"https://www.example.com/blog/write-to-us/\",\
             \"fields\":{\"name\":\"Grace Hopper\",\"reply_to\":\"grace@example.com\"}}}"
        );
    }
}
