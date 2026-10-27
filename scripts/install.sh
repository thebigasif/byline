#!/bin/sh
# Installs Khalid: downloads the binary for this machine from a GitHub
# release, checks it against the release's SHA256SUMS, and puts it in a
# folder on your PATH. With --systemd it also sets up the service.
#
#   curl -fsSL https://raw.githubusercontent.com/thebigasif/khalid/main/scripts/install.sh | sh
#
# Options:
#   --version vX.Y.Z   install this release instead of the newest one
#   --dir FOLDER       install into FOLDER [default: /usr/local/bin as root,
#                      else ~/.local/bin]
#   --systemd          also install the systemd unit and its user (root only)
#
# KHALID_RELEASES points at another copy of the releases, for mirrors and
# tests. Its files are named as on GitHub: <tag>/khalid-<tag>-<platform>
# and <tag>/SHA256SUMS. KHALID_UNIT_SOURCE does the same for the systemd
# files, read from <tag>/packaging/systemd/.
set -eu

repo="https://github.com/thebigasif/khalid"
releases="${KHALID_RELEASES:-$repo/releases/download}"
unit_source="${KHALID_UNIT_SOURCE:-https://raw.githubusercontent.com/thebigasif/khalid}"

# Every message says what happened, why, and what to do.
fail() {
  printf 'Khalid was not installed. %s\n' "$1" >&2
  shift
  for line in "$@"; do printf '%s\n' "$line" >&2; done
  exit 1
}

version=""
dir=""
systemd="no"
while [ $# -gt 0 ]; do
  case "$1" in
    --version) [ $# -ge 2 ] || fail "--version needs a release name." "Write it like --version v0.1.0."
      version="$2"; shift 2 ;;
    --dir) [ $# -ge 2 ] || fail "--dir needs a folder." "Write it like --dir /usr/local/bin."
      dir="$2"; shift 2 ;;
    --systemd) systemd="yes"; shift ;;
    *) fail "The option $1 is not one this script knows." "Use --version, --dir or --systemd." ;;
  esac
done

case "$(uname -s)" in
  Linux) os="linux" ;;
  Darwin) os="macos" ;;
  *) fail "This script installs on Linux and macOS only." "On Windows, download khalid.exe from $repo/releases." ;;
esac
case "$(uname -m)" in
  x86_64 | amd64) arch="x86_64" ;;
  aarch64 | arm64) arch="aarch64" ;;
  *) fail "There is no Khalid build for a $(uname -m) processor yet." "Build it from source with cargo, as CONTRIBUTING.md says." ;;
esac
platform="$os-$arch"
[ "$platform" != "macos-x86_64" ] || fail "There is no Khalid build for Intel Macs yet." "Build it from source with cargo, as CONTRIBUTING.md says."

command -v curl >/dev/null 2>&1 || fail "This script needs curl to download Khalid." "Install curl, then run it again."
if command -v sha256sum >/dev/null 2>&1; then
  sum() { sha256sum "$1" | cut -d ' ' -f 1; }
elif command -v shasum >/dev/null 2>&1; then
  sum() { shasum -a 256 "$1" | cut -d ' ' -f 1; }
else
  fail "This script needs sha256sum or shasum to check the download." "Install coreutils, then run it again."
fi

if [ -z "$version" ]; then
  # GitHub sends /releases/latest on to the newest release's own page.
  latest=$(curl -fsSLI -o /dev/null -w '%{url_effective}' "$repo/releases/latest") ||
    fail "GitHub did not say which release is the newest." "Check the connection, or name one with --version."
  version="${latest##*/}"
fi
case "$version" in
  v[0-9]*) ;;
  *) fail "$version is not a release name." "Release names look like v0.1.0." ;;
esac

if [ "$systemd" = "yes" ] && [ "$(id -u)" != "0" ]; then
  fail "--systemd sets up a service, which needs root." "Run the script with sudo."
fi
if [ -z "$dir" ]; then
  if [ "$(id -u)" = "0" ]; then dir="/usr/local/bin"; else dir="$HOME/.local/bin"; fi
fi

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
file="khalid-$version-$platform"
printf 'Downloading Khalid %s for %s.\n' "$version" "$platform"
curl -fsSL -o "$work/$file" "$releases/$version/$file" ||
  fail "The file $file could not be downloaded." "Check the release name at $repo/releases."
curl -fsSL -o "$work/SHA256SUMS" "$releases/$version/SHA256SUMS" ||
  fail "The release's SHA256SUMS could not be downloaded, so the file cannot be checked." "Try again in a minute."

expected=$(awk -v name="$file" '$2 == name || $2 == "*" name { print $1 }' "$work/SHA256SUMS")
[ -n "$expected" ] || fail "SHA256SUMS has no line for $file, so it cannot be checked." "Report this at $repo/issues."
[ "$(sum "$work/$file")" = "$expected" ] ||
  fail "The download does not match its checksum, so it may be damaged or changed." "Nothing was written. Run the script again."

mkdir -p "$dir"
chmod 0755 "$work/$file"
# Copied beside the old binary first, so the swap is one rename.
cp "$work/$file" "$dir/.khalid.new"
mv -f "$dir/.khalid.new" "$dir/khalid"
printf 'Installed Khalid %s at %s/khalid.\n' "$version" "$dir"

if [ "$systemd" = "yes" ]; then
  if ! curl -fsSL -o "$work/khalid.service" "$unit_source/$version/packaging/systemd/khalid.service" ||
    ! curl -fsSL -o "$work/khalid.sysusers" "$unit_source/$version/packaging/systemd/khalid.sysusers"; then
    printf 'The khalid service was not set up. Its files for %s could not be downloaded.\n' "$version" >&2
    printf 'The binary is installed. Copy the files from packaging/systemd/ by hand, as docs/deploy.md says.\n' >&2
    exit 1
  fi
  sed "s#/usr/local/bin/khalid#$dir/khalid#" "$work/khalid.service" >/etc/systemd/system/khalid.service
  cp "$work/khalid.sysusers" /etc/sysusers.d/khalid.conf
  systemd-sysusers
  install -d -o khalid -g khalid -m 0750 /var/lib/khalid
  systemctl daemon-reload
  printf 'Installed the khalid service. Make a site, then start it:\n'
  printf '  sudo -u khalid %s/khalid init /var/lib/khalid --address https://blog.example.com --yes\n' "$dir"
  printf '  sudo systemctl enable --now khalid\n'
  exit 0
fi

case ":$PATH:" in
  *":$dir:"*) printf 'Run khalid init to make a site.\n' ;;
  *) printf '%s is not on your PATH. Add it, or run %s/khalid init to make a site.\n' "$dir" "$dir" ;;
esac
