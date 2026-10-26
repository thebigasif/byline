//! Permalink patterns such as `/news/{year}/{slug}/`.

/// A collection's address pattern, relative to the site's mount path and
/// language prefix. It starts and ends with `/`, holds `{slug}` exactly once,
/// and may hold `{year}`, `{month}`, `{day}` and `{collection}` once each.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Permalink(String);

/// The placeholders a pattern may use, as written.
const PLACEHOLDERS: [&str; 5] = ["{slug}", "{year}", "{month}", "{day}", "{collection}"];

impl Permalink {
    /// Checks a pattern, or says in plain words which rule it breaks.
    pub(crate) fn parse(text: &str) -> Result<Self, String> {
        folder_shape(text)?;
        let mut seen: Vec<&str> = Vec::new();
        let mut rest = text;
        while let Some(open) = rest.find('{') {
            let (before, from) = rest.split_at(open);
            plain(before)?;
            let close = from.find('}').ok_or("a `{` is never closed with `}`")?;
            let (name, after) = from.split_at(close.saturating_add(1));
            if !PLACEHOLDERS.contains(&name) {
                return Err(format!(
                    "`{name}` is not a placeholder; use {{slug}}, {{year}}, {{month}}, {{day}} or {{collection}}"
                ));
            }
            if seen.contains(&name) {
                return Err(format!("`{name}` appears twice; use each placeholder once"));
            }
            seen.push(name);
            rest = after;
        }
        plain(rest)?;
        if !seen.contains(&"{slug}") {
            return Err(
                "the permalink must hold {slug}, so every post has its own address".to_owned(),
            );
        }
        Ok(Self(text.to_owned()))
    }

    /// The pattern as written.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for Permalink {
    /// `/{slug}/`: a post sits right under the mount path.
    fn default() -> Self {
        Self("/{slug}/".to_owned())
    }
}

/// Checks a path that is a folder: it starts and ends with `/` and has no
/// empty folder in it.
pub(crate) fn folder_shape(text: &str) -> Result<(), String> {
    if !text.starts_with('/') || !text.ends_with('/') || text.contains("//") {
        return Err("it must start and end with /, like /news/".to_owned());
    }
    Ok(())
}

/// Checks the fixed text between placeholders: lowercase letters, digits,
/// hyphens and slashes, which every server and browser treat alike.
pub(crate) fn plain(text: &str) -> Result<(), String> {
    match text
        .chars()
        .find(|&c| !(c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '/'))
    {
        Some(c) => Err(format!(
            "`{c}` is not allowed; use lowercase letters, digits, hyphens and slashes"
        )),
        None => Ok(()),
    }
}
