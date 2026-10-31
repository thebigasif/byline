//! The comment form's address on a running site, answered as plain HTML.

use std::fmt;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::extract::rejection::FormRejection;
use axum::extract::{ConnectInfo, DefaultBodyLimit, Form, State};
use axum::http::{HeaderMap, HeaderValue, Response, StatusCode, header};
use axum::routing::post;
use khalid_core::Locale;

use crate::{
    CommentForm, CommentStore, Comments, CommentsError, NewComment, PostFile, RateLimitError,
    RateLimits, Reading, Refusal, RefusalPages,
};

/// The largest form body read. A whole comment at its longest, written in
/// four-byte letters and percent-encoded, fits with room to spare.
const BODY_LIMIT: usize = 64 * 1024;

/// Where on the post's page the note that a comment was received sits.
const SENT_FRAGMENT: &str = "#comment-sent";

/// The site's post pages, as the comment form names them.
pub trait PostPages: fmt::Debug + Send + Sync {
    /// The post whose page is at `page`, such as `/blog/why-links-matter/`,
    /// or `None` when no post is there.
    fn post_at(&self, page: &str) -> Option<PostPage>;
}

/// A post page that takes comments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostPage {
    /// The post's file.
    pub file: PostFile,
    /// The page's address as the site serves it.
    pub href: String,
    /// The language the page is written in.
    pub language: Locale,
}

/// What the comment form's route needs.
#[derive(Debug)]
pub struct FormRoute<S> {
    /// The comments it sends to.
    pub comments: Arc<Comments<S>>,
    /// The limits per network address.
    pub limits: RateLimits,
    /// Where it finds the post a form was sent from.
    pub pages: Arc<dyn PostPages>,
    /// The pages that say why a comment was not sent.
    pub refusals: RefusalPages,
    /// The path the form posts to, such as `/blog/comments/`.
    pub action: String,
    /// The path of the site's stylesheet, which refusal pages link.
    pub stylesheet: String,
    /// The language of a refusal when the form names no post.
    pub language: Locale,
}

/// The route `POST <action>` that receives the comment form. A kept
/// comment, and bait too, answer 303 back to the post at
/// `#comment-sent`; anything else answers a page saying why, in the
/// post's language. It needs the reader's address, so serve it with
/// `ConnectInfo<SocketAddr>`.
pub fn form_router<S: CommentStore + 'static>(route: FormRoute<S>) -> Router {
    let path = route.action.clone();
    Router::new()
        .route(&path, post(receive::<S>))
        .layer(DefaultBodyLimit::max(BODY_LIMIT))
        .with_state(Arc::new(route))
}

async fn receive<S: CommentStore + 'static>(
    State(route): State<Arc<FormRoute<S>>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    form: Result<Form<CommentForm>, FormRejection>,
) -> Response<Body> {
    let form = form.map(|Form(form)| form).unwrap_or_default();
    let Some(post) = route.pages.post_at(&form.page) else {
        let page = route
            .refusals
            .page(&route.language, &Refusal::NoPost, &form, links(&route));
        return html(StatusCode::NOT_FOUND, page);
    };
    let refuse = |status, refusal: Refusal| {
        let page = route
            .refusals
            .page(&post.language, &refusal, &form, links(&route));
        html(status, page)
    };
    let comment = match form.clone().read() {
        Reading::Comment(comment) => comment,
        Reading::Bait => return back_to(&post),
        Reading::Problems(problems) => {
            return refuse(StatusCode::UNPROCESSABLE_ENTITY, Refusal::Fields(problems));
        }
    };
    match route.limits.check(reader(peer.ip(), &headers)) {
        Ok(()) => {}
        Err(RateLimitError::Limited { retry_after }) => {
            let minutes = retry_after.as_secs().div_ceil(60).max(1);
            let mut response = refuse(StatusCode::TOO_MANY_REQUESTS, Refusal::TooMany { minutes });
            response
                .headers_mut()
                .insert(header::RETRY_AFTER, HeaderValue::from(minutes * 60));
            return response;
        }
        Err(error) => {
            tracing::warn!(%error, "a comment could not be checked against the limits");
            return refuse(StatusCode::SERVICE_UNAVAILABLE, Refusal::Unavailable);
        }
    }
    let new = NewComment {
        post: post.file.clone(),
        parent: comment.parent,
        author: comment.author,
        text: comment.text,
    };
    match route.comments.submit(new).await {
        Ok(_) => back_to(&post),
        Err(CommentsError::ParentNotFound { .. }) => {
            refuse(StatusCode::UNPROCESSABLE_ENTITY, Refusal::ParentGone)
        }
        Err(error) => {
            tracing::warn!(%error, "a comment could not be stored");
            refuse(StatusCode::SERVICE_UNAVAILABLE, Refusal::Unavailable)
        }
    }
}

fn links<S>(route: &FormRoute<S>) -> (&str, &str) {
    (&route.action, &route.stylesheet)
}

/// The reader's address: the peer's, or, when the peer is a proxy on this
/// machine or the local network, the last address its `X-Forwarded-For`
/// names, which is the one the proxy saw. Other routes with
/// [`RateLimits`](crate::RateLimits) count readers by it too, as
/// `reader_address`.
#[must_use]
pub fn reader(peer: IpAddr, headers: &HeaderMap) -> IpAddr {
    let local = match peer.to_canonical() {
        IpAddr::V4(v4) => v4.is_loopback() || v4.is_private(),
        IpAddr::V6(v6) => v6.is_loopback() || v6.is_unique_local(),
    };
    if !local {
        return peer;
    }
    let forwarded = headers.get_all("x-forwarded-for").into_iter().next_back();
    forwarded
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.rsplit(',').next())
        .and_then(|last| last.trim().parse().ok())
        .unwrap_or(peer)
}

/// 303 to where the post's page says the comment was received.
fn back_to(post: &PostPage) -> Response<Body> {
    let mut response = Response::new(Body::empty());
    *response.status_mut() = StatusCode::SEE_OTHER;
    let location = format!("{}{SENT_FRAGMENT}", post.href);
    if let Ok(location) = HeaderValue::from_str(&location) {
        response.headers_mut().insert(header::LOCATION, location);
    }
    response
}

/// An HTML page with `status`, never cached, since it answers one form.
fn html(status: StatusCode, page: String) -> Response<Body> {
    let mut response = Response::new(Body::from(page));
    *response.status_mut() = status;
    let headers = response.headers_mut();
    let html = HeaderValue::from_static("text/html; charset=utf-8");
    headers.insert(header::CONTENT_TYPE, html);
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_local_proxy_may_name_the_reader() {
        let mut headers = HeaderMap::new();
        let forwarded = HeaderValue::from_static("198.51.100.1, 203.0.113.7");
        headers.insert("x-forwarded-for", forwarded);
        let ip = |text: &str| text.parse::<IpAddr>().unwrap();
        assert_eq!(reader(ip("127.0.0.1"), &headers), ip("203.0.113.7"));
        assert_eq!(reader(ip("10.0.0.2"), &headers), ip("203.0.113.7"));
        assert_eq!(reader(ip("192.0.2.9"), &headers), ip("192.0.2.9"));
        assert_eq!(reader(ip("::1"), &HeaderMap::new()), ip("::1"));
    }
}
