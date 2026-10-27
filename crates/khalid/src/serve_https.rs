//! Where `khalid serve` listens: plain HTTP on `--listen`, or HTTPS for
//! `--domain` on ports 443 and 80 with a certificate from Let's Encrypt
//! that Khalid gets and renews (docs/plan.md section 4.7).

use std::io::{self, BufRead, IsTerminal, Write};
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
use std::path::{Path, PathBuf};

use khalid_content::SiteContent;
use khalid_core::{Config, Mode, SystemClock, UserMessage};
use khalid_https::{AgreedToTerms, Directory, Domain, Https, HttpsFolder, Listening};
use khalid_outbound::Outbound;
use khalid_routes::Routes;
use khalid_server::FolderLook;
use tokio::net::TcpListener;

use crate::cli::{Cli, ServeArgs};
use crate::exit::Exit;
use crate::post_files::Assets;
use crate::{https_message, outbound, report, serve, serve_message};

/// The folder in `data/` that keeps the account and the certificate.
const FOLDER: &str = "https";

/// Where `khalid serve` answers.
#[derive(Debug)]
pub(crate) enum Listen {
    /// Plain HTTP on this address.
    Plain(SocketAddr),
    /// HTTPS for a domain.
    Https(Box<HttpsListen>),
}

/// HTTPS for a domain, as the command line asks for it.
#[derive(Debug)]
pub(crate) struct HttpsListen {
    pub(crate) domain: Domain,
    pub(crate) terms: Terms,
    pub(crate) directory: Directory,
    /// The TLS address and the port 80 address, when not the usual ones.
    pub(crate) addresses: (Option<SocketAddr>, Option<SocketAddr>),
    pub(crate) outbound: Outbound,
}

/// How the owner answers Let's Encrypt's terms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Terms {
    /// `--accept-terms` said yes.
    Accepted,
    /// Ask at the terminal, if there is one.
    Ask,
}

/// `khalid serve` with `args`, on the `site` folder with its checked
/// settings, routes and content.
pub(crate) fn serve(
    cli: &Cli,
    site: &Path,
    (config, routes): (&Config, &Routes),
    start: (SiteContent, Assets, FolderLook),
    args: &ServeArgs,
) -> Exit {
    match listen(cli, config.mode(), args) {
        Ok(listen) => {
            let to = (&listen, outbound::client(cli));
            serve::run(site, (config, routes), start, to, cli.output)
        }
        Err(message) => report::failure(cli.output, &message, Exit::Failure),
    }
}

/// Where the command line asks `khalid serve` to answer: HTTPS when
/// `--domain` names one, plain HTTP on `--listen` otherwise.
///
/// A static site has HTTPS from its host, so `--domain` is refused there
/// with the reason.
fn listen(cli: &Cli, mode: Mode, https: &ServeArgs) -> Result<Listen, UserMessage> {
    let Some(domain) = &https.domain else {
        return Ok(Listen::Plain(https.listen));
    };
    if mode == Mode::Static {
        return Err(https_message::static_site());
    }
    let outbound = outbound::client(cli).map_err(|_| serve_message::internal())?;
    let directory = https
        .acme_directory
        .clone()
        .map_or(Directory::LetsEncrypt, Directory::Other);
    let terms = if https.accept_terms {
        Terms::Accepted
    } else {
        Terms::Ask
    };
    Ok(Listen::Https(Box::new(HttpsListen {
        domain: domain.clone(),
        terms,
        directory,
        addresses: (https.https_listen, https.http_listen),
        outbound,
    })))
}

/// Opens where `listen` says, and returns it with the address visitors use.
pub(crate) async fn open(
    site: &Path,
    listen: &Listen,
) -> Result<(Listening, String), (UserMessage, Exit)> {
    let listening = match listen {
        Listen::Plain(address) => Listening::Plain(bind(*address).await?),
        Listen::Https(https) => Listening::Tls(https.start(site).await?),
    };
    let domain = match listen {
        Listen::Plain(_) => None,
        Listen::Https(https) => Some(&https.domain),
    };
    let address = listening
        .address(domain)
        .map_err(|_| (serve_message::internal(), Exit::Internal))?;
    Ok((listening, address))
}

impl HttpsListen {
    /// Asks for the owner's yes when a new account is needed, opens ports
    /// 443 and 80, and starts HTTPS.
    async fn start(&self, site: &Path) -> Result<khalid_https::TlsListener, (UserMessage, Exit)> {
        let https = Https {
            domain: self.domain.clone(),
            folder: HttpsFolder::new(folder(site)),
            directory: self.directory.clone(),
        };
        let failed = |error| (https_message::start(&self.domain, &error), Exit::Failure);
        let agreed = if https.has_account().await.map_err(failed)? {
            None
        } else {
            Some(self.agree().await?)
        };
        let tls = self.addresses.0.unwrap_or_else(|| usual(443));
        let plain = self.addresses.1.unwrap_or_else(|| usual(80));
        let listeners = (bind_https(tls).await?, bind_https(plain).await?);
        https
            .start(&self.outbound, agreed, listeners, SystemClock)
            .await
            .map_err(failed)
    }

