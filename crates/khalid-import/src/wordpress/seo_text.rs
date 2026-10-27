//! Filling in the variables SEO plugins write in titles and descriptions,
//! such as `%%title%% %%sep%% %%sitename%%`.

/// How a plugin marks a variable.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Syntax {
    /// `%%title%%`, as Yoast writes it.
    Double,
}

/// The values the variables stand for in one post.
#[derive(Debug)]
pub(crate) struct Vars<'a> {
    /// The post's headline.
    pub(crate) title: &'a str,
    /// The site's name.
    pub(crate) site: &'a str,
    /// The post's hand-written summary.
    pub(crate) excerpt: &'a str,
}

impl Vars<'_> {
    fn value(&self, name: &str) -> Option<&str> {
        match name {
            "title" | "post_title" => Some(self.title),
            "sitename" | "sitetitle" | "site_title" => Some(self.site),
            // The separator is a site setting the export leaves out.
            "sep" | "separator_sa" => Some("-"),
            "excerpt" | "excerpt_only" | "post_excerpt" => Some(self.excerpt),
            // A post's page number means nothing on its first page.
            "page" | "pagenumber" | "pagetotal" => Some(""),
            _ => None,
        }
    }
}

/// `text` with every variable filled in and spaces collapsed, or none when
/// it uses a variable Khalid cannot fill, such as the current year. The
/// text may be empty when every variable it held was empty.
pub(crate) fn fill(text: &str, syntax: Syntax, vars: &Vars<'_>) -> Option<String> {
    let mark = match syntax {
        Syntax::Double => "%%",
    };
    let mut filled = String::with_capacity(text.len());
    let mut rest = text;
    while let Some((before, tail)) = rest.split_once(mark) {
        filled.push_str(before);
        match tail.split_once(mark) {
            Some((name, after)) if is_name(name) => {
                filled.push_str(vars.value(name)?);
                rest = after;
            }
            // A mark that starts no variable, such as "50% off", is text.
            _ => {
                filled.push_str(mark);
                rest = tail;
            }
        }
    }
    filled.push_str(rest);
    let words: Vec<&str> = filled.split_whitespace().collect();
    Some(words.join(" "))
}

fn is_name(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}
