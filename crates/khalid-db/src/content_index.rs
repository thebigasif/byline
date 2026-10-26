//! What a rebuild writes: the content index, as read from the files.

/// Everything a rebuild writes into the derived tables. The caller reads the
/// content files and fills this in; this crate never reads `content/`.
///
/// Posts join the index in the next part of task 1.10. Until then a rebuild
/// empties the site's derived tables and writes nothing back.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct ContentIndex {}
