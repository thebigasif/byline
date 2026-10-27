//! `khalid theme sync`: the main site's look, read through a local `wiremock`
//! stand-in and written to `theme/tokens.toml` (task 3.8, RULES.md BL-CLI-1
//! to BL-CLI-3, BL-TST-3). Nothing is fetched from the internet.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;
use wiremock::matchers::path;
use wiremock::{Mock, MockServer, ResponseTemplate};

/// A home page that links one stylesheet on its own site, with a logo and
/// an icon.
const HOME: &str = r#"<!doctype html><html><head><title>Northwind Bakery</title>
<link rel="icon" href="/favicon.ico">
<link rel="stylesheet" href="/style.css"></head><body>
<header><img src="/logo.svg" alt="Northwind"></header><h1>Bread</h1></body></html>"#;

/// A warm theme whose pale amber link is too faint to read on the page.
const STYLE: &str = "body { background: #fbf8f3; color: #2b2118; }\na { color: #f0c040; }\n";

/// The same site after a redesign: darker text and a blue link.
const RESTYLED: &str = "body { background: #fbf8f3; color: #1b1b1f; }\na { color: #2255aa; }\n";

/// The host name the binary resolves to the stand-in, through the test hook.
const HOST: &str = "bakery.test";

/// The demo site's settings in a fresh folder.
#[cfg(test)]
fn site() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let settings = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/demo-site/site.toml"
    );
    fs::copy(settings, within(dir.path(), "site.toml")).unwrap();
    fs::create_dir(within(dir.path(), "content")).unwrap();
    dir
}

/// `name` inside `dir`.
#[cfg(test)]
fn within(dir: &Path, name: &str) -> PathBuf {
    let mut path = dir.to_path_buf();
    path.push(name);
    path
}

/// Serves `home` at `/` and, when given, `style` at `/style.css`. Returns the
/// server and the home address under [`HOST`].
#[cfg(test)]
async fn stand_in(home: &str, style: Option<&str>) -> (MockServer, String) {
    let server = MockServer::start().await;
    Mock::given(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_string(home))
        .mount(&server)
        .await;
    if let Some(style) = style {
        Mock::given(path("/style.css"))
            .respond_with(ResponseTemplate::new(200).set_body_string(style))
            .mount(&server)
            .await;
    }
    let home = format!("http://{HOST}:{}/", server.address().port());
    (server, home)
}

/// How the binary resolves host names in a test.
#[derive(Clone, Copy)]
enum Names {
    /// Through the system, with the private-network guard as released.
    Real,
    /// [`HOST`] answers on loopback, through the test hook.
    StandIn,
}

/// Runs the binary on `site` with `args`, off the async thread so the
/// stand-in keeps serving.
#[cfg(test)]
async fn khalid(site: &Path, args: &[&str], names: Names) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_khalid"));
    command.arg("--site").arg(site).args(args);
    if let Names::StandIn = names {
        command.arg("--test-hosts").arg(format!("{HOST}=127.0.0.1"));
    }
    tokio::task::spawn_blocking(move || command.output().unwrap())
        .await
        .unwrap()
}

