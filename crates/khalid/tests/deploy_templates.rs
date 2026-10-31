//! The one-click templates in docs/plan.md 4.7: Fly.io (`fly.toml`),
//! Railway (`railway.json`), Coolify (`packaging/coolify/`) and a Hetzner
//! server (`packaging/hetzner/`). Each one must start the image, or the
//! binary, the way `Dockerfile` and the systemd unit expect (G34, KH-INS-1).

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use serde_json::Value;

/// A running `khalid serve`, stopped when dropped.
struct Server(Child, String);

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// The path of a file in the repository.
#[cfg(test)]
fn repo_file(relative: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../..");
    path.push(relative);
    path
}

#[cfg(test)]
fn read(relative: &str) -> String {
    fs::read_to_string(repo_file(relative)).unwrap()
}

/// The text after the last `instruction ` line of the Dockerfile.
#[cfg(test)]
fn docker_value(instruction: &str) -> String {
    let prefix = format!("{instruction} ");
    let dockerfile = read("Dockerfile");
    let mut found = dockerfile
        .lines()
        .filter_map(|line| line.strip_prefix(&prefix));
    found.next_back().unwrap().to_owned()
}

/// The image's command, `ENTRYPOINT` then `CMD`.
#[cfg(test)]
fn image_command() -> Vec<String> {
    let mut args: Vec<String> = serde_json::from_str(&docker_value("ENTRYPOINT")).unwrap();
    args.extend(serde_json::from_str::<Vec<String>>(&docker_value("CMD")).unwrap());
    args
}

/// Runs `args` as the binary, with `KHALID_ADDRESS` set, and waits for the
/// line that names the address it answers on.
#[cfg(test)]
fn start(args: &[String], address: &str) -> Server {
    let mut child = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(args)
        .arg("--json")
        .env("KHALID_ADDRESS", address)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut line = String::new();
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    let json: Value = serde_json::from_str(&line).unwrap_or_else(|_| panic!("{line:?}"));
    let rest = json["address"].as_str().unwrap().strip_prefix("http://");
    let host = rest.unwrap().split_once('/').unwrap().0.to_owned();
    Server(child, host)
}

#[cfg(test)]
fn home_status(server: &Server) -> u16 {
    let mut stream = TcpStream::connect(&server.1).unwrap();
    let request = format!(
        "GET / HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        server.1
    );
    stream.write_all(request.as_bytes()).unwrap();
    let mut raw = String::new();
    stream.read_to_string(&mut raw).unwrap();
    raw.split(' ').nth(1).unwrap().parse().unwrap()
}

#[cfg(test)]
fn site_toml(folder: &Path) -> String {
    let mut path = folder.to_path_buf();
    path.push("site.toml");
    fs::read_to_string(path).unwrap()
}

/// What a host does on the first start: the image's own command on an
/// empty volume, with the address the Fly.io template sets.
#[test]
fn the_image_makes_the_site_on_an_empty_volume_and_serves_it() {
    let fly: toml::Table = toml::from_str(&read("fly.toml")).unwrap();
    let address = fly["env"]["KHALID_ADDRESS"].as_str().unwrap();
    let volume = tempfile::tempdir().unwrap();
    let args: Vec<String> = image_command()
        .into_iter()
        .skip(1)
        .map(|arg| match arg.as_str() {
            "/site" => volume.path().to_str().unwrap().to_owned(),
            "0.0.0.0:8080" => "127.0.0.1:0".to_owned(),
            _ => arg,
        })
        .collect();
    let server = start(&args, address);
    assert_eq!(home_status(&server), 200);
    assert!(site_toml(volume.path()).contains(&format!("address = \"{address}/\"")));
}

#[test]
fn the_fly_app_builds_the_image_and_keeps_the_site_on_a_volume() {
    let fly: toml::Table = toml::from_str(&read("fly.toml")).unwrap();
    let dockerfile = fly["build"]["dockerfile"].as_str().unwrap();
    assert!(repo_file(dockerfile).is_file(), "{dockerfile}");
    assert_eq!(fly["mounts"]["destination"].as_str(), Some("/site"));
    assert_eq!(docker_value("VOLUME"), "/site");
    let port = fly["http_service"]["internal_port"].as_integer().unwrap();
    assert_eq!(port.to_string(), docker_value("EXPOSE"));
    let app = fly["app"].as_str().unwrap();
    let address = fly["env"]["KHALID_ADDRESS"].as_str().unwrap();
    assert_eq!(address, format!("https://{app}.fly.dev"));
}

