//! HTML escaping that leaves slashes alone, so addresses stay readable and short.

use minijinja::value::Value;
use minijinja::{AutoEscape, Error, ErrorKind, Output, State, escape_formatter};

/// Writes a value into the page. Text that is not marked safe has `&`, `<`,
/// `>`, `"` and `'` escaped, which is all HTML needs in text and in quoted
/// attributes. `MiniJinja`'s own escaper also turns every `/` into `&#x2f;`,
/// which is valid but makes every address longer.
pub(crate) fn format(
    out: &mut Output<'_>,
    state: &mut State<'_, '_>,
    value: &Value,
) -> Result<(), Error> {
    if let (AutoEscape::Html, false, Some(text)) =
        (state.auto_escape(), value.is_safe(), value.as_str())
    {
        return write_escaped(out, text)
            .map_err(|_| Error::new(ErrorKind::WriteFailure, "could not write the page"));
    }
    escape_formatter(out, state, value)
}

fn write_escaped(out: &mut Output<'_>, text: &str) -> std::fmt::Result {
    let mut rest = text;
    while let Some(index) = rest.find(['&', '<', '>', '"', '\'']) {
        let (plain, special) = rest.split_at(index);
        out.write_str(plain)?;
        let mut chars = special.chars();
        let entity = match chars.next() {
            Some('&') => "&amp;",
            Some('<') => "&lt;",
            Some('>') => "&gt;",
            Some('"') => "&quot;",
            _ => "&#39;",
        };
        out.write_str(entity)?;
        rest = chars.as_str();
    }
    out.write_str(rest)
}

#[cfg(test)]
mod tests {
    use minijinja::{Environment, context};

    use super::*;

    fn render(template: &str, value: &str) -> String {
        let mut env = Environment::new();
        env.set_auto_escape_callback(|_| AutoEscape::Html);
        env.set_formatter(format);
        env.add_template("page.html", template).unwrap();
        let page = env.get_template("page.html").unwrap();
        page.render(context! { value }).unwrap()
    }

    #[test]
    fn the_five_html_characters_are_escaped() {
        let got = render(
            "<p title=\"{{ value }}\">{{ value }}</p>",
            r#"<b>"Tom" & 'Jerry'</b>"#,
        );
        let escaped = "&lt;b&gt;&quot;Tom&quot; &amp; &#39;Jerry&#39;&lt;/b&gt;";
        assert_eq!(got, format!("<p title=\"{escaped}\">{escaped}</p>"));
    }

    #[test]
    fn addresses_keep_their_slashes() {
        let got = render(
            "<a href=\"{{ value }}\">x</a>",
            "https://www.example.com/blog/",
        );
        assert_eq!(got, "<a href=\"https://www.example.com/blog/\">x</a>");
    }

    #[test]
    fn text_marked_safe_is_written_as_it_is() {
        let got = render("{{ value | safe }}", "<em>sanitised</em>");
        assert_eq!(got, "<em>sanitised</em>");
    }

    #[test]
    fn numbers_and_text_without_special_characters_pass_through() {
        assert_eq!(
            render("{{ 1200 }} {{ value }}", "Straße über"),
            "1200 Straße über"
        );
    }
}
