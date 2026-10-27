//! The APIs on a running `khalid serve`: the Content API answers from the
//! site's own index with a key made by `khalid keys create` and shows a
//! saved edit without a restart, and a post made and published through the
//! Admin API is served, or refused by the linter (KH-API-1, KH-API-2). The
//! `OpenAPI` document it serves is the SDK's, and its routes keep it
//! (KH-API-4).

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

use serde_json::Value;
use tempfile::TempDir;

/// The pause between looks for an edit.
const PAUSE: Duration = Duration::from_millis(100);

/// How many looks, 20 seconds in all, before the test fails.
const TRIES: u32 = 200;

/// A running `khalid serve`, stopped when dropped.
struct Server {
    child: Child,
    host: String,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// A copy of the demo site, so a test can change it.
#[cfg(test)]
fn demo_copy() -> TempDir {
    let mut demo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    demo.push("../../fixtures/demo-site");
    let temp = tempfile::tempdir().unwrap();
    copy_dir(&demo, temp.path());
    temp
}

#[cfg(test)]
fn copy_dir(from: &Path, into: &Path) {
    fs::create_dir_all(into).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let mut target = into.to_path_buf();
        target.push(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).unwrap();
        }
    }
}

/// Runs `khalid --site <site> keys create --json` with `args` and returns
/// the key it showed.
#[cfg(test)]
fn new_key(site: &Path, args: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["--site", site.to_str().unwrap(), "keys", "create", "--json"])
        .args(args)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    json["key"].as_str().unwrap().to_owned()
}

