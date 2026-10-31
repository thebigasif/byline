//! How a site sends email, from the `[mail]` table of `site.toml`.

use serde::Deserialize;

use crate::Environment;

/// Who mail comes from and the one way it leaves. Leave the table out and
/// the site sends no mail, so sign-ups stay closed. Pick exactly one way:
///
/// ```toml
/// [mail]
/// from = "news@example.com"
/// name = "Field Notes"
/// forms_to = "hello@example.com"
/// postal_address = "Field Notes Ltd, 1 Park Row, Leeds LS1 5AB, UK"
///
/// [mail.smtp]
/// host = "smtp.example.com"
/// username = "news@example.com"
/// ```
///
/// or an empty `[mail.resend]`, or `[mail.postmark]`. A password, API key
/// or token never goes in the file: it comes from the environment, see
/// [`crate::Environment::mail_secret`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MailSettings {
    /// The address mail comes from, such as `news@example.com`.
    pub from: String,
    /// The name shown beside it, or `None` for the address alone.
    pub name: Option<String>,
    /// The address each form block's answers are emailed to, or `None` to
    /// send them to [`MailSettings::from`].
    pub forms_to: Option<String>,
    /// The postal address every newsletter email ends with, as anti-spam
    /// laws ask. Khalid refuses to send a newsletter until it is set.
    pub postal_address: Option<String>,
    /// How the mail leaves.
    pub transport: MailTransport,
}

impl MailSettings {
    /// The address form answers are emailed to: `forms_to`, else `from`.
    #[must_use]
    pub fn form_answers_to(&self) -> &str {
        self.forms_to.as_deref().unwrap_or(&self.from)
    }
}

/// The one way a site's mail leaves.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum MailTransport {
    /// An SMTP server, with the password in `KHALID_SMTP_PASSWORD`.
    Smtp(SmtpSettings),
    /// Resend's API, with the key in `KHALID_RESEND_API_KEY`.
    Resend,
    /// Postmark's API, with the server token in `KHALID_POSTMARK_TOKEN`.
    Postmark(PostmarkSettings),
}

/// An SMTP server that takes the site's mail.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct SmtpSettings {
    /// The server's host name, such as `smtp.example.com`.
    pub host: String,
    /// The port, 587 unless set.
    #[serde(default = "submission_port")]
    pub port: u16,
    /// How the connection is encrypted, STARTTLS unless set.
    #[serde(default)]
    pub security: SmtpSecurity,
    /// The user to sign in as, or `None` for a server that takes mail
    /// without signing in.
    pub username: Option<String>,
    /// Whether the host is on the site's own network, such as a relay on
    /// the same machine. Only then may it resolve to a private or local
    /// address, and only then may mail go unencrypted.
    #[serde(default)]
    pub private_host: bool,
}

/// The port mail programs submit mail on (RFC 6409).
fn submission_port() -> u16 {
    587
}

/// How the SMTP connection is encrypted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum SmtpSecurity {
    /// A plain connection upgraded to TLS before anything is sent. A server
    /// that cannot upgrade is refused.
    #[default]
    StartTls,
    /// TLS from the first byte, usually on port 465.
    Tls,
    /// No encryption, written `none`, for a relay on the site's own
    /// network only.
    #[serde(rename = "none")]
    Plain,
}

/// Postmark's settings.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct PostmarkSettings {
    /// The message stream mail is sent on, `outbound` unless set.
    #[serde(default = "outbound_stream")]
    pub message_stream: String,
}

fn outbound_stream() -> String {
    "outbound".to_owned()
}

/// Resend needs nothing beyond its key, so its table is empty.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct ResendTable {}

/// `[mail]` as written, before the checks.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MailTable {
    from: String,
    name: Option<String>,
    forms_to: Option<String>,
    postal_address: Option<String>,
    smtp: Option<SmtpSettings>,
    resend: Option<ResendTable>,
    postmark: Option<PostmarkSettings>,
}

/// The longest sender's name, in characters.
const LONGEST_NAME: usize = 100;

/// The longest postal address, in characters: room for a company, a street,
/// a town, a postcode and a country.
const LONGEST_POSTAL_ADDRESS: usize = 300;

impl MailTable {
    /// The checked settings, with the secret they need in `env`, or the
    /// key a mistake is under and what is wrong.
    pub(crate) fn check(
        self,
        env: &Environment,
    ) -> Result<MailSettings, (&'static str, &'static str)> {
        if !is_address(&self.from) {
            return Err((
                "from",
                "from is the address mail comes from, such as news@example.com",
            ));
        }
        if let Some(name) = &self.name
            && (name.chars().any(char::is_control) || name.chars().count() > LONGEST_NAME)
        {
            return Err((
                "name",
                "the sender's name is one line of up to 100 characters",
            ));
        }
        if self.forms_to.as_deref().is_some_and(|to| !is_address(to)) {
            return Err((
                "forms_to",
                "forms_to is the address form answers are emailed to, such as hello@example.com",
            ));
        }
        if let Some(postal) = &self.postal_address
            && (postal.trim().is_empty()
                || postal.chars().any(char::is_control)
                || postal.chars().count() > LONGEST_POSTAL_ADDRESS)
        {
            return Err((
                "postal_address",
                "the postal address is one line of up to 300 characters, such as Field Notes Ltd, 1 Park Row, Leeds LS1 5AB, UK",
            ));
        }
        let transport = match (self.smtp, self.resend, self.postmark) {
            (Some(smtp), None, None) => MailTransport::Smtp(smtp),
            (None, Some(_), None) => MailTransport::Resend,
            (None, None, Some(postmark)) => MailTransport::Postmark(postmark),
            _ => {
                return Err((
                    "from",
                    "mail is sent one way, so add exactly one of [mail.smtp], [mail.resend] or [mail.postmark]",
                ));
            }
        };
        if let MailTransport::Smtp(smtp) = &transport
            && smtp.security == SmtpSecurity::Plain
            && !smtp.private_host
        {
            return Err((
                "security",
                "mail without encryption may only go to a host on your own network, so set private_host = true or use starttls",
            ));
        }
        if let Some(missing) = missing_secret(&transport, env) {
            return Err(missing);
        }
        Ok(MailSettings {
            from: self.from,
            name: self.name,
            forms_to: self.forms_to,
            postal_address: self.postal_address,
            transport,
        })
    }
}

/// The key to point at and the message when `transport` needs a secret
/// `env` does not hold.
fn missing_secret(
    transport: &MailTransport,
    env: &Environment,
) -> Option<(&'static str, &'static str)> {
    if env.mail_secret(transport).is_some() {
        return None;
    }
    match transport {
        MailTransport::Smtp(smtp) if smtp.username.is_none() => None,
        MailTransport::Smtp(_) => Some((
            "username",
            "this way of sending needs KHALID_SMTP_PASSWORD set in the environment",
        )),
        MailTransport::Resend => Some((
            "from",
            "this way of sending needs KHALID_RESEND_API_KEY set in the environment",
        )),
        MailTransport::Postmark(_) => Some((
            "from",
            "this way of sending needs KHALID_POSTMARK_TOKEN set in the environment",
        )),
    }
}

/// Whether `text` is a plain address: one `@`, no spaces or control
/// characters, and a domain with a dot that does not start or end it.
pub(crate) fn is_address(text: &str) -> bool {
    let Some((local, domain)) = text.split_once('@') else {
        return false;
    };
    let plain = |c: char| c.is_ascii_graphic() && !"<>\"(),:;[\\]".contains(c);
    !local.is_empty()
        && local.chars().all(plain)
        && domain.chars().all(plain)
        && !domain.contains('@')
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
}