#[cfg(test)]
fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[cfg(test)]
fn json(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn sync_writes_the_tokens_says_where_and_explains_each_contrast_fix() {
    let site = site();
    let (_server, home) = stand_in(HOME, Some(STYLE)).await;

    let output = khalid(site.path(), &["theme", "sync", &home], Names::StandIn).await;

    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let said = text(&output.stdout);
    let file = within(site.path(), "theme/tokens.toml");
    assert!(
        said.contains(&format!("into {}.", file.display())),
        "{said}"
    );
    assert!(
        said.contains("The colour of links, #f0c040, was too faint"),
        "{said}"
    );
    let tokens = fs::read_to_string(&file).unwrap();
    assert!(tokens.contains("background = \"#fbf8f3\""), "{tokens}");
    assert!(!tokens.contains("#f0c040"), "{tokens}");
    let brand = format!("[brand]\nlogo = \"{home}logo.svg\"\nicon = \"{home}favicon.ico\"\n");
    assert!(tokens.ends_with(&brand), "{tokens}");
    let preview = within(site.path(), "data/theme-preview/index.html");
    let open = format!("Open {} in a browser to see", preview.display());
    assert!(said.contains(&open), "{said}");
    let page = fs::read_to_string(&preview).unwrap();
    assert!(page.contains("How Northwind Bakery will look"), "{page}");
    let css = fs::read_to_string(within(site.path(), "data/theme-preview/site.css")).unwrap();
    assert!(css.contains("--color-background: #fbf8f3;"), "{css}");
}

#[tokio::test(flavor = "multi_thread")]
async fn sync_with_json_names_the_file_and_each_fix() {
    let site = site();
    let (_server, home) = stand_in(HOME, Some(STYLE)).await;

    let output = khalid(
        site.path(),
        &["--json", "theme", "sync", &home],
        Names::StandIn,
    )
    .await;

    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stdout));
    let body = json(&output);
    let file = within(site.path(), "theme/tokens.toml");
    assert_eq!(body["tokens"], file.display().to_string());
    let preview = within(site.path(), "data/theme-preview/index.html");
    assert_eq!(body["preview"], preview.display().to_string());
    assert_eq!(body["site"], home);
    let fix = &body["contrast_fixes"][0];
    assert_eq!(fix["token"], "colors.link");
    assert_eq!(fix["before"], "#f0c040");
    assert!(fix["after_ratio"].as_f64().unwrap() >= 4.5, "{fix}");
    assert_eq!(fix["meets_target"], true);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_private_address_is_refused_before_anything_is_sent() {
    let site = site();

    let output = khalid(
        site.path(),
        &["theme", "sync", "http://127.0.0.1:9/"],
        Names::Real,
    )
    .await;

    assert_eq!(output.status.code(), Some(1));
    let said = text(&output.stderr);
    assert!(said.contains("private address"), "{said}");
    assert!(!within(site.path(), "theme/tokens.toml").exists());

    let output = khalid(
        site.path(),
        &["--json", "theme", "sync", "http://10.0.0.1/"],
        Names::Real,
    )
    .await;
    assert_eq!(output.status.code(), Some(1));
    let what = json(&output)["error"]["what"].as_str().unwrap().to_owned();
    assert!(what.contains("private address"), "{what}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_site_that_cannot_be_reached_is_named_with_a_fix() {
    let site = site();

    let args = ["theme", "sync", "http://nowhere.test/"];
    let output = khalid(site.path(), &args, Names::StandIn).await;

    assert_eq!(output.status.code(), Some(1));
    let said = text(&output.stderr);
    assert!(
        said.contains("could not reach http://nowhere.test/"),
        "{said}"
    );
    assert!(
        said.contains("khalid theme sync http://nowhere.test/"),
        "{said}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_error_status_and_a_page_too_large_stop_the_sync() {
    let site = site();
    let (server, _) = stand_in(HOME, None).await;
    let missing = format!("http://{HOST}:{}/gone", server.address().port());
    let (_big, huge) = stand_in(&"x".repeat(2 * 1024 * 1024 + 1), None).await;

    for (address, said) in [(missing, "status 404"), (huge, "larger than the 2 MiB")] {
        let args = ["--json", "theme", "sync", address.as_str()];
        let output = khalid(site.path(), &args, Names::StandIn).await;
        assert_eq!(output.status.code(), Some(1), "{address}");
        let what = json(&output)["error"]["what"].as_str().unwrap().to_owned();
        assert!(what.contains(said), "{what}");
    }
    assert!(!within(site.path(), "theme/tokens.toml").exists());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_page_with_no_stylesheet_writes_nothing() {
    let site = site();
    let bare = r#"<!doctype html><html><head><title>Plain</title>
<link rel="stylesheet" href="https://fonts.example.net/sans.css"></head><body>Hi</body></html>"#;
    let (_server, home) = stand_in(bare, None).await;

    let output = khalid(site.path(), &["theme", "sync", &home], Names::StandIn).await;

    assert_eq!(output.status.code(), Some(1));
    let said = text(&output.stderr);
    assert!(said.contains("found no stylesheet"), "{said}");
    assert!(said.contains("on another site"), "{said}");
    assert!(!within(site.path(), "theme/tokens.toml").exists());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_site_drawn_by_javascript_gets_the_clean_default_and_the_way_to_paste_its_css() {
    let site = site();
    let app = r#"<!doctype html><html><head><title>Acme</title>
<link rel="icon" href="/favicon.ico"></head><body><div id="root"></div>
<script src="/static/js/main.js"></script></body></html>"#;
    let (_server, home) = stand_in(app, None).await;

    let human = khalid(site.path(), &["theme", "sync", &home], Names::StandIn).await;
    let output = khalid(
        site.path(),
        &["--json", "theme", "sync", &home],
        Names::StandIn,
    )
    .await;

    assert_eq!(human.status.code(), Some(0), "{}", text(&human.stderr));
    let said = text(&human.stdout);
    assert!(said.contains("draws its pages with JavaScript"), "{said}");
    assert!(said.contains("clean default"), "{said}");
    assert!(
        said.contains(&format!("khalid theme sync {home} --from-css")),
        "{said}"
    );
    let tokens = fs::read_to_string(within(site.path(), "theme/tokens.toml")).unwrap();
    assert!(tokens.contains("background = \"#ffffff\""), "{tokens}");
    assert!(tokens.contains("favicon.ico"), "{tokens}");
    assert_eq!(json(&output)["drawn_by_script"], true);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_font_the_blog_may_not_serve_is_named_in_plain_words() {
    let site = site();
    let kit = HOME.replace(
        "<link rel=\"stylesheet\" href=\"/style.css\">",
        "<link rel=\"stylesheet\" href=\"https://use.typekit.net/kxv2mqa.css\">\n\
         <link rel=\"stylesheet\" href=\"/style.css\">",
    );
    let style = format!("{STYLE}body {{ font-family: proxima-nova, Arial, sans-serif; }}\n");
    let (_server, home) = stand_in(&kit, Some(&style)).await;

    let human = khalid(site.path(), &["theme", "sync", &home], Names::StandIn).await;
    let output = khalid(
        site.path(),
        &["--json", "theme", "sync", &home],
        Names::StandIn,
    )
    .await;

    assert_eq!(human.status.code(), Some(0), "{}", text(&human.stderr));
    let said = text(&human.stdout);
    let line = "The font proxima-nova comes from Adobe Fonts, whose licence does not let \
                the blog serve it, so the blog shows the next font in its list.";
    assert!(said.contains(line), "{said}");
    let held = &json(&output)["held_fonts"][0];
    assert_eq!(held["family"], "proxima-nova");
    assert_eq!(held["server"], "use.typekit.net");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_file_in_the_way_of_the_theme_folder_is_named() {
    let site = site();
    fs::write(within(site.path(), "theme"), "not a folder").unwrap();
    let (_server, home) = stand_in(HOME, Some(STYLE)).await;

    let output = khalid(site.path(), &["theme", "sync", &home], Names::StandIn).await;

    assert_eq!(output.status.code(), Some(1));
    let said = text(&output.stderr);
    assert!(said.contains("could not read theme/tokens.toml"), "{said}");
    assert!(
        said.contains("is a folder you can read and write"),
        "{said}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn from_css_reads_pasted_variables_and_never_fetches_the_site() {
    let site = site();
    let pasted = within(site.path(), "pasted.css");
    let variables = ":root { --color-background: #fdfcfa; --color-text: #1e2a3a; }\n";
    fs::write(&pasted, variables).unwrap();
    let css = pasted.display().to_string();
    let home = "https://northwind.invalid/";

    let args = ["theme", "sync", home, "--from-css", &css];
    let output = khalid(site.path(), &args, Names::Real).await;

    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let said = text(&output.stdout);
    assert!(
        said.contains(&format!("from the CSS in {css} into")),
        "{said}"
    );
    let tokens = fs::read_to_string(within(site.path(), "theme/tokens.toml")).unwrap();
    assert!(tokens.contains("background = \"#fdfcfa\""), "{tokens}");
    assert!(tokens.contains("text = \"#1e2a3a\""), "{tokens}");
    let page = within(site.path(), "data/theme-preview/index.html");
    let page = fs::read_to_string(page).unwrap();
    assert!(page.contains("How northwind.invalid will look"), "{page}");
}

#[tokio::test(flavor = "multi_thread")]
async fn from_css_with_a_missing_file_names_it_and_writes_nothing() {
    let site = site();
    let missing = within(site.path(), "nothing-here.css");
    let css = missing.display().to_string();

    let args = [
        "theme",
        "sync",
        "https://northwind.invalid/",
        "--from-css",
        &css,
    ];
    let output = khalid(site.path(), &args, Names::Real).await;

    assert_eq!(output.status.code(), Some(1));
    let said = text(&output.stderr);
    assert!(said.contains(&format!("could not read {css}")), "{said}");
    assert!(!within(site.path(), "theme/tokens.toml").exists());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_preview_that_cannot_be_written_says_the_tokens_were_saved() {
    let site = site();
    fs::write(within(site.path(), "data"), "not a folder").unwrap();
    let (_server, home) = stand_in(HOME, Some(STYLE)).await;

    let output = khalid(site.path(), &["theme", "sync", &home], Names::StandIn).await;

    assert_eq!(output.status.code(), Some(1));
    let said = text(&output.stderr);
    assert!(said.contains("theme-preview"), "{said}");
    assert!(said.contains("theme/tokens.toml is saved"), "{said}");
    assert!(within(site.path(), "theme/tokens.toml").exists());
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn a_theme_link_that_leads_outside_the_site_writes_nothing_there() {
    let site = site();
    let outside = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(outside.path(), within(site.path(), "theme")).unwrap();
    let (_server, home) = stand_in(HOME, Some(STYLE)).await;

    let output = khalid(site.path(), &["theme", "sync", &home], Names::StandIn).await;

    assert_eq!(output.status.code(), Some(1));
    let said = text(&output.stderr);
    assert!(said.contains("a link to a place outside"), "{said}");
    assert!(!within(outside.path(), "tokens.toml").exists());
}

#[tokio::test(flavor = "multi_thread")]
async fn sync_needs_a_site_folder_and_a_web_address() {
    let site = site();

    let args = ["theme", "sync", "ftp://bakery.test/"];
    let output = khalid(site.path(), &args, Names::Real).await;
    assert_eq!(output.status.code(), Some(2), "{}", text(&output.stderr));

    let output = tokio::task::spawn_blocking(|| {
        Command::new(env!("CARGO_BIN_EXE_khalid"))
            .args(["--json", "theme", "sync", "https://www.example.com/"])
            .output()
            .unwrap()
    })
    .await
    .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let what = json(&output)["error"]["what"].as_str().unwrap().to_owned();
    assert!(what.contains("needs a site folder"), "{what}");
}

/// Sets `colors.link` by hand in the site's tokens file and locks it.
#[cfg(test)]
fn lock_link(site: &Path, value: &str) {
    let file = within(site, "theme/tokens.toml");
    let text = fs::read_to_string(&file).unwrap();
    let edited: Vec<String> = text
        .lines()
        .map(|line| {
            if line.starts_with("link = ") {
                format!("link = \"{value}\"")
            } else {
                line.to_owned()
            }
        })
        .collect();
    let locked = format!("locked = [\"colors.link\"]\n{}\n", edited.join("\n"));
    fs::write(&file, locked).unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_second_sync_keeps_the_locked_hand_edit_and_lists_what_changed() {
    let site = site();
    let (_before, home) = stand_in(HOME, Some(STYLE)).await;
    let first = khalid(site.path(), &["theme", "sync", &home], Names::StandIn).await;
    assert_eq!(first.status.code(), Some(0), "{}", text(&first.stderr));
    lock_link(site.path(), "#123456");
    let (_after, home) = stand_in(HOME, Some(RESTYLED)).await;

    let output = khalid(site.path(), &["theme", "sync", &home], Names::StandIn).await;

    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let said = text(&output.stdout);
    assert!(
        said.contains("colors.text changed from #2b2118 to #1b1b1f."),
        "{said}"
    );
    assert!(
        said.contains(
            "colors.link stays #123456, because it is locked. The site now uses #2255aa."
        ),
        "{said}"
    );
    let tokens = fs::read_to_string(within(site.path(), "theme/tokens.toml")).unwrap();
    assert!(tokens.contains("link = \"#123456\""), "{tokens}");
    assert!(tokens.contains("text = \"#1b1b1f\""), "{tokens}");
    assert!(tokens.contains("locked = [\"colors.link\"]"), "{tokens}");

    let args = ["--json", "theme", "sync", home.as_str()];
    let again = json(&khalid(site.path(), &args, Names::StandIn).await);
    let changes = again["changes"].as_array().unwrap();
    assert_eq!(changes.len(), 1, "{again}");
    assert_eq!(changes[0]["token"], "colors.link");
    assert_eq!(changes[0]["kept"], true);
    assert_eq!(changes[0]["file"], "#123456");
    assert_eq!(changes[0]["site"], "#2255aa");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_tokens_file_with_a_token_missing_is_left_as_it_is() {
    let site = site();
    let (_server, home) = stand_in(HOME, Some(STYLE)).await;
    let first = khalid(site.path(), &["theme", "sync", &home], Names::StandIn).await;
    assert_eq!(first.status.code(), Some(0), "{}", text(&first.stderr));
    let file = within(site.path(), "theme/tokens.toml");
    let text_before = fs::read_to_string(&file).unwrap();
    let broken: Vec<&str> = text_before
        .lines()
        .filter(|line| !line.starts_with("link = "))
        .collect();
    fs::write(&file, broken.join("\n")).unwrap();

    let output = khalid(site.path(), &["theme", "sync", &home], Names::StandIn).await;

    assert_eq!(output.status.code(), Some(1));
    let said = text(&output.stderr);
    assert!(
        said.contains("theme/tokens.toml has a mistake on line"),
        "{said}"
    );
    assert!(said.contains("link"), "{said}");
    assert_eq!(fs::read_to_string(&file).unwrap(), broken.join("\n"));
}

/// A site synced from `home`, whose site.toml names `home` as its address.
#[cfg(test)]
async fn synced_site(home: &str) -> tempfile::TempDir {
    let site = site();
    let settings = within(site.path(), "site.toml");
    let settings_text = fs::read_to_string(&settings).unwrap();
    let moved: Vec<String> = settings_text
        .lines()
        .map(|line| {
            if line.starts_with("address = ") {
                format!("address = \"{home}\"")
            } else {
                line.to_owned()
            }
        })
        .collect();
    fs::write(&settings, moved.join("\n")).unwrap();
    let first = khalid(site.path(), &["theme", "sync", home], Names::StandIn).await;
    assert_eq!(first.status.code(), Some(0), "{}", text(&first.stderr));
    site
}

#[tokio::test(flavor = "multi_thread")]
async fn check_after_a_sync_finds_nothing_and_reads_the_address_from_the_site() {
    let (_server, home) = stand_in(HOME, Some(STYLE)).await;
    let site = synced_site(&home).await;

    let output = khalid(site.path(), &["theme", "check"], Names::StandIn).await;

    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let said = text(&output.stdout);
    assert!(said.contains(&format!("against {home}.")), "{said}");
    assert!(
        said.contains("matches the main site, and every colour is readable"),
        "{said}"
    );

    let body = json(&khalid(site.path(), &["--json", "theme", "check"], Names::StandIn).await);
    assert_eq!(body["ok"], true, "{body}");
    assert_eq!(body["drift"], serde_json::json!([]));
}

#[tokio::test(flavor = "multi_thread")]
async fn check_reports_drift_from_a_redesigned_site_and_changes_nothing() {
    let (_before, home) = stand_in(HOME, Some(STYLE)).await;
    let site = synced_site(&home).await;
    let file = within(site.path(), "theme/tokens.toml");
    let saved = fs::read_to_string(&file).unwrap();
    let (_after, redesigned) = stand_in(HOME, Some(RESTYLED)).await;

    let args = ["theme", "check", redesigned.as_str()];
    let output = khalid(site.path(), &args, Names::StandIn).await;

    assert_eq!(output.status.code(), Some(1));
    let said = text(&output.stdout);
    assert!(
        said.contains("colors.text is #2b2118 here, and the main site now uses #1b1b1f."),
        "{said}"
    );
    assert!(
        said.contains(&format!("Run `khalid theme sync {redesigned}`")),
        "{said}"
    );
    assert_eq!(fs::read_to_string(&file).unwrap(), saved);

    let args = ["--json", "theme", "check", redesigned.as_str()];
    let output = khalid(site.path(), &args, Names::StandIn).await;
    assert_eq!(output.status.code(), Some(1));
    let body = json(&output);
    assert_eq!(body["ok"], false);
    let text_drift = body["drift"]
        .as_array()
        .unwrap()
        .iter()
        .find(|change| change["token"] == "colors.text")
        .unwrap();
    assert_eq!(text_drift["kept"], false);
    assert_eq!(text_drift["file"], "#2b2118");
    assert_eq!(text_drift["site"], "#1b1b1f");
}

#[tokio::test(flavor = "multi_thread")]
async fn check_reports_a_faint_colour_even_when_it_is_locked() {
    let (_server, home) = stand_in(HOME, Some(STYLE)).await;
    let site = synced_site(&home).await;
    lock_link(site.path(), "#f0c040");

    let output = khalid(site.path(), &["theme", "check"], Names::StandIn).await;

    assert_eq!(output.status.code(), Some(1));
    let said = text(&output.stdout);
    assert!(
        said.contains("colors.link stays #f0c040, because it is locked."),
        "{said}"
    );
    assert!(
        said.contains("colors.link is #f0c040, with a contrast of 1."),
        "{said}"
    );
    assert!(said.contains("Change the faint colours"), "{said}");

    let output = khalid(site.path(), &["--json", "theme", "check"], Names::StandIn).await;
    let body = json(&output);
    assert_eq!(body["ok"], false);
    assert_eq!(body["drift"][0]["kept"], true);
    let faint = &body["contrast"][0];
    assert_eq!(faint["token"], "colors.link");
    assert_eq!(faint["color"], "#f0c040");
    assert_eq!(faint["target"], 4.5);
    assert_eq!(faint["nearest_passes"], true);
}

#[tokio::test(flavor = "multi_thread")]
async fn check_without_a_tokens_file_says_to_sync_first() {
    let site = site();

    let output = khalid(site.path(), &["--json", "theme", "check"], Names::Real).await;

    assert_eq!(output.status.code(), Some(1));
    let error = &json(&output)["error"];
    assert_eq!(error["what"], "This site has no theme/tokens.toml yet.");
    let fix = error["fix"].as_str().unwrap();
    assert!(
        fix.contains("khalid theme sync https://www.example.com/"),
        "{fix}"
    );
}
