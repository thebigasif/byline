//! Severities a site sets in `lint.toml`.

use std::collections::BTreeMap;

use serde::Deserialize;
use toml::Spanned;

use crate::{Catalogue, LintError, RuleId, Severity, Thresholds};

/// The settings from a site's `lint.toml`.
///
/// The `[rules]` table maps a rule ID to `"block"`, `"warn"` or `"off"`. A
/// rule the file does not name keeps its default. The `[thresholds]` table
/// tunes the numbers behind the rules that measure (see [`Thresholds`]).
///
/// # Examples
///
/// ```
/// use khalid_lint::{LintConfig, RuleId, Severity};
///
/// let config = LintConfig::parse("[rules]\nno-author = \"warn\"\n")?;
/// assert_eq!(config.severity(RuleId::new("no-author")), Some(Severity::Warn));
/// # Ok::<(), khalid_lint::LintError>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LintConfig {
    severities: BTreeMap<String, Severity>,
    thresholds: Thresholds,
}

/// The file as written, with the position of each rule name.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConfig {
    #[serde(default)]
    rules: BTreeMap<Spanned<String>, Severity>,
    #[serde(default)]
    thresholds: Thresholds,
}

impl LintConfig {
    /// Reads the text of a `lint.toml` file.
    ///
    /// # Errors
    ///
    /// Returns [`LintError::Syntax`] for text that is not TOML or holds a key
    /// or severity the linter does not know, and [`LintError::UnknownRule`]
    /// for a rule name that is not in the catalogue. Both name the line and
    /// column.
    pub fn parse(text: &str) -> Result<Self, LintError> {
        let raw: RawConfig = toml::from_str(text).map_err(|source| {
            let (line, column) = position(text, source.span().map_or(0, |span| span.start));
            LintError::Syntax {
                line,
                column,
                source,
            }
        })?;
        let catalogue = Catalogue::builtin()?;
        let mut severities = BTreeMap::new();
        for (name, severity) in raw.rules {
            if !catalogue.contains(name.get_ref()) {
                let (line, column) = position(text, name.span().start);
                return Err(LintError::UnknownRule {
                    rule: name.into_inner(),
                    line,
                    column,
                });
            }
            severities.insert(name.into_inner(), severity);
        }
        Ok(Self {
            severities,
            thresholds: raw.thresholds,
        })
    }

    /// The severity the file sets for `rule`, or `None` to keep its default.
    pub fn severity(&self, rule: RuleId) -> Option<Severity> {
        self.severities.get(rule.as_str()).copied()
    }

    /// The limits for the rules that measure, with defaults for any the file
    /// leaves out.
    pub fn thresholds(&self) -> Thresholds {
        self.thresholds
    }
}

/// The line and column, counted from 1, of the byte at `offset` in `text`.
fn position(text: &str, offset: usize) -> (usize, usize) {
    let before = text.get(..offset).unwrap_or(text);
    let line = before.matches('\n').count() + 1;
    let line_start = before.rfind('\n').map_or(0, |at| at + 1);
    let column = before.get(line_start..).unwrap_or_default().chars().count() + 1;
    (line, column)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn position_counts_lines_and_characters_from_one() {
        let text = "[rules]\nnö-date = 1\n";
        assert_eq!(position(text, 0), (1, 1));
        assert_eq!(position(text, 8), (2, 1));
        assert_eq!(position(text, 11), (2, 3), "ö is one character");
    }
}
