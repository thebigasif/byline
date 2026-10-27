//! An email, and the transport that sends it.

use std::error::Error;
use std::fmt;

use crate::{BoxFuture, EmailAddress};

/// One plain-text email to one address. The sender comes from the
/// transport's settings, so a page or a template can never forge it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Mail {
    /// Who gets it.
    pub to: EmailAddress,
    /// The subject line.
    pub subject: String,
    /// The text, with the links written out in full.
    pub text: String,
}

impl Mail {
    /// An email to `to` with `subject` and `text`.
    pub fn new(to: EmailAddress, subject: String, text: String) -> Self {
        Self { to, subject, text }
    }
}

/// What sends mail: SMTP or a provider's API in the binary (task 5.6), or
/// [`MemoryMailer`] in tests. The text holds working links, so a transport
/// never logs it (RULES.md BL-RUN-5).
pub trait Mailer: fmt::Debug + Send + Sync {
    /// Hands `mail` to the transport.
    ///
    /// # Errors
    ///
    /// [`MailError`] when the transport refused it or could not be reached.
    fn send<'a>(&'a self, mail: &'a Mail) -> BoxFuture<'a, Result<(), MailError>>;
}

/// The transport did not take the email. It keeps the transport's own error
/// as its cause.
#[derive(Debug, thiserror::Error)]
#[error("the email could not be sent")]
pub struct MailError {
    #[source]
    source: Box<dyn Error + Send + Sync>,
}

impl MailError {
    /// Wraps the transport's own `source`.
    pub fn new(source: impl Error + Send + Sync + 'static) -> Self {
        Self {
            source: Box::new(source),
        }
    }
}

#[cfg(any(test, feature = "test-util"))]
pub use memory::MemoryMailer;

#[cfg(any(test, feature = "test-util"))]
mod memory {
    use std::sync::{Arc, Mutex, PoisonError};

    use super::{BoxFuture, Mail, MailError, Mailer};

    /// A fake transport for tests: it keeps every email it is given, or
    /// refuses them all.
    #[derive(Debug, Clone, Default)]
    pub struct MemoryMailer {
        sent: Arc<Mutex<Vec<Mail>>>,
        refusing: bool,
    }

    impl MemoryMailer {
        /// A transport that refuses every email, like an SMTP server that
        /// is down.
        pub fn refusing() -> Self {
            Self {
                refusing: true,
                ..Self::default()
            }
        }

        /// Every email taken so far, oldest first.
        pub fn sent(&self) -> Vec<Mail> {
            self.sent
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clone()
        }
    }

    impl Mailer for MemoryMailer {
        fn send<'a>(&'a self, mail: &'a Mail) -> BoxFuture<'a, Result<(), MailError>> {
            Box::pin(async move {
                if self.refusing {
                    return Err(MailError::new(std::io::Error::other("refused for a test")));
                }
                self.sent
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .push(mail.clone());
                Ok(())
            })
        }
    }
}
