//! Redirect lists as CSV, with the columns `from,to,status,match`, so a
//! site's redirects can be brought in from an importer or a spreadsheet and
//! taken out again.
//!
//! `from` is a path, or a regular expression when `match` is `regex`. `to`
//! is a path or a web address, empty for 410 and 451; a pattern's `to` may
//! use its groups as `$1`. `match` may be left out, and means `exact`.

use std::fmt::Write as _;
use std::iter::Peekable;
use std::str::Chars;

use khalid_core::AbsoluteUrl;

use crate::{
    Destination, Move, PatternRedirect, PatternRule, Redirect, RedirectEntry, RedirectPattern,
    RedirectRule, SeoError, UrlPath,
};

/// The header a list starts with, and the one it is written with.
const HEADER: &str = "from,to,status,match";

/// One row as read: the line it starts on and its fields.
type Row = (usize, Vec<String>);

/// Reads a redirect list, refusing the whole list at its first bad row so
/// an import either brings in every rule or none.
///
/// # Errors
///
/// [`SeoError::RedirectCsvRow`] for a missing header, a row with the wrong
/// number of columns, an unknown status or match, a move with no `to` or a
/// 410 or 451 with one, or a quote never closed. [`SeoError::RedirectCsvValue`]
/// for a path, address or pattern that is refused, with the reason as its
/// source.
pub fn read_redirects_csv(text: &str) -> Result<Vec<RedirectEntry>, SeoError> {
    let mut rows = rows(text)?.into_iter();
    let not_header = "is not the header from,to,status,match";
    match rows.next() {
        Some((_, fields)) if is_header(&fields) => {}
        Some((line, _)) => return Err(row_problem(line, not_header)),
        None => return Err(row_problem(1, not_header)),
    }
    rows.map(|(line, fields)| entry(line, &fields)).collect()
}

/// Writes `entries` as a redirect list, header first, one row each, with
/// `match` always given.
pub fn write_redirects_csv(entries: &[RedirectEntry]) -> String {
    let mut csv = format!("{HEADER}\n");
    for entry in entries {
        let (from, to, status, matching) = match entry {
            RedirectEntry::Exact(rule) => {
                let to = rule.redirect.location().map(ToString::to_string);
                let (from, status) = (rule.from.as_str(), rule.redirect.status());
                (from, to.unwrap_or_default(), status, "exact")
            }
            RedirectEntry::Pattern(rule) => {
                let (to, status) = match &rule.answer {
                    PatternRedirect::Move { kind, template } => (template.clone(), kind.status()),
                    PatternRedirect::Gone => (String::new(), 410),
                    PatternRedirect::LegalBlock => (String::new(), 451),
                };
                (rule.pattern.as_str(), to, status, "regex")
            }
        };
        let (from, to) = (field(from), field(&to));
        // Writing to a String cannot fail.
        let _ = writeln!(csv, "{from},{to},{status},{matching}");
    }
    csv
}

fn is_header(fields: &[String]) -> bool {
    let names: Vec<String> = fields.iter().map(|f| f.trim().to_lowercase()).collect();
    names == ["from", "to", "status"] || names == ["from", "to", "status", "match"]
}

fn row_problem(line: usize, problem: &'static str) -> SeoError {
    SeoError::RedirectCsvRow { line, problem }
}

fn value_problem(line: usize) -> impl Fn(SeoError) -> SeoError {
    move |source| SeoError::RedirectCsvValue {
        line,
        source: Box::new(source),
    }
}

