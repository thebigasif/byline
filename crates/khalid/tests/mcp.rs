//! `khalid mcp`: an agent starts it with a key and talks to it over a
//! pipe, one JSON-RPC message per line (KH-API-5, ADR "the MCP server
//! calls the API in process").

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Output, Stdio};

use serde_json::{Value, json};

/// A copy of the demo site, with a key made for it that may write posts.
#[cfg(test)]
fn demo() -> (tempfile::TempDir, String) {
    let mut fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    fixture.push("../../fixtures/demo-site");
    let site = tempfile::tempdir().unwrap();
    copy_dir(&fixture, site.path());
    let key = make_key(
        site.path(),
        &[
            "--name",
            "Claude",
            "--scope",
            "posts:write",
            "--scope",
            "posts:publish",
        ],
    );
    (site, key)
}

/// Makes a key with `args` and returns it.
#[cfg(test)]
fn make_key(site: &Path, args: &[&str]) -> String {
    let made = khalid(site)
        .args(["keys", "create", "--json"])
        .args(args)
        .output()
        .unwrap();
    assert_eq!(made.status.code(), Some(0), "{}", text(&made.stderr));
    let made: Value = serde_json::from_slice(&made.stdout).unwrap();
    made["key"].as_str().unwrap().to_owned()
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

#[cfg(test)]
fn khalid(site: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_khalid"));
    command.arg("--site").arg(site).env_remove("KHALID_KEY");
    command
}

#[cfg(test)]
fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// A running `khalid mcp`, asked one request at a time.
#[cfg(test)]
struct Session {
    child: Child,
    input: ChildStdin,
    answers: BufReader<ChildStdout>,
    next: u64,
}

#[cfg(test)]
impl Session {
    fn start(site: &Path, key: &str) -> Self {
        let mut child = khalid(site)
            .arg("mcp")
            .env("KHALID_KEY", key)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let input = child.stdin.take().unwrap();
        let answers = BufReader::new(child.stdout.take().unwrap());
        Self {
            child,
            input,
            answers,
            next: 1,
        }
    }

    fn ask(&mut self, method: &str, params: &Value) -> Value {
        let id = self.next;
        self.next += 1;
        let request = json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params });
        writeln!(self.input, "{request}").unwrap();
        let mut line = String::new();
        self.answers.read_line(&mut line).unwrap();
        let answer: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(answer["id"], id, "{answer}");
        answer
    }

    /// Calls `tool` and returns whether it failed, with its JSON answer.
    fn call(&mut self, tool: &str, arguments: &Value) -> (bool, Value) {
        let answer = self.ask(
            "tools/call",
            &json!({ "name": tool, "arguments": arguments }),
        );
        let result = &answer["result"];
        let body = result["content"][0]["text"].as_str().unwrap();
        (
            result["isError"] == true,
            serde_json::from_str(body).unwrap(),
        )
    }

    /// Closes stdin, as an agent does when it is done, and waits.
    fn finish(self) -> Output {
        drop(self.input);
        self.child.wait_with_output().unwrap()
    }
}

