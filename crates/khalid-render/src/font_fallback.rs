//! Tuned fallbacks for web fonts (RULES.md BL-WEB-5): while a token font
//! loads, or where it is not installed, text is drawn in a local font
//! resized to take the same space, so the page does not shift when the
//! font arrives.

use std::fmt::Write as _;

/// A web font and the local font that stands in for it, with the overrides
/// that make the stand-in match its size. The values follow the formula
/// Next.js and Capsize use: `size-adjust` is the ratio of the two fonts'
/// average letter widths, and each override is the web font's metric over
/// its units per em, divided by `size-adjust`.
struct Fallback {
    family: &'static str,
    local: &'static str,
    ascent: &'static str,
    descent: &'static str,
    line_gap: &'static str,
    size_adjust: &'static str,
}

/// The web fonts whose metrics are checked. Inter: 2816 units per em,
/// ascent 2728, descent 680. Roboto: 2048 units per em, ascent 1900,
/// descent 500. A font missing here keeps its stack as written.
const KNOWN: [Fallback; 2] = [
    Fallback {
        family: "Inter",
        local: "Arial",
        ascent: "90.44%",
        descent: "22.52%",
        line_gap: "0%",
        size_adjust: "107.12%",
    },
    Fallback {
        family: "Roboto",
        local: "Arial",
        ascent: "92.49%",
        descent: "24.34%",
        line_gap: "0%",
        size_adjust: "100.3%",
    },
];

/// `stack` with the tuned fallback of its first font after it, such as
/// `Inter, "Inter Fallback", sans-serif`, and that font's name. A stack
/// whose first font is not known is returned as it is.
pub(crate) fn tuned(stack: &str) -> (String, Option<&'static str>) {
    let (first, rest) = stack.split_once(',').unwrap_or((stack, ""));
    let name = first.trim().trim_matches('"');
    match KNOWN
        .iter()
        .find(|font| font.family.eq_ignore_ascii_case(name))
    {
        Some(font) => {
            let mut tuned = format!("{}, \"{} Fallback\"", first.trim(), font.family);
            if !rest.trim().is_empty() {
                let _ = write!(tuned, ", {}", rest.trim());
            }
            (tuned, Some(font.family))
        }
        None => (stack.to_owned(), None),
    }
}

/// The `@font-face` rule of the fallback for `family`, one line, or nothing
/// for a font that is not known.
pub(crate) fn face(family: &str) -> String {
    KNOWN
        .iter()
        .find(|font| font.family == family)
        .map_or_else(String::new, |font| {
            format!(
                "@font-face {{ font-family: \"{} Fallback\"; src: local(\"{}\"); \
                 ascent-override: {}; descent-override: {}; \
                 line-gap-override: {}; size-adjust: {}; }}\n",
                font.family, font.local, font.ascent, font.descent, font.line_gap, font.size_adjust
            )
        })
}