/// The entry one row holds.
fn entry(line: usize, fields: &[String]) -> Result<RedirectEntry, SeoError> {
    let (from, to, status, matching) = match fields {
        [from, to, status] => (from, to, status, ""),
        [from, to, status, matching] => (from, to, status, matching.as_str()),
        [] | [_] | [_, _] => return Err(row_problem(line, "has fewer than three columns")),
        _ => return Err(row_problem(line, "has more than four columns")),
    };
    let (from, to) = (from.trim(), to.trim());
    let kind = match status.trim() {
        "301" => Some(Move::Permanent),
        "302" => Some(Move::Found),
        "307" => Some(Move::Temporary),
        "410" | "451" => None,
        _ => {
            let problem = "has a status other than 301, 302, 307, 410 or 451";
            return Err(row_problem(line, problem));
        }
    };
    let pattern = match matching.trim().to_lowercase().as_str() {
        "" | "exact" => false,
        "regex" => true,
        _ => return Err(row_problem(line, "has a match other than exact or regex")),
    };
    match (kind, to.is_empty()) {
        (Some(_), true) => {
            let problem = "moves to nowhere: a 301, 302 or 307 needs a to address";
            return Err(row_problem(line, problem));
        }
        (None, false) => {
            let problem = "has a to address, which a 410 or 451 cannot use";
            return Err(row_problem(line, problem));
        }
        _ => {}
    }
    let gone = status.trim() == "410";
    let refused = value_problem(line);
    if pattern {
        let answer = match kind {
            Some(kind) => PatternRedirect::move_to(kind, to).map_err(&refused)?,
            None if gone => PatternRedirect::Gone,
            None => PatternRedirect::LegalBlock,
        };
        let pattern = RedirectPattern::try_new(from).map_err(&refused)?;
        return Ok(RedirectEntry::Pattern(PatternRule::new(pattern, answer)));
    }
    let redirect = match kind {
        Some(kind) => Redirect::Move {
            kind,
            to: destination(to).map_err(&refused)?,
        },
        None if gone => Redirect::Gone,
        None => Redirect::LegalBlock,
    };
    let from = UrlPath::try_new(from).map_err(&refused)?;
    Ok(RedirectEntry::Exact(RedirectRule::new(from, redirect)))
}

/// A path on this site when `to` starts with `/`, otherwise a web address.
fn destination(to: &str) -> Result<Destination, SeoError> {
    if to.starts_with('/') {
        return Ok(Destination::Path(UrlPath::try_new(to)?));
    }
    AbsoluteUrl::parse(to)
        .map(Destination::External)
        .map_err(|source| SeoError::RedirectAddress {
            address: to.to_owned(),
            source,
        })
}

/// Splits `text` into rows of fields, quoted as RFC 4180 has it, leaving
/// out blank lines.
fn rows(text: &str) -> Result<Vec<Row>, SeoError> {
    let mut rows = Vec::new();
    let mut chars = text.chars().peekable();
    let mut line = 1;
    loop {
        let start = line;
        let Some(fields) = row(&mut chars, &mut line)? else {
            return Ok(rows);
        };
        let blank = matches!(fields.as_slice(), [only] if only.trim().is_empty());
        if !blank {
            rows.push((start, fields));
        }
    }
}

/// Reads one row and its line break, or `None` at the end of the text.
fn row(chars: &mut Peekable<Chars<'_>>, line: &mut usize) -> Result<Option<Vec<String>>, SeoError> {
    if chars.peek().is_none() {
        return Ok(None);
    }
    let start = *line;
    let mut fields = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    while let Some(c) = chars.next() {
        match (quoted, c) {
            (true, '"') if chars.peek() == Some(&'"') => {
                chars.next();
                field.push('"');
            }
            (true, '"') => quoted = false,
            (false, '"') if field.is_empty() => quoted = true,
            (false, ',') => fields.push(std::mem::take(&mut field)),
            (false, '\r') if chars.peek() == Some(&'\n') => {}
            (false, '\n') => {
                *line += 1;
                fields.push(field);
                return Ok(Some(fields));
            }
            (_, c) => {
                if c == '\n' {
                    *line += 1;
                }
                field.push(c);
            }
        }
    }
    if quoted {
        return Err(row_problem(start, "opens a quote it never closes"));
    }
    fields.push(field);
    Ok(Some(fields))
}

/// A CSV field, quoted when it holds a comma, a quote or a line break.
fn field(text: &str) -> String {
    if text.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", text.replace('"', "\"\""))
    } else {
        text.to_owned()
    }
}
