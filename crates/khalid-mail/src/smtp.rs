//! Sending through an SMTP server with `lettre`.

use std::io;
use std::net::IpAddr;
use std::time::Duration;

use khalid_core::{Secret, SmtpSecurity, SmtpSettings};
use khalid_newsletter::{BoxFuture, Mail, MailError, Mailer};
use khalid_outbound::Resolver;
use lettre::message::header::ContentType;
use lettre::message::{Mailbox, Message};
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::client::{Tls, TlsParameters};
use lettre::transport::smtp::extension::ClientId;
use lettre::{Address, AsyncSmtpTransport, AsyncTransport, Tokio1Executor};

use crate::{SendError, Sender};

/// The longest one email may take, the shared HTTP client's limit too.
const TIMEOUT: Duration = Duration::from_secs(10);

/// Sends each email over one new connection to an SMTP server. The host is
/// looked up through the private-network guard unless `site.toml` marks it
/// `private_host`, then reached by its address, while TLS still checks the
/// certificate against the host's name.
#[derive(Debug)]
pub struct SmtpMailer {
    settings: SmtpSettings,
    password: Option<Secret<String>>,
    sender: Sender,
    resolver: Resolver,
}

impl SmtpMailer {
    /// Sends through the server `settings` name as `sender`, signing in
    /// with the settings' user and `password` when both are there, and
    /// looking a public host up through `resolver`.
    #[must_use]
    pub fn new(
        settings: SmtpSettings,
        password: Option<Secret<String>>,
        sender: Sender,
        resolver: Resolver,
    ) -> Self {
        Self {
            settings,
            password,
            sender,
            resolver,
        }
    }

    /// The address to connect to.
    async fn address(&self) -> Result<IpAddr, SendError> {
        let host = self.settings.host.as_str();
        let found = if self.settings.private_host {
            let mut found = tokio::net::lookup_host((host, self.settings.port))
                .await
                .map_err(SendError::Lookup)?;
            found.next().map(|address| address.ip())
        } else {
            let found = self.resolver.lookup(host).await;
            found.map_err(SendError::Resolve)?.first().copied()
        };
        found.ok_or_else(|| SendError::Lookup(io::ErrorKind::NotFound.into()))
    }

    /// The connection to the server at `address`.
    fn transport(&self, address: IpAddr) -> Result<AsyncSmtpTransport<Tokio1Executor>, SendError> {
        let tls = match self.settings.security {
            SmtpSecurity::Plain => Tls::None,
            security => {
                let parameters = TlsParameters::new(self.settings.host.clone())
                    .map_err(|error| SendError::Smtp(Box::new(error)))?;
                if security == SmtpSecurity::Tls {
                    Tls::Wrapper(parameters)
                } else {
                    Tls::Required(parameters)
                }
            }
        };
        let domain = self
            .sender
            .address()
            .rsplit_once('@')
            .map_or("localhost", |(_, domain)| domain);
        let mut builder =
            AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(address.to_string())
                .port(self.settings.port)
                .tls(tls)
                .timeout(Some(TIMEOUT))
                .hello_name(ClientId::Domain(domain.to_owned()));
        if let (Some(user), Some(password)) = (&self.settings.username, &self.password) {
            builder =
                builder.credentials(Credentials::new(user.clone(), password.expose().clone()));
        }
        Ok(builder.build())
    }

    /// `mail` as an SMTP message, plain text in UTF-8.
    fn message(&self, mail: &Mail) -> Result<Message, SendError> {
        let from: Address = self
            .sender
            .address()
            .parse()
            .map_err(|error| SendError::Address(Box::new(error)))?;
        let to: Address = mail
            .to
            .as_str()
            .parse()
            .map_err(|error| SendError::Address(Box::new(error)))?;
        let from = Mailbox::new(self.sender.name().map(str::to_owned), from);
        Message::builder()
            .from(from)
            .to(Mailbox::new(None, to))
            .subject(mail.subject.clone())
            .header(ContentType::TEXT_PLAIN)
            .body(mail.text.clone())
            .map_err(|error| SendError::Message(Box::new(error)))
    }

    /// Hands `mail` to the server.
    async fn deliver(&self, mail: &Mail) -> Result<(), SendError> {
        let message = self.message(mail)?;
        let address = self.address().await?;
        let transport = self.transport(address)?;
        transport
            .send(message)
            .await
            .map_err(|error| SendError::Smtp(Box::new(error)))?;
        Ok(())
    }
}

impl Mailer for SmtpMailer {
    fn send<'a>(&'a self, mail: &'a Mail) -> BoxFuture<'a, Result<(), MailError>> {
        Box::pin(async move { self.deliver(mail).await.map_err(MailError::new) })
    }
}
