//! Dates written the way the page's language writes them.

use chrono::{Datelike, NaiveDate};
use minijinja::value::Value;
use minijinja::{Error, ErrorKind, State};

/// The template filter `date`: turns `2026-09-14` into `14 September 2026`
/// in English and `14. September 2026` in German. The month names and the
/// order come from the page's strings file (`month_1` to `month_12` and
/// `date_format`), so a new language needs no code.
pub(crate) fn format_date(state: &State<'_, '_>, iso: &str) -> Result<String, Error> {
    let date = NaiveDate::parse_from_str(iso, "%Y-%m-%d").map_err(|error| {
        Error::new(ErrorKind::InvalidOperation, "date needs YYYY-MM-DD").with_source(error)
    })?;
    let strings = state.lookup("t").unwrap_or_default();
    let month = string(&strings, &format!("month_{}", date.month()))?;
    Ok(string(&strings, "date_format")?
        .replace("{day}", &date.day().to_string())
        .replace("{month}", &month)
        .replace("{year}", &date.year().to_string()))
}

fn string(strings: &Value, key: &str) -> Result<String, Error> {
    let value = strings.get_attr(key)?;
    value.as_str().map(str::to_owned).ok_or_else(|| {
        Error::new(
            ErrorKind::UndefinedError,
            format!("the strings table has no {key}"),
        )
    })
}
