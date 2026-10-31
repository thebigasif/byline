//! The newest release for `khalid update`: found through the release's
//! `SHA256SUMS`, which names every file and the version in its name.

use khalid_core::{AbsoluteUrl, UserMessage};
use khalid_outbound::Outbound;
use semver::Version;

use crate::update_message;

/// Where the project's releases are. GitHub sends `latest/download/<file>`
/// to the file in the newest published release.
pub(crate) const RELEASES: &str = "https://github.com/thebigasif/khalid/releases";

/// The newest release's file for this system.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Release {
    /// The version, from the tag without its `v`.
    pub(crate) version: Version,
    /// The tag, such as `v0.2.0`.
    tag: String,
    /// The file's name, such as `khalid-v0.2.0-linux-x86_64`.
    file: String,
}

/// What the release workflow calls this system's file, after the tag, or
/// `None` when no release is built for it.
pub(crate) fn platform() -> Option<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => Some("linux-x86_64"),
        ("linux", "aarch64") => Some("linux-aarch64"),
        ("macos", "aarch64") => Some("macos-aarch64"),
        ("windows", "x86_64") => Some("windows-x86_64.exe"),
        _ => None,
    }
}

/// Reads the newest release's `SHA256SUMS` from `base` and finds the file
/// for `platform` in it.
///
/// # Errors
///
/// A [`UserMessage`] when the list cannot be fetched or read, or names no
/// file for `platform`.
pub(crate) async fn newest(
    client: &Outbound,
    base: &str,
    platform: &str,
) -> Result<Release, UserMessage> {
    let url = address(&format!("{base}/latest/download/SHA256SUMS"))?;
    let sums = fetch(client, &url).await?;
    let sums = String::from_utf8_lossy(&sums);
    sums.lines()
        .find_map(|line| entry(line, platform))
        .ok_or_else(|| update_message::no_file(platform, &url))
}

/// One line of `SHA256SUMS`, `<sum>  khalid-<tag>-<platform>`, as a
/// release, or `None` when it is for another system or cannot be read.
fn entry(line: &str, platform: &str) -> Option<Release> {
    let (sum, file) = line.split_once(char::is_whitespace)?;
    let file = file.trim_start_matches([' ', '*']).trim_end();
    let tag = file
        .strip_prefix("khalid-")?
        .strip_suffix(platform)?
        .strip_suffix('-')?;
    let version = Version::parse(tag.strip_prefix('v').unwrap_or(tag)).ok()?;
    (sum.len() == 64 && sum.bytes().all(|byte| byte.is_ascii_hexdigit())).then(|| Release {
        version,
        tag: tag.to_owned(),
        file: file.to_owned(),
    })
}

impl Release {
    /// Where this release's file can be downloaded from `base`.
    pub(crate) fn address(&self, base: &str) -> String {
        format!("{base}/download/{}/{}", self.tag, self.file)
    }
}

/// `text` as an address, which every caller builds from fixed parts.
fn address(text: &str) -> Result<AbsoluteUrl, UserMessage> {
    AbsoluteUrl::parse(text).map_err(|_| update_message::unreachable(text, "it is not an address"))
}

/// The body at `url`, when it answers with success.
async fn fetch(client: &Outbound, url: &AbsoluteUrl) -> Result<Vec<u8>, UserMessage> {
    let fetched = client
        .get(url)
        .await
        .map_err(|error| update_message::unreachable(url.as_str(), &error.to_string()))?;
    if (200..300).contains(&fetched.status()) {
        Ok(fetched.body().to_vec())
    } else {
        let status = format!("it answered with the error {}", fetched.status());
        Err(update_message::unreachable(url.as_str(), &status))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_for_this_system_gives_the_version_and_the_file() {
        let sum = "A".repeat(64);
        let line = format!("{sum}  khalid-v0.2.0-rc.1-linux-x86_64");
        let release = entry(&line, "linux-x86_64").unwrap();
        assert_eq!(release.version, Version::parse("0.2.0-rc.1").unwrap());
        assert_eq!(
            release.address("https://example.com/releases"),
            "https://example.com/releases/download/v0.2.0-rc.1/khalid-v0.2.0-rc.1-linux-x86_64"
        );
    }

    #[test]
    fn lines_for_other_systems_or_with_a_bad_sum_are_skipped() {
        let sum = "0".repeat(64);
        for line in [
            format!("{sum}  khalid-v0.2.0-linux-aarch64"),
            "abc  khalid-v0.2.0-linux-x86_64".to_owned(),
            format!("{sum}  khalid-latest-linux-x86_64"),
        ] {
            assert_eq!(entry(&line, "linux-x86_64"), None, "{line}");
        }
    }
}