#[test]
fn an_agent_drafts_and_publishes_a_post_behind_the_lint_gate() {
    let (site, key) = demo();
    let mut session = Session::start(site.path(), &key);
    let hello = json!({ "protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": { "name": "test", "version": "0" } });
    let started = session.ask("initialize", &hello);
    assert_eq!(
        started["result"]["serverInfo"]["name"], "khalid",
        "{started}"
    );
    let tools = session.ask("tools/list", &json!({}));
    let names = tools["result"]["tools"].to_string();
    for needed in [
        "browse_posts",
        "create_admin_post",
        "lint_post",
        "publish_post",
    ] {
        assert!(
            names.contains(&format!("\"{needed}\"")),
            "{needed}: {names}"
        );
    }

    let weak = json!({ "posts": [{ "title": "A weak post", "markdown": "No link and no author. See [[no-such-page]]." }] });
    let (failed, weak) = session.call("create_admin_post", &json!({ "body": weak }));
    assert!(!failed, "{weak}");
    let (failed, report) = session.call("lint_post", &json!({ "id": "en.a-weak-post" }));
    assert!(!failed, "linting worked, whatever it found: {report}");
    assert_eq!(report["blocks"], true, "{report}");
    let rules: Vec<&Value> = report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| &f["rule"])
        .collect();
    assert!(rules.contains(&&json!("no-author")), "{rules:?}");
    assert!(rules.contains(&&json!("no-internal-link")), "{rules:?}");
    assert!(rules.contains(&&json!("broken-wikilink")), "{rules:?}");
    assert!(
        !rules.contains(&&json!("no-date")),
        "linted as it would go live: {rules:?}"
    );
    let publish = json!({ "id": "en.a-weak-post", "updated_at": weak["posts"][0]["updated_at"] });
    let (refused, problem) = session.call("publish_post", &publish);
    assert!(refused, "{problem}");
    assert_eq!(problem["status"], 422);
    let findings = problem["lint"]["findings"].to_string();
    assert!(findings.contains("no-author"), "{findings}");

    let good = json!({ "posts": [{
        "title": "Choosing anchor text",
        "author": "asif",
        "markdown": "Anchor text is the visible words of a link.\n\nRead [why internal links matter](/blog/why-internal-links-matter/) first."
    }] });
    let (_, made) = session.call("create_admin_post", &json!({ "body": good }));
    let publish =
        json!({ "id": "en.choosing-anchor-text", "updated_at": made["posts"][0]["updated_at"] });
    let (refused, live) = session.call("publish_post", &publish);
    assert!(!refused, "{live}");
    assert_eq!(live["posts"][0]["status"], "published");

    let (_, listed) = session.call(
        "browse_posts",
        &json!({ "filter": "slug:choosing-anchor-text" }),
    );
    assert_eq!(
        listed["posts"][0]["id"], "en.choosing-anchor-text",
        "{listed}"
    );
    let output = session.finish();
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let mut kept = site.path().to_path_buf();
    kept.push("drafts/en/a-weak-post.md");
    assert!(kept.is_file(), "the refused draft stays a draft");
}

#[test]
fn a_missing_read_only_or_revoked_key_stops_it_before_any_message() {
    let (site, key) = demo();
    let reader = make_key(site.path(), &["--name", "Web page", "--kind", "content"]);
    let id = key.split('_').nth(1).unwrap();
    let revoked = khalid(site.path()).args(["keys", "revoke", id]).output();
    assert_eq!(revoked.unwrap().status.code(), Some(0));
    for (key, says) in [
        (None, "khalid keys create"),
        (Some(reader.as_str()), "content key"),
        (Some(key.as_str()), "not accepted"),
    ] {
        let mut command = khalid(site.path());
        command.arg("mcp");
        if let Some(key) = key {
            command.env("KHALID_KEY", key);
        }
        let output = command.output().unwrap();
        let message = text(&output.stderr);
        assert_eq!(output.status.code(), Some(1), "{message}");
        assert_eq!(text(&output.stdout), "", "stdout is only for the protocol");
        assert!(message.contains(says), "{says}: {message}");
    }
}

#[test]
fn the_agents_guide_explains_every_tool_the_server_offers() {
    let (site, key) = demo();
    let mut session = Session::start(site.path(), &key);
    let tools = session.ask("tools/list", &json!({}));
    drop(session.finish());
    let guide = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .arg("agents")
        .output()
        .unwrap();
    assert_eq!(guide.status.code(), Some(0), "{}", text(&guide.stderr));
    let guide = text(&guide.stdout);
    assert!(
        guide.contains("`operationId`"),
        "the guide explains how operations become tools"
    );
    let mut document = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    document.push("../../sdk/typescript/openapi.json");
    let document = fs::read_to_string(document).unwrap();
    for tool in tools["result"]["tools"].as_array().unwrap() {
        let name = tool["name"].as_str().unwrap();
        if document.contains(&format!("\"operationId\": \"{name}\"")) {
            continue;
        }
        assert!(
            guide.contains(&format!("`{name}`")),
            "the guide never names {name}"
        );
    }
}
