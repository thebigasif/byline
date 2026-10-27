//! Embeds the admin that `npm run build` left in `admin/dist/`. When it has
//! not run, the list is empty and `/admin/` says how to build it.

use std::ffi::OsString;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::{fs, io};

fn main() -> io::Result<()> {
    let mut dist = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dist.pop();
    dist.pop();
    dist.push("admin");
    dist.push("dist");
    println!("cargo::rerun-if-changed={}", dist.display());

    let mut files = Vec::new();
    if dist.is_dir() {
        collect(&dist, "", &mut files)?;
    }
    files.sort();
    let mut code = String::from("pub(crate) static FILES: &[crate::Asset] = &[\n");
    for (path, full) in &files {
        writeln!(
            code,
            "    crate::Asset {{ path: {path:?}, body: include_bytes!({full:?}) }},"
        )
        .map_err(io::Error::other)?;
    }
    code.push_str("];\n");

    let mut out = PathBuf::from(out_dir()?);
    out.push("admin_files.rs");
    fs::write(out, code)
}

/// Every file under `dir`, as its path under `admin/dist/` and its full path.
fn collect(dir: &Path, prefix: &str, files: &mut Vec<(String, String)>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = format!("{prefix}{name}");
        if entry.file_type()?.is_dir() {
            collect(&entry.path(), &format!("{path}/"), files)?;
        } else {
            files.push((path, entry.path().to_string_lossy().into_owned()));
        }
    }
    Ok(())
}

/// Where cargo wants this script's output. Cargo hands it over only as an
/// environment variable, and a build script cannot read `Config`, so this
/// is the one setting read here (BL-RUN-2 is about the running program).
fn out_dir() -> io::Result<OsString> {
    std::env::vars_os()
        .find_map(|(name, value)| (name == "OUT_DIR").then_some(value))
        .ok_or_else(|| io::Error::other("cargo did not set OUT_DIR"))
}
