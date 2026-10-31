//! `khalid db export`: every table of the site as one CSV and one JSON
//! file each, in `data/export/` or the folder `--out` names, so a
//! spreadsheet or Looker Studio can read them (KH-GRO-6). Sign-in tables
//! and secret columns stay out (`khalid_db::SiteExport`).

use std::fs;
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

use khalid_core::{Slug, UserMessage};
use khalid_db::{Cell, Database, DbError, ExportTable, SiteExport};
use serde::Serialize;
use serde_json::{Map, Value};
use tempfile::NamedTempFile;

use crate::cli::Output;
use crate::db::{DATA, MAIN};
use crate::exit::Exit;
use crate::{db, db_message, report};

/// The folder in `data/` the export goes to unless `--out` names another.
const FOLDER: &str = "export";

/// What stopped an export.
enum Failed {
    /// The database could not be read.
    Db(DbError),
    /// A file could not be written.
    Write(PathBuf, io::Error),
}

impl From<DbError> for Failed {
    fn from(error: DbError) -> Self {
        Self::Db(error)
    }
}

/// One table as `--json` reports it.
#[derive(Serialize)]
struct Written {
    name: String,
    rows: u64,
}

/// Exports the tables of the `site` folder's database into `out`, or
/// `data/export/` in the site folder.
pub(crate) fn run(site: &Path, out: Option<&Path>, output: Output) -> Exit {
    let folder = out.map_or_else(|| default_folder(site), Path::to_path_buf);
    if let Err(error) = fs::create_dir_all(&folder) {
        return report::failure(output, &db_message::folder(&folder, &error), Exit::Failure);
    }
    let path = match db::data_file(site) {
        Ok(path) => path,
        Err(message) => return report::failure(output, &message, Exit::Failure),
    };
    let (Ok(main), Ok(runtime)) = (
        Slug::try_new(MAIN),
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build(),
    ) else {
        return report::failure(output, &db_message::internal(), Exit::Internal);
    };
    match runtime.block_on(export(&path, &main, &folder)) {
        Ok(written) => success(output, &folder, &written),
        Err(Failed::Db(error)) => {
            report::failure(output, &db_message::of(&path, &error), Exit::Failure)
        }
        Err(Failed::Write(file, error)) => {
            report::failure(output, &write_failed(&file, &error), Exit::Failure)
        }
    }
}

/// `data/export/` in the site folder.
fn default_folder(site: &Path) -> PathBuf {
    let mut folder = site.to_path_buf();
    folder.push(DATA);
    folder.push(FOLDER);
    folder
}

/// Opens the database, brings it up to date, and writes each table into
/// `folder`.
async fn export(path: &Path, handle: &Slug, folder: &Path) -> Result<Vec<Written>, Failed> {
    let db = Database::open(path).await?;
    let done = async {
        db.migrate().await?;
        let site = db.site(handle).await?;
        let export = SiteExport::new(db.clone(), site);
        let mut written = Vec::new();
        for table in export.tables().await? {
            let rows = write_table(&export, &table, folder).await?;
            let name = table.name().to_owned();
            written.push(Written { name, rows });
        }
        Ok(written)
    }
    .await;
    db.close().await;
    done
}

