//! `docs/getting-started.md` works as written: every `khalid` line in its
//! shell blocks runs in order and succeeds, with each file it says to save
//! written first. Only `serve`, which runs until stopped, is left out; the
//! README's own test serves a site. Finding 8 of the 1.0 readiness check.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// One step of the guide.
#[derive(Debug)]
enum Step {
    /// Save `text` at the path, from the folder the reader works in.
    Save(String, String),
    /// Run the command line, which starts with `khalid `.
    Run(String),
}

/// The file at `relative` from the root of the repository.
#[cfg(test)]
fn repository(relative: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../..");
    path.push(relative);
    path
}

/// `name` inside `dir`.
#[cfg(test)]
fn within(dir: &Path, name: &str) -> PathBuf {
    let mut path = dir.to_path_buf();
    path.push(name);
    path
}

/// The steps of `guide` in order. A block follows a line that ends with
/// "Save this as `<path>`:" to be saved, and a `sh` block's `khalid` lines
/// are run.
#[cfg(test)]
fn steps(guide: &str) -> Vec<Step> {
    let mut steps = Vec::new();
    let mut lines = guide.lines();
    let mut save: Option<String> = None;
    while let Some(line) = lines.next() {
        if let Some(fence) = line.strip_prefix("```") {
            let block: Vec<&str> = lines.by_ref().take_while(|line| *line != "```").collect();
            if let Some(path) = save.take() {
                steps.push(Step::Save(path, format!("{}\n", block.join("\n"))));
            } else if fence == "sh" {
                let runs = block.iter().filter(|line| line.starts_with("khalid "));
                steps.extend(runs.map(|line| Step::Run((*line).to_owned())));
            }
        } else if !line.trim().is_empty() {
            save = line
                .strip_suffix("`:")
                .and_then(|line| line.rsplit_once("Save this as `"))
                .map(|(_, path)| path.to_owned());
        }
    }
    steps
}

/// The words of a command line, with double quotes holding spaces.
#[cfg(test)]
fn words(line: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut quoted = false;
    for character in line.chars() {
        match character {
            '"' => quoted = !quoted,
            ' ' if !quoted => {
                if !word.is_empty() {
                    words.push(std::mem::take(&mut word));
                }
            }
            _ => word.push(character),
        }
    }
    words.push(word);
    words
}

#[test]
fn the_getting_started_guide_runs_as_written() {
    let guide = fs::read_to_string(repository("docs/getting-started.md")).unwrap();
    let steps = steps(&guide);
    let runs = steps.iter().filter(|step| matches!(step, Step::Run(_)));
    assert!(runs.count() >= 8, "{steps:?}");
    let here = tempfile::tempdir().unwrap();
    // The guide copies Khalid's sample WordPress export beside the blog.
    let sample = repository("crates/khalid-import/fixtures/wordpress/basic.xml");
    fs::copy(sample, within(here.path(), "export.xml")).unwrap();
    for step in &steps {
        match step {
            Step::Save(path, text) => {
                let file = within(here.path(), path);
                fs::create_dir_all(file.parent().unwrap()).unwrap();
                fs::write(file, text).unwrap();
            }
            Step::Run(line) if words(line).iter().any(|word| word == "serve") => {}
            Step::Run(line) => {
                let output = Command::new(env!("CARGO_BIN_EXE_khalid"))
                    .args(&words(line)[1..])
                    .current_dir(here.path())
                    .output()
                    .unwrap();
                assert!(
                    output.status.success(),
                    "`{line}` failed:\n{}{}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }
    }
    let published = within(here.path(), "blog/content/en/posts");
    assert_eq!(fs::read_dir(published).unwrap().count(), 1);
    assert!(within(here.path(), "blog-restored/data/site.db").is_file());
}

/// Before the first release, the README's way in is a build from source,
/// and the build steps make the admin, which `cargo` alone leaves out.
#[test]
fn the_readme_and_contributing_say_how_to_build_with_the_admin() {
    let readme = fs::read_to_string(repository("README.md")).unwrap();
    let contributing = fs::read_to_string(repository("CONTRIBUTING.md")).unwrap();
    assert!(readme.contains("docs/getting-started.md"), "{readme}");
    assert!(readme.contains("CONTRIBUTING.md"), "{readme}");
    assert!(!readme.contains("Phase 0 complete"), "{readme}");
    for wanted in [
        "npm ci",
        "npm run build",
        "cargo install --path crates/khalid --locked",
        "without the admin",
    ] {
        assert!(
            contributing.contains(wanted),
            "CONTRIBUTING.md has no {wanted}"
        );
    }
    let package = fs::read_to_string(repository("admin/package.json")).unwrap();
    assert!(package.contains("\"build\":"), "{package}");
    assert!(repository("admin/package-lock.json").is_file());
}
