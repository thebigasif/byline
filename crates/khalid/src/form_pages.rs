//! The form block on a running site (G4, KH-GRO-1): each form posts under
//! the mount path, the binary notes which page holds it, and the route of
//! `khalid-forms` keeps what readers send in the site database.

use std::collections::BTreeMap;
use std::sync::{Arc, PoisonError, RwLock};

use axum::Router;
use axum::response::Response;
use khalid_comments::RateLimits;
use khalid_core::{Config, SystemClock};
use khalid_db::{Database, FormTable, SiteId};
use khalid_forms::{FormId, FormRoute, SiteForms, form_router};
use khalid_render::{BodyHtml, PageBody};
use khalid_server::SecurityHeaders;

use crate::build_pages::LanguagePages;
use crate::site_pages::SitePages;

/// How the form block opens a form, up to its id (`khalid-content`).
const BLOCK_FORM: &str = "<form class=\"form\" method=\"post\" action=\"/forms/";

/// The same opening without the address, which the mount path goes after.
const BEFORE_ACTION: &str = "<form class=\"form\" method=\"post\" action=\"";

/// The page each form is on, by form id, as the renders found them. A
/// saved post's render adds its forms, and a full render adds every
/// page's. Cloning shares them.
#[derive(Debug, Clone, Default)]
pub(crate) struct LiveForms(Arc<RwLock<BTreeMap<FormId, String>>>);

impl SiteForms for LiveForms {
    fn page_of(&self, form: &FormId) -> Option<String> {
        let pages = self.0.read().unwrap_or_else(PoisonError::into_inner);
        pages.get(form).cloned()
    }
}

/// Points each form block in `languages` at `<mount>forms/<id>` and notes
/// its page in `forms`. A site that takes no forms, `forms` being `None`,
/// is left as it is.
pub(crate) fn fill(config: &Config, forms: Option<&LiveForms>, languages: &mut [LanguagePages]) {
    let Some(forms) = forms else {
        return;
    };
    let mut pages = forms.0.write().unwrap_or_else(PoisonError::into_inner);
    for language in languages {
        for page in &mut language.pages {
            let body = match &mut page.body {
                PageBody::Post(post) => &mut post.body,
                PageBody::Page(shown) => &mut shown.body,
                _ => continue,
            };
            let Some((html, ids)) = mounted(body.as_str(), config.mount()) else {
                continue;
            };
            *body = BodyHtml::from_sanitised(html);
            for id in ids {
                pages.insert(id, page.meta.path.clone());
            }
        }
    }
}

/// `html` with each form block posting under `mount`, and the forms'
/// ids, or `None` when it has no form block. Most bodies have none, so
/// those are never copied.
fn mounted(html: &str, mount: &str) -> Option<(String, Vec<FormId>)> {
    if !html.contains(BLOCK_FORM) {
        return None;
    }
    let mut parts = html.split(BLOCK_FORM);
    let mut out = String::with_capacity(html.len() + 32);
    out.push_str(parts.next()?);
    let mut ids = Vec::new();
    for rest in parts {
        out.extend([BEFORE_ACTION, mount, "forms/", rest]);
        if let Some(id) = rest
            .split_once('"')
            .and_then(|(id, _)| FormId::try_new(id).ok())
        {
            ids.push(id);
        }
    }
    (!ids.is_empty()).then_some((out, ids))
}

/// The form route of a running site, keeping answers in site `site` of
/// `db`, each answer carrying `security`. A site that takes no forms, or
/// has no theme to style the answer pages, gets no route.
pub(crate) fn router(
    config: &Config,
    (db, site): (&Database, SiteId),
    source: &SitePages,
    security: &SecurityHeaders,
) -> Router {
    let security = security.clone();
    let (Some(forms), Some(stylesheet)) = (source.forms(), source.stylesheet()) else {
        return Router::new();
    };
    let route = FormRoute {
        store: Arc::new(FormTable::new(db.clone(), site)),
        forms: Arc::new(forms),
        limits: RateLimits::new(SystemClock),
        clock: Box::new(SystemClock),
        mount: config.mount().to_owned(),
        stylesheet,
    };
    let secured = move |mut response: Response| {
        security.apply(response.headers_mut());
        async move { response }
    };
    form_router(route).layer(axum::middleware::map_response(secured))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_form_block_posts_under_the_mount() {
        let html = "<p>Hi</p>\n<form class=\"form\" method=\"post\" action=\"/forms/contact\">\n\
                    <form class=\"form\" method=\"post\" action=\"/forms/quiz-2\">\n";
        let (out, ids) = mounted(html, "/blog/").unwrap();
        assert!(out.contains("action=\"/blog/forms/contact\""), "{out}");
        assert!(out.contains("action=\"/blog/forms/quiz-2\""), "{out}");
        assert!(out.starts_with("<p>Hi</p>"), "{out}");
        let ids: Vec<&str> = ids.iter().map(FormId::as_str).collect();
        assert_eq!(ids, ["contact", "quiz-2"]);
        assert_eq!(mounted("<p>No form</p>", "/blog/"), None);
        let (root, _) = mounted(html, "/").unwrap();
        assert_eq!(root, html);
    }
}
