//! The headless starter kits (G55, KH-INS-5): each kit's `khalid.js`,
//! run under Node against `khalid serve` on the headless demo, finds every
//! post and writes, for each one, the head the theme writes into the
//! post's page byte for byte, so schema and meta stay intact.

mod common;

use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use serde_json::Value;
use tempfile::TempDir;

/// The kits, each with its copy of `khalid.js`, from the repository root.
const KITS: [&str; 2] = [
    "starters/astro/src/lib/khalid.js",
    "starters/nextjs/lib/khalid.js",
];

#[cfg(test)]
fn at(base: &Path, rel: &str) -> PathBuf {
    let mut path = base.to_path_buf();
    path.push(rel);
    path
}

#[cfg(test)]
fn root() -> PathBuf {
    at(Path::new(env!("CARGO_MANIFEST_DIR")), "../..")
}

/// A copy of the demo site, in `mode`.
#[cfg(test)]
fn demo(mode: &str) -> TempDir {
    let temp = common::demo_copy();
    let settings = at(temp.path(), "site.toml");
    let text = fs::read_to_string(&settings).unwrap();
    let text = text.replace("mode = \"server\"", &format!("mode = \"{mode}\""));
    fs::write(&settings, text).unwrap();
    temp
}

/// What the kit's `khalid.js` at `kit` writes for every post of the site
/// at `origin`, read with `key`.
#[cfg(test)]
fn run_kit(kit: &Path, origin: &str, key: &str) -> Value {
    let script = at(
        Path::new(env!("CARGO_MANIFEST_DIR")),
        "tests/starter_kits.mjs",
    );
    let output = Command::new("node")
        .arg(script)
        .arg(kit)
        .env("KHALID_API_URL", origin)
        .env("KHALID_CONTENT_KEY", key)
        .output()
        .expect("Node 18 or later must be installed to test the starter kits");
    let said = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{}: {said}", kit.display());
    serde_json::from_slice(&output.stdout).unwrap()
}

