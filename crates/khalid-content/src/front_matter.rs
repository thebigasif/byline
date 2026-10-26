//! Splitting a file into its front matter and its body.

/// A file's front matter and body, as text.
pub(crate) struct Split<'text> {
    /// The YAML between the two `---` lines.
    pub(crate) yaml: &'text str,
    /// Everything after the closing `---` line.
    pub(crate) body: &'text str,
}

/// Why a file has no usable front matter fence.
pub(crate) enum Fence {
    /// The file does not start with `---`.
    Missing,
    /// The opening `---` has no closing `---`.
    Unclosed,
}

/// Splits `text` at its front matter fence. The YAML starts on the file's
/// second line, which callers use to turn YAML lines into file lines.
pub(crate) fn split(text: &str) -> Result<Split<'_>, Fence> {
    let rest = text
        .strip_prefix("---\n")
        .or_else(|| text.strip_prefix("---\r\n"))
        .ok_or(Fence::Missing)?;
    let mut offset = 0;
    for line in rest.split_inclusive('\n') {
        if line.trim_end_matches(['\n', '\r']) == "---" {
            let yaml = rest.get(..offset).unwrap_or_default();
            let body = rest
                .get(offset.saturating_add(line.len())..)
                .unwrap_or_default();
            return Ok(Split { yaml, body });
        }
        offset = offset.saturating_add(line.len());
    }
    Err(Fence::Unclosed)
}
