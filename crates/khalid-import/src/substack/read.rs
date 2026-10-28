//! Reading a Substack export: `posts.csv`, one row per post, and the
//! `posts/` folder, one HTML file per post named after the row's
//! `post_id`, such as `140062712.why-rye.html`.

use chrono::DateTime;

use crate::csv::{CsvError, rows};
use crate::substack::{Audience, Export, Post, PostKind, UnreadRow};

/// Why [`read_export`] refused an export.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ExportError {
    /// `posts.csv` is not CSV.
    #[error("posts.csv cannot be read: {0}")]
    Csv(#[from] CsvError),
    /// Its first row lacks a column Khalid needs, often because another
    /// file of the export was given, such as a list of subscribers.
    #[error("posts.csv has no {0} column, so it is not the post list of a Substack export")]
    Column(&'static str),
}

/// Why one row of `posts.csv` was not read as a post.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum RowError {
    /// Its `post_id` is not a number, a dot and a slug.
    #[error("its post_id {0:?} is not a number, a dot and a slug")]
    Id(String),
    /// Its `post_date` is not a time Substack writes.
    #[error("its post_date {0:?} is not a date")]
    Date(String),
    /// It has fewer fields than the header.
    #[error("it has fewer fields than the header")]
    Short,
}

/// The columns of `posts.csv` Khalid reads, in the order [`post`] takes
/// their values.
const COLUMNS: [&str; 7] = [
    "post_id",
    "post_date",
    "is_published",
    "type",
    "audience",
    "title",
    "subtitle",
];

/// Reads a Substack export from its `posts.csv` and the files of its
/// `posts/` folder, as names and texts. Posts come in the list's order. A
/// row that cannot be read goes into [`Export::unread`] with the reason. A
/// file no row names, such as a post's open counts, is not read.
///
/// # Errors
///
/// Returns [`ExportError::Csv`] when `posts.csv` is not CSV, and
/// [`ExportError::Column`] when it lacks a column Khalid needs.
pub fn read_export<N, T>(
    posts_csv: &str,
    files: impl IntoIterator<Item = (N, T)>,
) -> Result<Export, ExportError>
where
    N: AsRef<str>,
    T: AsRef<str>,
{
    let mut rows = rows(posts_csv)?.into_iter();
    let header = rows.next().map(|(_, fields)| fields).unwrap_or_default();
    let columns = COLUMNS
        .iter()
        .map(|name| {
            let found = header.iter().position(|field| field.trim() == *name);
            found.ok_or(ExportError::Column(name))
        })
        .collect::<Result<Vec<usize>, _>>()?;
    let files: Vec<(N, T)> = files.into_iter().collect();
    let body = |id: &str| {
        let name = format!("{id}.html");
        files
            .iter()
            .find(|(file, _)| file.as_ref() == name)
            .map(|(_, text)| text.as_ref().to_owned())
    };
    let mut export = Export::default();
    for (line, fields) in rows {
        let field = |index: usize| fields.get(index).map(|field| field.trim());
        let values: Option<Vec<&str>> = columns.iter().map(|index| field(*index)).collect();
        let read = values
            .ok_or(RowError::Short)
            .and_then(|values| post(&values, &body));
        match read {
            Ok(post) => export.posts.push(post),
            Err(error) => export.unread.push(UnreadRow { line, error }),
        }
    }
    Ok(export)
}

/// The post one row's `values` hold, in the order of the columns above,
/// with its body from `body`.
fn post(values: &[&str], body: &impl Fn(&str) -> Option<String>) -> Result<Post, RowError> {
    let [id, date, published, kind, audience, title, subtitle] = values else {
        return Err(RowError::Short);
    };
    let Some((number, slug)) = id
        .split_once('.')
        .filter(|(number, slug)| !slug.is_empty() && number.parse::<u64>().is_ok())
    else {
        return Err(RowError::Id((*id).to_owned()));
    };
    // Substack writes times such as `2024-03-12T09:30:44.123Z`, in UTC.
    let date = match *date {
        "" => None,
        written => Some(
            DateTime::parse_from_rfc3339(written)
                .map_err(|_| RowError::Date(written.to_owned()))?
                .naive_utc(),
        ),
    };
    Ok(Post {
        id: number.to_owned(),
        slug: slug.to_owned(),
        title: (*title).to_owned(),
        subtitle: Some((*subtitle).to_owned()).filter(|s| !s.is_empty()),
        date,
        published: *published == "true",
        kind: PostKind::named(kind),
        audience: Audience::named(audience),
        body: body(id),
    })
}
