//! Why a `WordPress` export file could not be read.

/// Why [`read_wxr`](crate::wordpress::read_wxr) refused a file.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum WxrError {
    /// The file is not well-formed XML, often because the export stopped
    /// part of the way through.
    #[error("the file is not complete XML: {source}")]
    NotXml {
        /// What the XML reader found.
        #[source]
        source: roxmltree::Error,
    },
    /// The file is XML, but not an export from Tools, Export in `WordPress`.
    #[error("the file is not a WordPress export: it has no wp:wxr_version in its channel")]
    NotWordPress,
    /// A field holds a value of the wrong kind, such as a date that is not a
    /// date.
    #[error("line {line}: {field} holds {value:?}, which is not what WordPress writes there")]
    BadValue {
        /// The file's line, counting from 1.
        line: usize,
        /// The field, such as `wp:post_date`.
        field: String,
        /// The value as written.
        value: String,
    },
}