/// The status `khalid serve` at `origin` answers `path` with. A share card
/// is drawn behind the first answers, so a 404 is asked again for a while.
#[cfg(test)]
fn status(origin: &str, path: &str) -> u16 {
    let host = origin.trim_start_matches("http://");
    let mut answered = 0;
    for _ in 0..100 {
        let mut stream = TcpStream::connect(host).unwrap();
        let request = format!("GET {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n");
        stream.write_all(request.as_bytes()).unwrap();
        let mut raw = Vec::new();
        stream.read_to_end(&mut raw).unwrap();
        let raw = String::from_utf8_lossy(&raw);
        answered = raw
            .split(' ')
            .nth(1)
            .and_then(|c| c.parse().ok())
            .unwrap_or(0);
        if answered != 404 {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    answered
}

/// Checks that `post`, as `kit` showed it, has the head of its page in the
/// built site `out` byte for byte.
#[cfg(test)]
fn same_head(kit: &str, out: &Path, post: &Value) {
    let route = post["route"].as_str().unwrap();
    let page = fs::read_to_string(at(out, &format!("{route}/index.html")))
        .unwrap_or_else(|_| panic!("{kit}: no built page at {route}"));
    let head = post["head"].as_str().unwrap();
    assert!(
        head.contains("application/ld+json"),
        "{kit} {route}: {head}"
    );
    let differs = head.lines().find(|line| !page.contains(line));
    let theirs = differs.and_then(|line| {
        let start = line.get(..12)?;
        page.lines().find(|own| own.trim_start().starts_with(start))
    });
    assert!(
        page.contains(head),
        "{kit} {route}: {differs:?} is not in the page, which has {theirs:?}"
    );
    assert_eq!(post["listed_head"], post["head"], "{kit} {route}");
    assert!(post["body"].as_str().is_some_and(|body| !body.is_empty()));
}

#[test]
fn each_kit_writes_the_head_the_theme_writes_for_every_demo_post() {
    // A static site has no search box either, so its pages' heads are the
    // ones a headless site's front end should write.
    let built = demo("static");
    let out = tempfile::tempdir().unwrap();
    let made = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .arg("--site")
        .arg(built.path())
        .arg("build")
        .arg("--out")
        .arg(out.path())
        .output()
        .unwrap();
    assert!(made.status.success(), "{made:?}");

    let headless = demo("headless");
    let key = common::new_key(headless.path(), &["--kind", "content", "--name", "Kit"]);
    let server = common::start(headless.path(), &key);
    let origin = format!("http://{}", server.host());
    let settings = format!("/api/content/v1/settings/?key={key}");
    assert!(server.answers(&settings, 200), "{settings}");
    // The key a kit holds reads published posts and nothing more.
    let (refused, _) = server.admin("GET", "/api/admin/v1/posts/", None);
    assert!(matches!(refused, 401 | 403), "{refused}");

    for kit in KITS {
        let shown = run_kit(&at(&root(), kit), &origin, &key);
        let posts = shown["posts"].as_array().unwrap();
        assert!(posts.len() >= 4, "{kit}: {shown}");
        let german = |post: &&Value| post["route"] == "de/was-eine-sitemap-leistet";
        assert!(posts.iter().any(|post| german(&post)), "{kit}: {shown}");
        for post in posts {
            same_head(kit, out.path(), post);
        }
        // The kits' READMEs leave share cards with Khalid: they are there.
        let head = posts[0]["head"].as_str().unwrap();
        let card = head
            .split("og:image\" content=\"https://www.example.com")
            .nth(1);
        let card = card.and_then(|rest| rest.split('"').next()).unwrap();
        assert_eq!(status(&origin, card), 200, "{card}");
        assert_eq!(shown["missing"], Value::Null, "{kit}");
        assert_eq!(shown["settings"]["title"], "Field Notes on SEO", "{kit}");
    }
}

/// The kits' pages that import from `khalid.js`, which no test can build
/// here without installing the frameworks.
const PAGES: [&str; 7] = [
    "starters/astro/src/layouts/Base.astro",
    "starters/astro/src/pages/index.astro",
    "starters/astro/src/pages/[...path].astro",
    "starters/nextjs/app/khalid-head.tsx",
    "starters/nextjs/app/layout.tsx",
    "starters/nextjs/app/page.tsx",
    "starters/nextjs/app/[...path]/page.tsx",
];

#[test]
fn every_name_a_kit_page_imports_from_khalid_js_is_exported() {
    let library = fs::read_to_string(at(&root(), KITS[0])).unwrap();
    for page in PAGES {
        let text = fs::read_to_string(at(&root(), page)).unwrap();
        let imports = text
            .lines()
            .filter(|line| line.starts_with("import {") && line.contains("khalid.js"));
        let mut names = imports
            .filter_map(|line| line.split(['{', '}']).nth(1))
            .flat_map(|names| names.split(','))
            .map(str::trim)
            .peekable();
        assert!(
            names.peek().is_some(),
            "{page} imports nothing from khalid.js"
        );
        for name in names {
            // A type comes from a JSDoc typedef, which the type checker reads.
            if let Some(typedef) = name.strip_prefix("type ") {
                let declared = format!("}}}} {typedef}\n");
                assert!(library.contains(&declared), "{page} imports type {typedef}");
                continue;
            }
            let exported = format!("export function {name}(");
            let async_exported = format!("export async function {name}(");
            assert!(
                library.contains(&exported) || library.contains(&async_exported),
                "{page} imports {name}, which khalid.js does not export"
            );
        }
    }
}

#[test]
fn every_kit_carries_the_same_khalid_js() {
    let first = fs::read(at(&root(), KITS[0])).unwrap();
    for kit in KITS {
        assert_eq!(fs::read(at(&root(), kit)).unwrap(), first, "{kit}");
    }
}
