//! Where in a TOML file a mistake is, so every settings file reports it the
//! same way: the file, the 1-based line and the field (RULES.md BL-FS-3).
//!
//! `site.toml` and `routes.toml` both use these, so their messages agree.

/// The 1-based line a byte offset in `text` falls on.
///
/// # Examples
///
/// ```
/// use khalid_core::toml_location::line_of;
///
/// assert_eq!(line_of("name = \"a\"\nmode = \"b\"\n", 12), 2);
/// ```
pub fn line_of(text: &str, offset: usize) -> usize {
    text.bytes()
        .take(offset)
        .filter(|&b| b == b'\n')
        .count()
        .saturating_add(1)
}

/// The 1-based line where `key = ...` is written, or 1 when it is not.
pub fn line_of_key(text: &str, key: &str) -> usize {
    text.lines()
        .position(|line| {
            line.split_once('=')
                .is_some_and(|(left, _)| left.trim() == key)
        })
        .map_or(1, |index| index.saturating_add(1))
}

/// The 1-based line where `key = ...` is first written at or after the
/// line that opens `table`, such as `[mail`, or 1 when it is not.
pub fn line_of_key_in(text: &str, table: &str, key: &str) -> usize {
    text.lines()
        .enumerate()
        .skip_while(|(_, line)| !line.trim_start().starts_with(table))
        .find(|(_, line)| {
            line.split_once('=')
                .is_some_and(|(left, _)| left.trim() == key)
        })
        .map_or(1, |(index, _)| index.saturating_add(1))
}

/// The field a TOML parse error is about: the one its message names, or the
/// key on the line it points at, or empty when there is none.
pub fn field_of(text: &str, error: &toml::de::Error) -> String {
    let message = error.message();
    if let Some((_, rest)) = message.split_once("field `")
        && let Some((name, _)) = rest.split_once('`')
    {
        return name.to_owned();
    }
    let line = line_of(text, error.span().map_or(0, |span| span.start));
    text.lines()
        .nth(line.saturating_sub(1))
        .and_then(|line| line.split_once('='))
        .map_or_else(String::new, |(key, _)| key.trim().to_owned())
}