#[test]
fn the_railway_service_builds_the_image_once_and_checks_the_home_page() {
    let railway: Value = serde_json::from_str(&read("railway.json")).unwrap();
    assert_eq!(railway["build"]["builder"], "DOCKERFILE");
    let dockerfile = railway["build"]["dockerfilePath"].as_str().unwrap();
    assert!(repo_file(dockerfile).is_file(), "{dockerfile}");
    assert_eq!(railway["deploy"]["numReplicas"], 1);
    assert_eq!(railway["deploy"]["healthcheckPath"], "/");
}

#[test]
fn the_coolify_service_takes_coolify_s_address_and_keeps_the_site_on_a_volume() {
    let compose = read("packaging/coolify/compose.yaml");
    let lines: Vec<&str> = compose.lines().map(str::trim).collect();
    let port = docker_value("EXPOSE");
    for wanted in [
        "context: ../..",
        &format!("- SERVICE_URL_KHALID_{port}"),
        &format!("- KHALID_ADDRESS=${{SERVICE_URL_KHALID_{port}}}"),
        "- khalid-site:/site",
        "read_only: true",
        "- no-new-privileges:true",
    ] {
        assert!(
            lines.contains(&wanted),
            "the Coolify file has no line {wanted}"
        );
    }
    assert!(repo_file("packaging/coolify/../../Dockerfile").is_file());
}

/// The quoted command after `marker` in the Hetzner cloud config, up to
/// the closing quote.
#[cfg(test)]
fn after<'a>(config: &'a str, marker: &str) -> &'a str {
    let (_, rest) = config
        .split_once(marker)
        .unwrap_or_else(|| panic!("no {marker}"));
    rest.split_once('"').unwrap().0
}

/// The server's `runcmd`: the install line the guide gives, then `init`
/// at the domain, which this test runs into a temp folder.
#[test]
fn the_hetzner_server_installs_khalid_and_makes_the_site_at_its_domain() {
    let config = read("packaging/hetzner/cloud-config.yaml");
    let guide = read("docs/deploy.md");
    let install = after(&config, "[sh, -c, \"");
    let (download, run) = install.split_once(" | ").unwrap();
    assert!(guide.contains(download), "the guide has no {download}");
    assert_eq!(run, "sh -s -- --systemd");
    let domain = after(&config, "KHALID_DOMAIN=").lines().next().unwrap();
    assert_eq!(domain, "blog.example.com");
    let init = after(&config, "runuser -u khalid -- /usr/local/bin/khalid ");
    let site = tempfile::tempdir().unwrap();
    let args: Vec<String> = init
        .replace("$KHALID_DOMAIN", domain)
        .replace("/var/lib/khalid", site.path().to_str().unwrap())
        .split(' ')
        .map(str::to_owned)
        .collect();
    let made = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(args)
        .output()
        .unwrap();
    assert!(made.status.success(), "{made:?}");
    assert!(site_toml(site.path()).contains("address = \"https://blog.example.com/\""));
}

/// The server's own command replaces the unit's: the same binary and site
/// folder, with HTTPS for the domain instead of a local port.
#[test]
fn the_hetzner_server_serves_the_unit_s_site_over_https_for_its_domain() {
    let config = read("packaging/hetzner/cloud-config.yaml");
    let unit = read("packaging/systemd/khalid.service");
    let local = unit
        .lines()
        .find_map(|line| line.strip_prefix("ExecStart="))
        .unwrap();
    let own = config
        .lines()
        .filter_map(|line| line.trim().strip_prefix("ExecStart="))
        .find(|command| !command.is_empty())
        .unwrap();
    let serve = local.split_once(" --listen ").unwrap().0;
    assert_eq!(
        own,
        format!("{serve} --domain ${{KHALID_DOMAIN}} --accept-terms")
    );
    assert!(
        config.contains("ExecStart=\n"),
        "the unit's command is not cleared first"
    );
    assert!(config.contains("EnvironmentFile=/etc/khalid/domain.env\n"));
}
