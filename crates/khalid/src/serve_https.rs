//! Where `khalid serve` listens, and the address visitors use. Plain HTTP
//! on `--listen` here; `--domain` adds HTTPS with a certificate from Let's
//! Encrypt (docs/plan.md section 4.7).

use std::net::SocketAddr;
use std::path::Path;

use khalid_content::SiteContent;
use khalid_core::{Config, UserMessage};
use khalid_https::Listening;
use khalid_routes::Routes;
use khalid_server::FolderLook;
use tokio::net::TcpListener;

use crate::cli::{Cli, ServeArgs};
use crate::exit::Exit;
use crate::post_files::Assets;
use crate::{serve, serve_message};

/// Where `khalid serve` answers.
#[derive(Debug)]
pub(crate) enum Listen {
    /// Plain HTTP on this address.
    Plain(SocketAddr),
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
    let listen = Listen::Plain(args.listen);
    serve::run(site, (config, routes), start, &listen, cli.output)
}

/// Opens where `listen` says, and returns it with the address visitors use.
pub(crate) async fn open(listen: &Listen) -> Result<(Listening, String), (UserMessage, Exit)> {
    let Listen::Plain(address) = listen;
    let tcp = TcpListener::bind(address)
        .await
        .map_err(|error| (serve_message::listen(*address, &error), Exit::Failure))?;
    let listening = Listening::Plain(tcp);
    let address = listening
        .address(None)
        .map_err(|_| (serve_message::internal(), Exit::Internal))?;
    Ok((listening, address))
}
