//! `khalid serve --domain` gets its first certificate from pebble, Let's
//! Encrypt's own test server, and renews one that is due (task 5.13, gap
//! audit 2 G35). Ignored unless pebble runs on this machine, started as
//! the first lines of `crates/khalid-https/tests/pebble.rs` say, with its
//! management address on `localhost:15000`. To run them:
//!
//! `cargo test -p khalid --features test-util --test serve_pebble -- --ignored --test-threads 1`
//!
//! Both tests answer pebble's checks on ports 5001 and 5002, so they run
//! one at a time.

use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use khalid_core::AbsoluteUrl;
use khalid_outbound::{Limits, Outbound, Reach, Resolver};
use serde_json::Value;
use tempfile::TempDir;

/// The domain to serve. Pebble's test DNS server answers it with this
/// machine.
const NAME: &str = "blog.khalid.test";

/// Pebble's directory.
const DIRECTORY: &str = "https://localhost:14000/dir";

/// Where pebble hands out the root its certificates chain up to.
const ROOTS: &str = "https://localhost:15000/roots/0";

/// The certificate pebble's own addresses are signed with.
const MINICA: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../target/pebble/pebble.minica.pem"
);

/// A running `khalid serve`, stopped when dropped.
struct Server {
    child: Child,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
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

/// A copy of the demo site with nothing in `data/https`.
#[cfg(test)]
fn demo() -> TempDir {
    let mut demo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    demo.push("../../fixtures/demo-site");
    let site = tempfile::tempdir().unwrap();
    copy_dir(&demo, site.path());
    site
}

/// `data/https/<name>` in `site`.
#[cfg(test)]
fn saved(site: &Path, name: &str) -> PathBuf {
    let mut path = site.to_path_buf();
    path.push("data/https");
    path.push(name);
    path
}

/// Starts `khalid serve --domain NAME` on `site` against pebble, answering
/// its checks on ports 5001 and 5002, and returns it once it says where it
/// answers, with that address. What it printed is in `site/serve.log`.
#[cfg(test)]
fn serve(site: &Path) -> (Server, String) {
    let mut log = site.to_path_buf();
    log.push("serve.log");
    let mut child = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["--site", site.to_str().unwrap(), "--json", "serve"])
        .args(["--domain", NAME, "--accept-terms"])
        .args(["--https-listen", "127.0.0.1:5001"])
        .args(["--http-listen", "127.0.0.1:5002"])
        .args(["--acme-directory", DIRECTORY])
        .args(["--test-hosts", "localhost=127.0.0.1"])
        .args(["--test-root", MINICA])
        .stdout(Stdio::piped())
        .stderr(fs::File::create(&log).unwrap())
        .spawn()
        .unwrap();
    let mut line = String::new();
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    let server = Server { child };
    let said = || fs::read_to_string(&log).unwrap_or_default();
    let json: Value = serde_json::from_str(&line).unwrap_or_else(|_| panic!("{}", said()));
    let address = json["address"].as_str().unwrap().to_owned();
    (server, address)
}

/// The home page at `address`, fetched trusting only pebble's root, so a
/// certificate pebble did not issue fails the handshake.
#[cfg(test)]
fn home_through_pebble(address: &str) -> (u16, String) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let minica = fs::read(MINICA).expect("pebble's minica certificate is saved");
    let hosts = vec![
        ("localhost".to_owned(), vec!["127.0.0.1".parse().unwrap()]),
        (NAME.to_owned(), vec!["127.0.0.1".parse().unwrap()]),
    ];
    let resolver = Resolver::for_tests(hosts, Reach::PublicAndLoopback);
    let pebble = Outbound::trusting(Limits::default(), resolver.clone(), &minica).unwrap();
    let root = runtime
        .block_on(pebble.get(&AbsoluteUrl::parse(ROOTS).unwrap()))
        .unwrap();
    let blog = Outbound::trusting(Limits::default(), resolver, root.body()).unwrap();
    let home = runtime
        .block_on(blog.get(&AbsoluteUrl::parse(address).unwrap()))
        .unwrap();
    (
        home.status(),
        String::from_utf8_lossy(home.body()).into_owned(),
    )
}

#[test]
#[ignore = "needs pebble on this machine; see this file's first lines"]
fn khalid_serve_gets_its_first_certificate_from_pebble_and_saves_it() {
    let site = demo();

    let (_server, address) = serve(site.path());
    let (status, body) = home_through_pebble(&address);

    assert_eq!(address, format!("https://{NAME}:5001/blog/"));
    assert_eq!(status, 200);
    assert!(body.contains("<h1"), "{body}");
    assert!(saved(site.path(), "account.json").is_file());
    assert!(saved(site.path(), "certificate.pem").is_file());
}

#[test]
#[ignore = "needs pebble on this machine; see this file's first lines"]
fn khalid_serve_renews_a_due_certificate_with_the_saved_account() {
    let site = demo();
    drop(serve(site.path()));
    let account = fs::read(saved(site.path(), "account.json")).unwrap();
    let mut params = rcgen::CertificateParams::new(vec![NAME.to_owned()]).unwrap();
    params.not_before = rcgen::date_time_ymd(2026, 1, 1);
    params.not_after = rcgen::date_time_ymd(2026, 1, 2);
    let key = rcgen::KeyPair::generate().unwrap();
    let due = params.self_signed(&key).unwrap().pem();
    fs::write(saved(site.path(), "certificate.pem"), &due).unwrap();
    fs::write(saved(site.path(), "key.pem"), key.serialize_pem()).unwrap();

    let (_server, address) = serve(site.path());
    let (status, _) = home_through_pebble(&address);

    assert_eq!(status, 200);
    assert_ne!(
        fs::read_to_string(saved(site.path(), "certificate.pem")).unwrap(),
        due
    );
    assert_eq!(
        fs::read(saved(site.path(), "account.json")).unwrap(),
        account
    );
}
