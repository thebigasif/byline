//! `khalid serve --domain`: the demo blog over HTTPS with the site's saved
//! certificate, port 80 moving visitors to HTTPS, and no start without the
//! owner's yes to Let's Encrypt's terms (task 5.13).

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use khalid_core::AbsoluteUrl;
use khalid_outbound::{Limits, Outbound, Reach, Resolver};
use serde_json::Value;
use tempfile::TempDir;

/// The domain the test serves, resolved to loopback.
const NAME: &str = "blog.test";

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

/// A copy of the demo site with a saved self-signed certificate for
/// `NAME` in `data/https`, and the certificate as PEM.
#[cfg(test)]
fn demo_with_certificate() -> (TempDir, String) {
    let mut demo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    demo.push("../../fixtures/demo-site");
    let site = tempfile::tempdir().unwrap();
    copy_dir(&demo, site.path());
    let made = rcgen::generate_simple_self_signed(vec![NAME.to_owned()]).unwrap();
    let mut path = site.path().to_path_buf();
    path.push("data/https");
    fs::create_dir_all(&path).unwrap();
    path.push("certificate.pem");
    fs::write(&path, made.cert.pem()).unwrap();
    path.set_file_name("key.pem");
    fs::write(&path, made.signing_key.serialize_pem()).unwrap();
    (site, made.cert.pem())
}

/// A port nothing listens on right now.
#[cfg(test)]
fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

/// `khalid serve --domain NAME` on `site` with `extra` flags, its HTTPS
/// and port 80 on loopback `ports`, and Let's Encrypt out of reach.
#[cfg(test)]
fn khalid(site: &Path, ports: (u16, u16), extra: &[&str]) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_khalid"));
    command
        .args(["--site", site.to_str().unwrap(), "serve", "--domain", NAME])
        .args(["--https-listen", &format!("127.0.0.1:{}", ports.0)])
        .args(["--http-listen", &format!("127.0.0.1:{}", ports.1)])
        .args(["--acme-directory", "http://127.0.0.1:9/dir"])
        .args(["--test-hosts", "blog.test=127.0.0.1"])
        .args(extra);
    command
}

#[test]
fn the_demo_blog_is_served_over_https_with_the_saved_certificate() {
    let (site, root) = demo_with_certificate();
    let ports = (free_port(), free_port());
    let mut child = khalid(site.path(), ports, &["--accept-terms", "--json"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut line = String::new();
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    let _server = Server { child };
    let json: Value = serde_json::from_str(&line).unwrap_or_else(|_| panic!("{line}"));
    let address = json["address"].as_str().unwrap();

    let hosts = vec![(NAME.to_owned(), vec!["127.0.0.1".parse().unwrap()])];
    let resolver = Resolver::for_tests(hosts, Reach::PublicAndLoopback);
    let outbound = Outbound::trusting(Limits::default(), resolver, root.as_bytes()).unwrap();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let home = runtime
        .block_on(outbound.get(&AbsoluteUrl::parse(address).unwrap()))
        .unwrap();

    assert_eq!(address, format!("https://blog.test:{}/blog/", ports.0));
    assert_eq!(home.status(), 200);
    assert!(String::from_utf8_lossy(home.body()).contains("<h1"));
}

#[test]
fn port_80_moves_a_visitor_to_https() {
    let (site, _) = demo_with_certificate();
    let ports = (free_port(), free_port());
    let mut child = khalid(site.path(), ports, &["--accept-terms", "--json"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut line = String::new();
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    let _server = Server { child };

    let mut stream = TcpStream::connect(("127.0.0.1", ports.1)).unwrap();
    let request = "GET /blog/?ref=mail HTTP/1.1\r\nHost: blog.test\r\nConnection: close\r\n\r\n";
    stream.write_all(request.as_bytes()).unwrap();
    let mut answer = String::new();
    stream.read_to_string(&mut answer).unwrap();

    assert!(answer.starts_with("HTTP/1.1 308"), "{answer}");
    assert!(
        answer.contains("location: https://blog.test/blog/?ref=mail"),
        "{answer}"
    );
}

#[test]
fn a_static_site_refuses_https_and_says_why() {
    let (site, _) = demo_with_certificate();
    let mut settings = site.path().to_path_buf();
    settings.push("site.toml");
    let text = fs::read_to_string(&settings).unwrap();
    fs::write(
        &settings,
        text.replace("mode = \"server\"", "mode = \"static\""),
    )
    .unwrap();

    let output = khalid(site.path(), (free_port(), free_port()), &["--accept-terms"])
        .stdin(Stdio::null())
        .output()
        .unwrap();

    let said = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "{said}");
    assert!(said.contains("off for a static site"), "{said}");
}

#[test]
fn with_no_one_to_ask_and_no_accept_terms_khalid_says_how_to_agree() {
    let (site, _) = demo_with_certificate();
    let ports = (free_port(), free_port());

    let output = khalid(site.path(), ports, &[])
        .stdin(Stdio::null())
        .output()
        .unwrap();

    let said = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "{said}");
    assert!(said.contains("letsencrypt.org/repository"), "{said}");
    assert!(said.contains("--accept-terms"), "{said}");
}