/// Writes `table` as `<name>.csv` and `<name>.json` in `folder`, each to
/// a temporary file first, so a reader never finds half a file. Returns
/// how many rows it holds.
async fn write_table(
    export: &SiteExport,
    table: &ExportTable,
    folder: &Path,
) -> Result<u64, Failed> {
    let target = |extension: &str| {
        let mut path = folder.to_path_buf();
        path.push(format!("{}.{extension}", table.name()));
        path
    };
    let (csv_path, json_path) = (target("csv"), target("json"));
    let open = |path: &Path| {
        NamedTempFile::new_in(folder)
            .map(BufWriter::new)
            .map_err(|error| Failed::Write(path.to_path_buf(), error))
    };
    let (mut csv, mut json) = (open(&csv_path)?, open(&json_path)?);
    let columns = table.columns();
    let header: Vec<String> = columns.iter().map(|column| field(column)).collect();
    let started = writeln!(csv, "{}", header.join(",")).and_then(|()| json.write_all(b"["));
    started.map_err(|error| Failed::Write(csv_path.clone(), error))?;
    let mut rows: u64 = 0;
    export
        .each_row(table, |cells| {
            let line: Vec<String> = cells.iter().map(csv_field).collect();
            writeln!(csv, "{}", line.join(","))
                .map_err(|error| Failed::Write(csv_path.clone(), error))?;
            let object: Map<String, Value> = columns
                .iter()
                .cloned()
                .zip(cells.iter().map(value))
                .collect();
            let comma = if rows == 0 { "\n" } else { ",\n" };
            json.write_all(comma.as_bytes())
                .and_then(|()| serde_json::to_writer(&mut json, &object).map_err(io::Error::from))
                .map_err(|error| Failed::Write(json_path.clone(), error))?;
            rows = rows.saturating_add(1);
            Ok::<_, Failed>(())
        })
        .await?;
    let end: &[u8] = if rows == 0 { b"]\n" } else { b"\n]\n" };
    json.write_all(end)
        .map_err(|error| Failed::Write(json_path.clone(), error))?;
    persist(csv, &csv_path)?;
    persist(json, &json_path)?;
    Ok(rows)
}

/// Flushes `file` to disk and renames it to `path`, over an older export.
fn persist(file: BufWriter<NamedTempFile>, path: &Path) -> Result<(), Failed> {
    let failed = |error: io::Error| Failed::Write(path.to_path_buf(), error);
    let file = file
        .into_inner()
        .map_err(|error| failed(error.into_error()))?;
    file.as_file().sync_all().map_err(failed)?;
    file.persist(path).map_err(|error| failed(error.error))?;
    Ok(())
}

/// `cell` as a JSON value: bytes as hex text.
fn value(cell: &Cell) -> Value {
    match cell {
        Cell::Null => Value::Null,
        Cell::Integer(number) => Value::from(*number),
        Cell::Real(number) => Value::from(*number),
        _ => Value::String(cell.text()),
    }
}

/// `cell` as one CSV field. Only text can look like a formula; a number
/// stays as it is, so a spreadsheet reads it as a number.
fn csv_field(cell: &Cell) -> String {
    match cell {
        Cell::Text(text) => field(text),
        _ => cell.text(),
    }
}

/// `text` as one CSV field: in quotes, with quotes doubled, when it holds
/// a comma, a quote or a line break. A field a spreadsheet would read as a
/// formula gets a leading `'`, so opening the file runs nothing.
fn field(text: &str) -> String {
    let text = if text.starts_with(['=', '+', '-', '@']) {
        format!("'{text}")
    } else {
        text.to_owned()
    };
    if text.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", text.replace('"', "\"\""))
    } else {
        text
    }
}

/// Says where the files went.
fn success(output: Output, folder: &Path, written: &[Written]) -> Exit {
    let shown = folder.display().to_string();
    match output {
        Output::Human => report::text(
            &format!(
                "Wrote {} tables to {shown}, each as a CSV file and a JSON file.\n",
                written.len()
            ),
            Exit::Success,
        ),
        Output::Json => {
            let body = serde_json::json!({ "folder": shown, "tables": written });
            report::json(&body, Exit::Success)
        }
    }
}

/// The message when an export file cannot be written.
fn write_failed(file: &Path, error: &io::Error) -> UserMessage {
    let file = file.display();
    let why = match error.kind() {
        io::ErrorKind::PermissionDenied => "You do not have permission to write there.",
        io::ErrorKind::StorageFull => "The disk is full.",
        _ => "Something on the computer stopped the write.",
    };
    UserMessage::new(
        format!("Khalid could not write the export file {file}."),
        why,
        "Free some space or fix the folder's permissions, then run `khalid db export` again.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_field_is_quoted_only_when_it_must_be_and_never_runs_as_a_formula() {
        assert_eq!(field("plain"), "plain");
        assert_eq!(field("a,b"), "\"a,b\"");
        assert_eq!(field("say \"hi\""), "\"say \"\"hi\"\"\"");
        assert_eq!(field("=SUM(A1)"), "'=SUM(A1)");
        assert_eq!(field("-1"), "'-1");
        assert_eq!(csv_field(&Cell::Integer(-1)), "-1");
    }
}
