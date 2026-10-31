//! Many sites behind one listener: each request goes to the site whose
//! domain it names in `Host` (plan 4.4, ADR "many sites in one process").
//! A site puts its router in when it is ready and takes it out when it
//! stops, so a site added while the process runs answers without a restart.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, PoisonError, RwLock};

use axum::Router;
use axum::extract::Request;
use axum::http::header::{CONTENT_TYPE, HOST};
use axum::http::{HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use khalid_auth::SiteDirectory;
use khalid_core::{Config, DomainName, UserMessage};
use khalid_https::Listening;
use khalid_server::{ServerError, Stopped};
use tower::ServiceExt;

use crate::cli::Output;
use crate::exit::Exit;
use crate::serve::{self, GRACE};
use crate::serve_https::{self, Listen};
use crate::site_folders::NoPanel;

/// The answer for a host no site here has.
const UNKNOWN: &str = "No site on this server answers to this name. \
Check the address, or add the site with khalid sites add.\n";

/// The routers of the sites in this process, by domain. Cloning shares it.
#[derive(Debug, Clone, Default)]
pub(crate) struct Hosts(Arc<RwLock<BTreeMap<String, Router>>>);

impl Hosts {
    /// Answers requests for `domain` with `app` from now on.
    pub(crate) fn insert(&self, domain: &DomainName, app: Router) {
        let mut hosts = self.0.write().unwrap_or_else(PoisonError::into_inner);
        hosts.insert(domain.as_str().to_owned(), app);
    }

    /// Stops answering for `domain`.
    pub(crate) fn remove(&self, domain: &DomainName) {
        let mut hosts = self.0.write().unwrap_or_else(PoisonError::into_inner);
        hosts.remove(domain.as_str());
    }

    /// The one router the listener serves: each request goes to its
    /// site's router, and one for no site gets 404.
    pub(crate) fn router(&self) -> Router {
        let hosts = self.clone();
        Router::new().fallback(move |request: Request| {
            let hosts = hosts.clone();
            async move { hosts.answer(request).await }
        })
    }

    async fn answer(&self, request: Request) -> Response {
        let app = host(&request).and_then(|host| {
            let hosts = self.0.read().unwrap_or_else(PoisonError::into_inner);
            hosts.get(&host).cloned()
        });
        if let Some(app) = app {
            let Ok(response) = app.oneshot(request).await;
            return response;
        }
        let mut response = (StatusCode::NOT_FOUND, UNKNOWN).into_response();
        let plain = HeaderValue::from_static("text/plain; charset=utf-8");
        response.headers_mut().insert(CONTENT_TYPE, plain);
        response
    }
}

/// A site served as one of many: the shared routers, its domain, and the
/// directory its super-admin routes use.
#[derive(Clone)]
pub(crate) struct Among {
    pub(crate) hosts: Hosts,
    pub(crate) domain: DomainName,
    pub(crate) directory: Arc<dyn SiteDirectory>,
}

/// The directory a site's super-admin routes use: the one it has `among`
/// many, or none when it is served alone.
pub(crate) fn panel(among: Option<&Among>) -> Arc<dyn SiteDirectory> {
    match among {
        Some(among) => Arc::clone(&among.directory),
        None => Arc::new(NoPanel),
    }
}

/// Where one site answers: its own listener, or its place among many.
pub(crate) enum Door {
    Alone(Listening),
    Among(Hosts, DomainName),
}

impl Door {
    /// Opens the site's own listener on `listen` and says where it answers,
    /// or, among many, takes the site's place, which the sites folder's
    /// listener has said already.
    pub(crate) async fn open(
        (site, listen, among): (&Path, &Listen, Option<&Among>),
        (output, config): (Output, &Config),
    ) -> Result<Self, (UserMessage, Exit)> {
        if let Some(among) = among {
            return Ok(Self::Among(among.hosts.clone(), among.domain.clone()));
        }
        let (listener, address) = serve_https::open(site, listen).await?;
        serve::announce(output, config, &format!("{address}{}", config.mount()));
        Ok(Self::Alone(listener))
    }

    /// Answers with `app` until `stopping` completes.
    ///
    /// # Errors
    ///
    /// Returns the error when the site's own listener fails.
    pub(crate) async fn answer(
        self,
        app: Router,
        stopping: impl Future<Output = ()> + Send + 'static,
    ) -> Result<Stopped, ServerError> {
        match self {
            Self::Alone(listener) => khalid_server::serve(listener, app, stopping, GRACE).await,
            Self::Among(hosts, domain) => {
                hosts.insert(&domain, app);
                stopping.await;
                hosts.remove(&domain);
                Ok(Stopped::Finished)
            }
        }
    }
}

/// The domain a request names: its `Host` header, or the address's own
/// host for HTTP/2, in lower case without the port or a final dot.
fn host(request: &Request) -> Option<String> {
    let named = match request.headers().get(HOST) {
        Some(value) => value.to_str().ok()?,
        None => request.uri().host()?,
    };
    let name = match named.rsplit_once(':') {
        Some((name, port)) if port.bytes().all(|b| b.is_ascii_digit()) => name,
        _ => named,
    };
    Some(name.trim_end_matches('.').to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use axum::body::{Body, to_bytes};
    use axum::routing::get;
    use khalid_auth::site_domain;

    use super::*;

    async fn ask(hosts: &Hosts, host: &str) -> (StatusCode, String) {
        let request = Request::get("/").header(HOST, host).body(Body::empty());
        let response = hosts.router().oneshot(request.unwrap()).await.unwrap();
        let status = response.status();
        let body = to_bytes(response.into_body(), 1024).await.unwrap();
        (status, String::from_utf8(body.to_vec()).unwrap())
    }

    #[tokio::test]
    async fn each_host_reaches_its_own_site_and_no_other() {
        let hosts = Hosts::default();
        let (home, news) = (
            site_domain("example.com").unwrap(),
            site_domain("news.example.com").unwrap(),
        );
        hosts.insert(&home, Router::new().route("/", get(|| async { "home" })));
        hosts.insert(&news, Router::new().route("/", get(|| async { "news" })));

        for (host, want) in [
            ("example.com", "home"),
            ("EXAMPLE.com:8080", "home"),
            ("news.example.com.", "news"),
        ] {
            assert_eq!(
                ask(&hosts, host).await,
                (StatusCode::OK, want.to_owned()),
                "{host}"
            );
        }
        for host in [
            "other.example.com",
            "example.com.evil.test",
            "127.0.0.1:8080",
        ] {
            let (status, body) = ask(&hosts, host).await;
            assert_eq!(status, StatusCode::NOT_FOUND, "{host}");
            assert!(body.contains("No site on this server"), "{host}: {body}");
        }

        hosts.remove(&news);
        assert_eq!(
            ask(&hosts, "news.example.com").await.0,
            StatusCode::NOT_FOUND
        );
    }
}
