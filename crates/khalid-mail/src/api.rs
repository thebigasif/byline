//! Sending through a mail service's HTTP API: Resend or Postmark.

use khalid_core::{AbsoluteUrl, ParseUrlError, Secret};
use khalid_newsletter::{BoxFuture, Mail, MailError, Mailer};
use khalid_outbound::Outbound;
use serde::Serialize;

use crate::{SendError, Sender};

/// A mail service with an HTTP API.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Service {
    /// Resend, which takes a bearer API key.
    Resend,
    /// Postmark, which takes a server token and sends on a message stream.
    Postmark {
        /// The stream, such as `outbound`.
        message_stream: String,
    },
}

impl Service {
    /// Where the service takes one email.
    fn endpoint(&self) -> &'static str {
        match self {
            Self::Resend => "https://api.resend.com/emails",
            Self::Postmark { .. } => "https://api.postmarkapp.com/email",
        }
    }
}

/// Sends each email with one POST to a mail service's API, through the
/// shared client, signed with the site's key. Any 2xx answer means the
/// service took it.
#[derive(Debug)]
pub struct ApiMailer {
    service: Service,
    key: Secret<String>,
    sender: Sender,
    outbound: Outbound,
    endpoint: AbsoluteUrl,
}

impl ApiMailer {
    /// Sends through `service` as `sender`, with `key`, through `outbound`.
    ///
    /// # Errors
    ///
    /// [`ParseUrlError`] when the service's built-in address is not valid,
    /// which this crate's tests rule out.
    pub fn new(
        service: Service,
        key: Secret<String>,
        sender: Sender,
        outbound: Outbound,
    ) -> Result<Self, ParseUrlError> {
        let endpoint = AbsoluteUrl::parse(service.endpoint())?;
        Ok(Self {
            service,
            key,
            sender,
            outbound,
            endpoint,
        })
    }

    /// The same mailer sending to `endpoint` instead, such as a stand-in
    /// server in a test.
    #[cfg(any(test, feature = "test-util"))]
    #[must_use]
    pub fn at(self, endpoint: AbsoluteUrl) -> Self {
        Self { endpoint, ..self }
    }

    /// The service's JSON for `mail`.
    fn body(&self, mail: &Mail) -> Result<Vec<u8>, SendError> {
        let from = self.sender.mailbox();
        let to = mail.to.as_str();
        let written = match &self.service {
            Service::Resend => serde_json::to_vec(&ResendEmail {
                from: &from,
                to: [to],
                subject: &mail.subject,
                text: &mail.text,
            }),
            Service::Postmark { message_stream } => serde_json::to_vec(&PostmarkEmail {
                from: &from,
                to,
                subject: &mail.subject,
                text_body: &mail.text,
                message_stream,
            }),
        };
        written.map_err(SendError::Encode)
    }

    /// Hands `mail` to the service.
    async fn post(&self, mail: &Mail) -> Result<(), SendError> {
        let body = self.body(mail)?;
        let (name, value) = match &self.service {
            Service::Resend => ("Authorization", format!("Bearer {}", self.key.expose())),
            Service::Postmark { .. } => ("X-Postmark-Server-Token", self.key.expose().clone()),
        };
        let headers = [
            ("Content-Type", "application/json"),
            ("Accept", "application/json"),
            (name, value.as_str()),
        ];
        let answer = self.outbound.post(&self.endpoint, &headers, body).await;
        match answer.map(|fetched| fetched.status()) {
            Ok(status) if (200..300).contains(&status) => Ok(()),
            Ok(status) => Err(SendError::Refused { status }),
            Err(source) => Err(SendError::Unreachable(source)),
        }
    }
}

impl Mailer for ApiMailer {
    fn send<'a>(&'a self, mail: &'a Mail) -> BoxFuture<'a, Result<(), MailError>> {
        Box::pin(async move { self.post(mail).await.map_err(MailError::new) })
    }
}

/// One email as Resend's `POST /emails` takes it.
#[derive(Serialize)]
struct ResendEmail<'a> {
    from: &'a str,
    to: [&'a str; 1],
    subject: &'a str,
    text: &'a str,
}

/// One email as Postmark's `POST /email` takes it.
#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct PostmarkEmail<'a> {
    from: &'a str,
    to: &'a str,
    subject: &'a str,
    text_body: &'a str,
    message_stream: &'a str,
}