/// Starts `khalid serve` on `site` on a free port and reads the address it
/// prints.
#[cfg(test)]
fn start(site: &Path) -> Server {
    let mut child = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["--site", site.to_str().unwrap(), "serve", "--listen"])
        .args(["127.0.0.1:0", "--json"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut line = String::new();
    let stdout = child.stdout.take().unwrap();
    BufReader::new(stdout).read_line(&mut line).unwrap();
    let json: Value = serde_json::from_str(&line).unwrap_or_else(|_| panic!("{line}"));
    let address = json["address"].as_str().unwrap();
    let host = address.strip_prefix("http://").unwrap();
    let host = host.trim_end_matches("/blog/").to_owned();
    Server { child, host }
}

/// Sends `method` to `path` with `headers` and a JSON `body`, if any, and
/// returns the status and the body.
#[cfg(test)]
fn request(
    server: &Server,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: Option<&Value>,
) -> (u16, String) {
    let (status, _, body) = exchange(server, method, path, headers, body);
    (status, body)
}

/// As [`request`], with the media type of the answer, or `""`.
#[cfg(test)]
fn exchange(
    server: &Server,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: Option<&Value>,
) -> (u16, String, String) {
    let host = &server.host;
    let mut stream = TcpStream::connect(host).unwrap();
    let body = body.map(Value::to_string).unwrap_or_default();
    let mut head = format!(
        "{method} {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\nContent-Length: {}\r\n",
        body.len()
    );
    if !body.is_empty() {
        head.push_str("Content-Type: application/json\r\n");
    }
    for (name, value) in headers {
        head = format!("{head}{name}: {value}\r\n");
    }
    stream
        .write_all(format!("{head}\r\n{body}").as_bytes())
        .unwrap();
    let mut raw = String::new();
    stream.read_to_string(&mut raw).unwrap();
    let (head, body) = raw.split_once("\r\n\r\n").unwrap();
    let media = head
        .lines()
        .filter_map(|line| line.split_once(": "))
        .find(|(name, _)| name.eq_ignore_ascii_case("content-type"))
        .map_or(String::new(), |(_, value)| value.to_owned());
    (
        head.split(' ').nth(1).unwrap().parse().unwrap(),
        media,
        body.to_owned(),
    )
}

/// The English posts, newest first, as the Content API lists them.
#[cfg(test)]
fn english_posts(server: &Server, key: &str) -> Value {
    let path = format!("/api/content/v1/posts/?key={key}&filter=language:en");
    let (status, body) = request(server, "GET", &path, &[], None);
    assert_eq!(status, 200, "{body}");
    serde_json::from_str(&body).unwrap()
}

#[test]
fn the_content_api_lists_the_demo_posts_and_shows_an_edit() {
    let site = demo_copy();
    let key = new_key(site.path(), &["--kind", "content", "--name", "Web page"]);
    let server = start(site.path());

    let listed = english_posts(&server, &key);
    let posts = listed["posts"].as_array().unwrap();
    let slugs: Vec<&str> = posts.iter().filter_map(|p| p["slug"].as_str()).collect();
    assert_eq!(
        slugs,
        [
            "planning-a-topic-cluster",
            "fixing-slow-pages",
            "what-a-sitemap-does",
            "how-to-write-a-title-tag",
            "why-internal-links-matter",
        ]
    );
    assert_eq!(listed["meta"]["pagination"]["total"], 5, "{listed}");
    let html = posts[0]["html"].as_str().unwrap_or_default();
    assert!(html.contains("<p>"), "each body is rendered on request");

    let (status, _) = request(&server, "GET", "/api/content/v1/posts/", &[], None);
    assert_eq!(status, 401, "no key, no posts");

    let mut file = site.path().to_path_buf();
    file.push("content/en/posts/2026-09-28-what-a-sitemap-does.md");
    let post = fs::read_to_string(&file).unwrap();
    fs::write(
        &file,
        post.replace("title: What a sitemap does", "title: What a sitemap is for"),
    )
    .unwrap();
    let shown = (0..TRIES).any(|_| {
        thread::sleep(PAUSE);
        english_posts(&server, &key)
            .to_string()
            .contains("What a sitemap is for")
    });
    assert!(shown, "the edit never reached the Content API");
}

/// The resources `docs/standards/api.md` names, and `navigation`, as the
/// contract test in `khalid-api` tries them.
const RESOURCES: [&str; 18] = [
    "posts",
    "pages",
    "authors",
    "tags",
    "categories",
    "collections",
    "glossary",
    "case-studies",
    "changelog",
    "media",
    "redirects",
    "snippets",
    "forms",
    "subscribers",
    "webhooks",
    "settings",
    "site",
    "navigation",
];

/// The methods a route could answer.
const METHODS: [&str; 5] = ["GET", "POST", "PUT", "PATCH", "DELETE"];

/// Whether an answer came from an API handler: those answer JSON or a
/// problem, and every other address gets the site's HTML pages.
#[cfg(test)]
fn from_a_handler(media: &str) -> bool {
    media.starts_with("application/json") || media.starts_with("application/problem+json")
}

/// Brief B-API-4's done line: `khalid serve` answers `/api/openapi.json`
/// without a key, it is the document the SDK is generated from, and the
/// contract holds on the running server: every documented route reaches a
/// handler, and every API address that answers is documented.
#[test]
fn khalid_serve_serves_the_openapi_document_its_routes_keep() {
    let site = demo_copy();
    let content_key = new_key(site.path(), &["--kind", "content", "--name", "Web page"]);
    let admin_key = new_key(site.path(), &["--name", "Contract", "--scope", "admin:all"]);
    let server = start(site.path());

    let (status, media, body) = exchange(&server, "GET", "/api/openapi.json", &[], None);
    assert_eq!(
        (status, media.as_str()),
        (200, "application/json"),
        "{body}"
    );
    let document: Value = serde_json::from_str(&body).unwrap();
    let mut sdk = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    sdk.push("../../sdk/typescript/openapi.json");
    let committed: Value = serde_json::from_str(&fs::read_to_string(sdk).unwrap()).unwrap();
    assert!(
        document == committed,
        "the served document is not sdk/typescript/openapi.json"
    );

    let content = format!("Khalid-Content {content_key}");
    let admin = format!("Khalid {admin_key}");
    let reached = |method: &str, path: &str| {
        let key = if path.starts_with("/api/admin/") {
            &admin
        } else {
            &content
        };
        let headers = [("Authorization", key.as_str())];
        let empty = serde_json::json!({});
        let (_, media, _) = exchange(&server, method, path, &headers, Some(&empty));
        from_a_handler(&media)
    };
    let concrete = |path: &str| {
        path.replace("{id}", "contract-id")
            .replace("{slug}", "contract-slug")
    };
    let paths = document["paths"].as_object().unwrap();
    assert!(paths.len() > 30, "{}", paths.len());
    for (path, item) in paths {
        for method in METHODS {
            let documented = item.get(method.to_lowercase()).is_some();
            let answered = reached(method, &concrete(path));
            assert_eq!(answered, documented, "{method} {path}");
        }
    }
    for base in ["/api/content/v1", "/api/admin/v1"] {
        for resource in RESOURCES {
            for suffix in ["/", "/contract-id/", "/slug/contract-slug/"] {
                let address = format!("{base}/{resource}{suffix}");
                for method in METHODS {
                    if reached(method, &address) {
                        let listed = paths.iter().any(|(path, item)| {
                            concrete(path) == address && item.get(method.to_lowercase()).is_some()
                        });
                        assert!(listed, "{method} {address} answers but is not documented");
                    }
                }
            }
        }
    }
}

/// Brief B-API-1's done line: the demo authors, and the site's other lists,
/// come back through `khalid serve` from the index.
#[test]
fn the_content_api_serves_the_demo_authors_and_the_site_lists() {
    let site = demo_copy();
    let key = new_key(site.path(), &["--kind", "content", "--name", "Web page"]);
    let server = start(site.path());
    let get = |resource: &str| {
        let joint = if resource.contains('?') { '&' } else { '?' };
        let path = format!("/api/content/v1/{resource}{joint}key={key}");
        let (status, body) = request(&server, "GET", &path, &[], None);
        assert_eq!(status, 200, "{resource}: {body}");
        serde_json::from_str::<Value>(&body).unwrap()
    };
    let ids = |json: &Value, field: &str| -> Vec<String> {
        let entries = json[field].as_array().unwrap();
        entries.iter().map(|e| e["id"].to_string()).collect()
    };

    let authors = get("authors/?filter=language:en");
    assert_eq!(
        ids(&authors, "authors"),
        ["\"en.asif\"", "\"en.lena-vogt\""]
    );
    let asif = &authors["authors"][0];
    assert_eq!(asif["name"], "Asif");
    assert_eq!(asif["role"], "Founder of SerpDad");
    assert_eq!(asif["url"], "https://www.example.com/blog/authors/asif/");
    assert_eq!(asif["same_as"][0], "https://serpdad.com/");
    assert!(
        asif["bio"].as_str().unwrap().starts_with("Asif writes"),
        "{asif}"
    );
    let german = get("authors/de.asif/");
    assert_eq!(
        german["authors"][0]["url"],
        "https://www.example.com/blog/de/authors/asif/"
    );

    let categories = get("categories/?filter=slug:on-page-seo");
    assert_eq!(
        ids(&categories, "categories"),
        ["\"de.on-page-seo\"", "\"en.on-page-seo\""]
    );
    assert_eq!(categories["categories"][1]["name"], "On-page SEO");

    let posts = get("collections/en.posts/");
    assert_eq!(posts["collections"][0]["title"], "Field notes on SEO");
    let english = get("collections/?filter=language:en&limit=all");
    let slugs: Vec<&str> = english["collections"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["slug"].as_str().unwrap())
        .collect();
    assert_eq!(
        slugs,
        ["posts", "pages", "glossary", "case-studies", "changelog"]
    );

    let settings = get("settings/");
    assert_eq!(
        settings["settings"][0],
        serde_json::json!({
            "title": "Field Notes on SEO",
            "url": "https://www.example.com/blog/",
            "default_language": "en",
            "languages": ["en", "de"],
        })
    );
    let menus = get("navigation/");
    assert_eq!(
        ids(&menus, "navigation"),
        [
            "\"en.primary\"",
            "\"en.footer\"",
            "\"de.primary\"",
            "\"de.footer\""
        ]
    );
    assert_eq!(get("tags/")["meta"]["pagination"]["total"], 0);
}

/// The API wiring task's done line: on the demo site, list the posts
/// through the Content API, create and publish a post through the Admin
/// API with a key made by `khalid keys create`, and see its page served.
#[test]
fn a_post_created_and_published_through_the_admin_api_is_served() {
    let site = demo_copy();
    let content_key = new_key(site.path(), &["--kind", "content", "--name", "Web page"]);
    let admin_key = new_key(
        site.path(),
        &["--name", "SerpDad", "--scope", "posts:write"],
    );
    let server = start(site.path());
    let before = english_posts(&server, &content_key);
    assert_eq!(before["meta"]["pagination"]["total"], 5, "{before}");

    let authorization = format!("Khalid {admin_key}");
    let auth = [("Authorization", authorization.as_str())];
    let markdown = "Anchor text is the visible words of a link.\n\n\
        Read [why internal links matter](/blog/why-internal-links-matter/) first.";
    let new = serde_json::json!({"posts": [{
        "title": "Choosing anchor text",
        "author": "asif",
        "markdown": markdown,
    }]});
    let (status, body) = request(&server, "POST", "/api/admin/v1/posts/", &auth, Some(&new));
    assert_eq!(status, 201, "{body}");
    let made: Value = serde_json::from_str(&body).unwrap();
    let draft = &made["posts"][0];
    assert_eq!(draft["id"], "en.choosing-anchor-text", "{made}");
    assert_eq!(draft["status"], "draft", "{made}");
    let (status, _) = request(&server, "GET", "/blog/choosing-anchor-text/", &[], None);
    assert_eq!(status, 404, "a draft is never served");
    let drafts = "/api/admin/v1/posts/?filter=status:draft";
    let (status, body) = request(&server, "GET", drafts, &auth, None);
    assert_eq!(status, 200, "{body}");
    let listed: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(
        listed["posts"][0]["id"], "en.choosing-anchor-text",
        "{listed}"
    );
    assert_eq!(listed["posts"][0]["gate"], "passes", "{listed}");
    assert_eq!(listed["meta"]["pagination"]["total"], 1, "{listed}");

    let path = "/api/admin/v1/posts/en.choosing-anchor-text/";
    let excerpt = "Pick the words a reader clicks with care.";
    let publish = serde_json::json!({"posts": [{
        "status": "published",
        "settings": {
            "excerpt": excerpt,
            "featured": true,
            "co_authors": ["lena-vogt"],
            "review_state": "ready",
        },
        "updated_at": draft["updated_at"],
    }]});
    let (status, body) = request(&server, "PUT", path, &auth, Some(&publish));
    assert_eq!(status, 200, "{body}");
    assert!(body.contains("\"status\":\"published\""), "{body}");
    assert!(body.contains("\"featured\":true"), "{body}");
    assert!(body.contains("\"co_authors\":[\"lena-vogt\"]"), "{body}");
    assert!(body.contains("\"review_state\":\"ready\""), "{body}");

    let page_shown = (0..TRIES).any(|_| {
        thread::sleep(PAUSE);
        let (status, page) = request(&server, "GET", "/blog/choosing-anchor-text/", &[], None);
        status == 200 && page.contains("Choosing anchor text") && page.contains(excerpt)
    });
    assert!(page_shown, "the published post was never served");
    let after = english_posts(&server, &content_key);
    assert_eq!(after["posts"][0]["slug"], "choosing-anchor-text", "{after}");
    assert_eq!(after["meta"]["pagination"]["total"], 6, "{after}");
}

#[test]
fn a_weak_post_is_refused_with_its_findings_and_never_served() {
    let site = demo_copy();
    let admin_key = new_key(
        site.path(),
        &["--name", "SerpDad", "--scope", "posts:write"],
    );
    let server = start(site.path());
    let authorization = format!("Khalid {admin_key}");
    let auth = [("Authorization", authorization.as_str())];
    let new = serde_json::json!({"posts": [{"title": "A weak post", "markdown": "Short."}]});
    let (status, body) = request(&server, "POST", "/api/admin/v1/posts/", &auth, Some(&new));
    assert_eq!(status, 201, "{body}");
    let made: Value = serde_json::from_str(&body).unwrap();
    let publish = serde_json::json!({"posts": [{
        "status": "published",
        "updated_at": made["posts"][0]["updated_at"],
    }]});
    let path = "/api/admin/v1/posts/en.a-weak-post/";
    let (status, body) = request(&server, "PUT", path, &auth, Some(&publish));
    assert_eq!(status, 422, "{body}");
    assert!(body.contains("no-author"), "{body}");
    let (status, _) = request(&server, "GET", "/blog/a-weak-post/", &[], None);
    assert_eq!(status, 404);
    let (status, _) = request(&server, "POST", "/api/admin/v1/posts/", &[], Some(&new));
    assert_eq!(status, 401, "no key, no post");
}

#[test]
fn khalid_serve_answers_staff_sign_in_and_keeps_staff_tokens_working() {
    let site = demo_copy();
    let staff_token = new_key(
        site.path(),
        &["--kind", "staff", "--name", "Admin app", "--role", "owner"],
    );
    let server = start(site.path());
    let sign_in =
        serde_json::json!({"email": "nobody@example.com", "password": "correct horse battery"});
    let (status, media, body) = exchange(
        &server,
        "POST",
        "/api/admin/v1/session/",
        &[],
        Some(&sign_in),
    );
    assert_eq!(
        (status, media.as_str()),
        (401, "application/problem+json"),
        "{body}"
    );
    let (status, _) = request(&server, "GET", "/api/admin/v1/session/", &[], None);
    assert_eq!(status, 401, "no session cookie");
    let authorization = format!("Khalid {staff_token}");
    let auth = [("Authorization", authorization.as_str())];
    let (status, body) = request(&server, "GET", "/api/admin/v1/staff/", &auth, None);
    assert_eq!(status, 200, "an owner's staff token manages staff: {body}");
    let invite = serde_json::json!({"email": "mia@example.com", "name": "Mia", "role": "writer"});
    let (status, body) = request(
        &server,
        "POST",
        "/api/admin/v1/staff/",
        &auth,
        Some(&invite),
    );
    assert_eq!(status, 201, "{body}");
    let (_, body) = request(&server, "GET", "/api/admin/v1/staff/", &auth, None);
    assert!(
        body.contains("mia@example.com"),
        "kept in the site's database: {body}"
    );
}

/// Runs `khalid --site <site> staff owner --json` with `email`.
#[cfg(test)]
fn owner(site: &Path, email: &str) -> (Option<i32>, Value) {
    let output = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["--site", site.to_str().unwrap(), "--json", "staff", "owner"])
        .args(["--email", email, "--name", "Asif"])
        .output()
        .unwrap();
    let text = [output.stdout, output.stderr].concat();
    (output.status.code(), serde_json::from_slice(&text).unwrap())
}

#[test]
fn the_first_owner_signs_in_with_the_link_khalid_staff_owner_shows_once() {
    let site = demo_copy();
    let (code, made) = owner(site.path(), "asif@example.com");
    assert_eq!(code, Some(0), "{made}");
    let link = made["link"].as_str().unwrap();
    let (admin, token) = link.split_once("/sign-in/link#token=").unwrap();
    assert!(admin.ends_with("/admin"), "{link}");
    let (code, second) = owner(site.path(), "someone@example.com");
    assert_eq!(code, Some(1), "a second owner is refused: {second}");
    let server = start(site.path());
    let redeem = serde_json::json!({ "token": token });
    let path = "/api/admin/v1/session/link/redeem/";
    let (status, body) = request(&server, "POST", path, &[], Some(&redeem));
    assert_eq!(status, 200, "{body}");
    assert!(body.contains("\"role\":\"owner\""), "{body}");
    let (status, _) = request(&server, "POST", path, &[], Some(&redeem));
    assert_eq!(status, 401, "the link works once");
}
