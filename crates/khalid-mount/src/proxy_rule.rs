//! The exact proxy rule for each stack.

use khalid_core::AbsoluteUrl;

use crate::{MountError, MountPath, Stack};

/// Prints the rule that makes `stack` send the folder `at` to Khalid at `blog`.
///
/// The path is passed on unchanged, because Khalid knows its mount path and
/// writes every link with it. The bare folder is passed on or redirected.
///
/// # Errors
///
/// Returns [`MountError::UpstreamPath`] when `blog` has a path other than
/// `/`, a query or a fragment, since the rule adds the mount path itself.
pub fn proxy_rule(stack: Stack, at: &MountPath, blog: &AbsoluteUrl) -> Result<String, MountError> {
    let (m, text) = (at.as_str(), blog.as_str());
    let u = text.strip_suffix('/').unwrap_or(text);
    let after_scheme = u.split_once("://").map_or(u, |(_, rest)| rest);
    if after_scheme.contains(['/', '?', '#']) {
        return Err(MountError::UpstreamPath);
    }
    Ok(match stack {
        Stack::Nginx => format!(
            "location = {m} {{\n    return 301 {m}/;\n}}\nlocation {m}/ {{\n    \
             proxy_pass {u};\n    proxy_ssl_server_name on;\n    \
             proxy_set_header X-Forwarded-Host $host;\n    \
             proxy_set_header X-Forwarded-Proto $scheme;\n    \
             proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;\n}}\n"
        ),
        Stack::Caddy => format!(
            "redir {m} {m}/ 301\nhandle {m}/* {{\n    reverse_proxy {u} {{\n        \
             header_up Host {{upstream_hostport}}\n    }}\n}}\n"
        ),
        Stack::Apache => format!(
            "SSLProxyEngine on\nRedirectMatch 301 ^{m}$ {m}/\n\
             ProxyPass {m}/ {u}{m}/\nProxyPassReverse {m}/ {u}{m}/\n"
        ),
        Stack::Traefik => format!(
            "http:\n  routers:\n    khalid-blog:\n      \
             rule: \"Path(`{m}`) || PathPrefix(`{m}/`)\"\n      service: khalid-blog\n      \
             priority: 1000\n  services:\n    khalid-blog:\n      loadBalancer:\n        \
             passHostHeader: false\n        servers:\n          - url: \"{u}\"\n"
        ),
        Stack::Vercel => format!(
            "{{\n  \"rewrites\": [\n    \
             {{ \"source\": \"{m}\", \"destination\": \"{u}{m}\" }},\n    \
             {{ \"source\": \"{m}/:path*\", \"destination\": \"{u}{m}/:path*\" }}\n  ]\n}}\n"
        ),
        Stack::Nextjs => format!(
            "module.exports = {{\n  async rewrites() {{\n    return [\n      \
             {{ source: \"{m}\", destination: \"{u}{m}\" }},\n      \
             {{ source: \"{m}/:path*\", destination: \"{u}{m}/:path*\" }},\n    \
             ];\n  }},\n}};\n"
        ),
        Stack::Netlify => format!("{m}  {u}{m}  200\n{m}/*  {u}{m}/:splat  200\n"),
        Stack::CloudflareWorker | Stack::Webflow | Stack::Shopify => format!(
            "export default {{\n  async fetch(request) {{\n    \
             const url = new URL(request.url);\n    \
             if (url.pathname !== \"{m}\" && !url.pathname.startsWith(\"{m}/\")) {{\n      \
             return fetch(request);\n    }}\n    \
             const forwarded = new Request(`{u}${{url.pathname}}${{url.search}}`, request);\n    \
             forwarded.headers.set(\"X-Forwarded-Host\", url.host);\n    \
             return fetch(forwarded);\n  }},\n}};\n"
        ),
    })
}