    /// The owner's yes to Let's Encrypt's subscriber agreement: from
    /// `--accept-terms`, or asked at the terminal.
    async fn agree(&self) -> Result<AgreedToTerms, (UserMessage, Exit)> {
        if self.terms == Terms::Accepted {
            return Ok(AgreedToTerms);
        }
        if !io::stdin().is_terminal() {
            return Err((https_message::terms_needed(&self.domain), Exit::Failure));
        }
        let domain = self.domain.clone();
        let asked = tokio::task::spawn_blocking(move || {
            ask(&domain, &mut io::stdin().lock(), &mut io::stderr().lock())
        })
        .await;
        match asked {
            Ok(Ok(true)) => Ok(AgreedToTerms),
            Ok(Ok(false) | Err(_)) => Err((https_message::terms_declined(), Exit::Failure)),
            Err(_) => Err((serve_message::internal(), Exit::Internal)),
        }
    }
}

/// Asks the owner to agree to Let's Encrypt's subscriber agreement, once.
/// Only `yes` or `y` is a yes.
pub(crate) fn ask(
    domain: &Domain,
    input: &mut impl BufRead,
    out: &mut impl Write,
) -> io::Result<bool> {
    writeln!(
        out,
        "Khalid gets a free certificate for {domain} from Let's Encrypt, so browsers show the site as secure."
    )?;
    writeln!(
        out,
        "Let's Encrypt asks you to agree to its subscriber agreement first: {}",
        khalid_https::LETS_ENCRYPT_TERMS
    )?;
    write!(out, "Do you agree? Type yes or no. ")?;
    out.flush()?;
    let mut line = String::new();
    input.read_line(&mut line)?;
    Ok(matches!(
        line.trim().to_ascii_lowercase().as_str(),
        "yes" | "y"
    ))
}

/// `data/https/` in the site folder.
fn folder(site: &Path) -> PathBuf {
    let mut folder = site.to_path_buf();
    folder.push("data");
    folder.push(FOLDER);
    folder
}

/// The usual address for `port`: every address of the computer, IPv6 and
/// IPv4 together where the system allows it.
fn usual(port: u16) -> SocketAddr {
    SocketAddr::from((Ipv6Addr::UNSPECIFIED, port))
}

/// Listens on `address` for plain HTTP.
async fn bind(address: SocketAddr) -> Result<TcpListener, (UserMessage, Exit)> {
    TcpListener::bind(address)
        .await
        .map_err(|error| (serve_message::listen(address, &error), Exit::Failure))
}

/// Listens on `address` for HTTPS. When IPv6 is off on this computer, the
/// same port on every IPv4 address is used instead.
async fn bind_https(address: SocketAddr) -> Result<TcpListener, (UserMessage, Exit)> {
    let failed = |error: &io::Error| (https_message::listen(address, error), Exit::Failure);
    let error = match TcpListener::bind(address).await {
        Ok(listener) => return Ok(listener),
        Err(error) => error,
    };
    let taken = matches!(
        error.kind(),
        io::ErrorKind::AddrInUse | io::ErrorKind::PermissionDenied
    );
    if address.ip() != Ipv6Addr::UNSPECIFIED || taken {
        return Err(failed(&error));
    }
    let ipv4 = SocketAddr::from((Ipv4Addr::UNSPECIFIED, address.port()));
    TcpListener::bind(ipv4)
        .await
        .map_err(|error| failed(&error))
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    fn run(typed: &str) -> (bool, String) {
        let domain = Domain::try_new("blog.example.com").unwrap();
        let mut out = Vec::new();
        let agreed = ask(&domain, &mut Cursor::new(typed), &mut out).unwrap();
        (agreed, String::from_utf8(out).unwrap())
    }

    #[test]
    fn the_question_names_the_domain_and_links_the_agreement() {
        let (_, asked) = run("yes\n");
        assert!(asked.contains("blog.example.com"), "{asked}");
        assert!(asked.contains(khalid_https::LETS_ENCRYPT_TERMS), "{asked}");
    }

    #[test]
    fn only_yes_is_a_yes() {
        for typed in ["yes\n", "Y\n", " YES \n"] {
            assert!(run(typed).0, "{typed:?}");
        }
        for typed in ["no\n", "\n", "sure\n", ""] {
            assert!(!run(typed).0, "{typed:?}");
        }
    }
}
