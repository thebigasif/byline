//! The command line contract for `init`, `new` and `build`, task 1.16: every
//! outcome in human and `--json` output, with its exit code (RULES.md BL-CLI-1
//! to BL-CLI-4, ADR 0006), and the journey from `init` to a served post.

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::process::{Command, Output, Stdio};

use serde_json::Value;

#[cfg(test)]
fn khalid(args: &[String]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(args)
        .output()
        .unwrap()
}

#[cfg(test)]
fn path(base: &Path, rel: &str) -> String {
    let mut path = base.to_path_buf();
    path.push(rel);
    path.to_str().unwrap().to_owned()
}

/// Every outcome of the three commands, with the exit code it must give: 0
/// success, 1 a failure the person must act on, 2 a usage error. `site` is a
/// site made by `khalid init`, and `fresh` a folder that does not exist yet.
/// `build` runs before `new`, because the publish gate refuses a new post
/// until it has an author and a link; `tests/build.rs` covers that refusal.
fn cases(site: &str, fresh: &str) -> Vec<(Vec<String>, i32)> {
    let address = "https://www.example.com";
    let content = format!("{site}/content/html");
    let table: [(&[&str], i32); 10] = [
        (&["init", fresh], 2),
        (&["init", site, "--address", address], 1),
        (
            &["init", fresh, "--address", address, "--mount", "/a//b/"],
            1,
        ),
        (&["init", fresh, "--address", address], 0),
        (&["--site", site, "build"], 0),
        (&["--site", site, "build", "--out", &content], 1),
        (&["build"], 2),
        (&["--site", site, "new", "Why links matter"], 0),
        (&["--site", site, "new", "Bonjour", "--language", "fr"], 1),
        (&["new", "Why links matter"], 2),
    ];
    table
        .iter()
        .map(|(args, code)| (args.iter().map(|&arg| arg.to_owned()).collect(), *code))
        .collect()
}

/// A temporary folder with a site in `site/` and nothing at `fresh/`.
#[cfg(test)]
fn folders() -> (tempfile::TempDir, String, String) {
    let temp = tempfile::tempdir().unwrap();
    let (site, fresh) = (path(temp.path(), "site"), path(temp.path(), "fresh"));
    let made = khalid(&[
        "init".to_owned(),
        site.clone(),
        "--address".to_owned(),
        "https://www.example.com".to_owned(),
    ]);
    assert_eq!(made.status.code(), Some(0));
    (temp, site, fresh)
}

/// Checks one sentence of a message the way BL-CLI-3 asks.
fn assert_plain(sentence: &str, context: &str) {
    assert!(sentence.ends_with('.'), "{context}: {sentence:?}");
    assert!(
        sentence.split_whitespace().count() < 25,
        "{context}: {sentence:?}"
    );
    assert!(!sentence.contains('\u{2014}'), "{context}: {sentence:?}");
}

#[test]
fn human_output_puts_results_on_stdout_and_messages_on_stderr() {
    let (_temp, site, fresh) = folders();
    for (args, code) in cases(&site, &fresh) {
        let context = args.join(" ");
        let output = khalid(&args);
        assert_eq!(output.status.code(), Some(code), "khalid {context}");
        let (stdout, stderr) = (
            String::from_utf8(output.stdout).unwrap(),
            String::from_utf8(output.stderr).unwrap(),
        );
        if code == 0 {
            assert!(
                !stdout.is_empty() && stderr.is_empty(),
                "khalid {context}: {stderr}"
            );
        } else {
            assert!(stdout.is_empty(), "khalid {context}: {stdout}");
            let lines: Vec<&str> = stderr.lines().collect();
            assert_eq!(
                lines.len(),
                3,
                "khalid {context}: what, why, fix:\n{stderr}"
            );
            for line in lines {
                assert_plain(line, &context);
            }
        }
    }
}

#[test]
fn json_output_is_one_value_on_stdout_with_an_error_only_on_failure() {
    let (_temp, site, fresh) = folders();
    for (mut args, code) in cases(&site, &fresh) {
        args.push("--json".to_owned());
        let context = args.join(" ");
        let output = khalid(&args);
        assert_eq!(output.status.code(), Some(code), "khalid {context}");
        assert!(output.stderr.is_empty(), "khalid {context} wrote to stderr");
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert_eq!(stdout.lines().count(), 1, "khalid {context}: {stdout}");
        let json: Value = serde_json::from_str(&stdout).unwrap();
        assert_eq!(
            json.get("error").is_some(),
            code != 0,
            "khalid {context}: {json}"
        );
        if code != 0 {
            for part in ["what", "why", "fix"] {
                assert_plain(json["error"][part].as_str().unwrap(), &context);
            }
        }
    }
}

/// Gives the one post `khalid new` wrote in `site` an author and two links,
/// which the publish gate asks for before `build` publishes it.
#[cfg(test)]
fn finish_the_new_post(site: &str) {
    let posts = fs::read_dir(path(Path::new(site), "content/en/posts")).unwrap();
    let file = posts.map(|entry| entry.unwrap().path()).next().unwrap();
    let mut text = fs::read_to_string(&file)
        .unwrap()
        .replacen("---\n", "---\nauthor: asif\n", 1);
    text.push_str("Read [the home page](/) and [a guide](https://example.org/).\n");
    fs::write(&file, text).unwrap();
}

/// The last check of task 1.16's done line: `init`, then `new`, then `build`
/// give a site that `serve` serves. Since the publish gate (task 2.6), the
/// new post needs an author and a link before `build` publishes it.
#[test]
fn init_new_build_then_serve_serves_the_post() {
    let (_temp, site, _) = folders();
    let new = khalid(&[
        "--site".to_owned(),
        site.clone(),
        "new".to_owned(),
        "Why links matter".to_owned(),
    ]);
    assert_eq!(new.status.code(), Some(0));
    finish_the_new_post(&site);
    let build = khalid(&["--site".to_owned(), site.clone(), "build".to_owned()]);
    assert_eq!(build.status.code(), Some(0));

    let mut server = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["--site", &site, "serve", "--listen", "127.0.0.1:0"])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut first = String::new();
    BufReader::new(server.stdout.take().unwrap())
        .read_line(&mut first)
        .unwrap();
    // "Serving <name> at http://127.0.0.1:<port>/. Press Ctrl+C to stop."
    let address = first
        .split_whitespace()
        .find_map(|word| word.strip_prefix("http://"))
        .unwrap();
    let listening = address.split('/').next().unwrap();
    let mut stream = TcpStream::connect(listening).unwrap();
    write!(
        stream,
        "GET /why-links-matter/ HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
    )
    .unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    server.kill().unwrap();
    server.wait().unwrap();
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    assert!(response.contains("Why links matter</h1>"), "{response}");
}
