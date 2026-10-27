//! Reading a CSV file as rows of fields, the way spreadsheets and
//! Substack write it: commas between fields, double quotes around a field
//! that holds a comma, a quote or a line break, and a quote inside such a
//! field written twice.

/// Why a CSV file could not be read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum CsvError {
    /// A quoted field is never closed: the field's first line.
    #[error("a quote opened on line {0} is never closed")]
    Unclosed(usize),
}

/// One row: the line it starts on and its fields.
pub(crate) type Row = (usize, Vec<String>);

/// Every row of `text`, header included. A blank line is no row.
///
/// # Errors
///
/// Returns [`CsvError::Unclosed`] for a quoted field never closed.
pub(crate) fn rows(text: &str) -> Result<Vec<Row>, CsvError> {
    let mut rows = Vec::new();
    let mut fields = Vec::new();
    let mut field = String::new();
    let (mut line, mut start) = (1, 1);
    let mut quoted = false;
    let mut chars = text.trim_start_matches('\u{feff}').chars().peekable();
    while let Some(c) = chars.next() {
        match (quoted, c) {
            (true, '"') if chars.peek() == Some(&'"') => {
                field.push('"');
                chars.next();
            }
            (true, '"') => quoted = false,
            (false, '"') if field.is_empty() => quoted = true,
            (false, ',') => fields.push(std::mem::take(&mut field)),
            (false, '\r') => {}
            (false, '\n') => {
                fields.push(std::mem::take(&mut field));
                if fields.iter().any(|f| !f.is_empty()) {
                    rows.push((start, std::mem::take(&mut fields)));
                }
                line += 1;
                start = line;
            }
            (_, c) => {
                if c == '\n' {
                    line += 1;
                }
                field.push(c);
            }
        }
    }
    if quoted {
        return Err(CsvError::Unclosed(start));
    }
    fields.push(field);
    if fields.iter().any(|f| !f.is_empty()) {
        rows.push((start, fields));
    }
    Ok(rows)
}
