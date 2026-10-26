//! The public site's one stylesheet, built from the theme's tokens.

use crate::{Asset, RenderError, Tokens};

/// The rules every site shares. They read the tokens through CSS variables.
const SITE_CSS: &str = include_str!("../assets/site.css");

/// Builds the one public stylesheet (RULES.md BL-WEB-7): the tokens as
/// CSS variables, then the shared rules with comments and indentation
/// removed, as a hashed asset with its compressed copies.
///
/// # Errors
///
/// Returns [`RenderError::Compress`] if a compressor fails.
pub fn stylesheet(tokens: &Tokens) -> Result<Asset, RenderError> {
    let mut css = tokens.to_css();
    css.push_str(&minify(SITE_CSS));
    Asset::build("site", "css", "text/css; charset=utf-8", css.into_bytes())
}

/// Removes comments, indentation and blank lines. The rules themselves are
/// left as written, so the result reads the same in a browser's inspector.
fn minify(css: &str) -> String {
    let mut out = String::with_capacity(css.len());
    let mut rest = css;
    while let Some(start) = rest.find("/*") {
        push_lines(&mut out, rest.get(..start).unwrap_or_default());
        rest = rest
            .get(start..)
            .and_then(|from| {
                from.find("*/")
                    .and_then(|end| from.get(end.saturating_add(2)..))
            })
            .unwrap_or_default();
    }
    push_lines(&mut out, rest);
    out
}

fn push_lines(out: &mut String, text: &str) {
    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        out.push_str(line);
        out.push('\n');
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comments_indentation_and_blank_lines_go() {
        let css = "/* a note */\nbody {\n  color: red; /* why */\n\n}\n";
        assert_eq!(minify(css), "body {\ncolor: red;\n}\n");
    }

    #[test]
    fn an_unclosed_comment_drops_the_rest() {
        assert_eq!(minify("a { b: c; }\n/* never closed"), "a { b: c; }\n");
    }
}
