//! Reading the PHP-serialised arrays plugins store in custom fields, such as
//! Rank Math's `a:2:{i:0;s:7:"noindex";i:1;s:8:"nofollow";}`.

/// The key and value pairs of a serialised array of plain values, in order,
/// each as text. None when the text is not such an array.
pub(crate) fn pairs(text: &str) -> Option<Vec<(String, String)>> {
    let (_, body) = text.strip_prefix("a:")?.split_once(":{")?;
    let mut rest = body.strip_suffix('}')?;
    let mut values = Vec::new();
    while !rest.is_empty() {
        let (value, after) = scalar(rest)?;
        values.push(value);
        rest = after;
    }
    if values.len() % 2 != 0 {
        return None;
    }
    let mut values = values.into_iter();
    let mut pairs = Vec::new();
    while let (Some(key), Some(value)) = (values.next(), values.next()) {
        pairs.push((key, value));
    }
    Some(pairs)
}

/// One plain value, `s:7:"noindex";` or `i:0;`, and the text after it.
fn scalar(text: &str) -> Option<(String, &str)> {
    if let Some(rest) = text.strip_prefix("s:") {
        // The length counts bytes, so a quote inside the string is fine.
        let (len, rest) = rest.split_once(':')?;
        let len: usize = len.parse().ok()?;
        let rest = rest.strip_prefix('"')?;
        let value = rest.get(..len)?;
        let after = rest.get(len..)?.strip_prefix("\";")?;
        return Some((value.to_owned(), after));
    }
    let (typed, after) = text.split_once(';')?;
    let (kind, value) = typed.split_once(':')?;
    ["i", "b", "d"]
        .contains(&kind)
        .then(|| (value.to_owned(), after))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_list_reads_as_index_and_value() {
        let text = r#"a:2:{i:0;s:7:"noindex";i:1;s:8:"nofollow";}"#;
        let expected = [("0", "noindex"), ("1", "nofollow")];
        let expected: Vec<(String, String)> = expected
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();
        assert_eq!(pairs(text), Some(expected));
    }

    #[test]
    fn a_string_may_hold_quotes_and_semicolons() {
        let text = r#"a:1:{s:1:"k";s:5:"a";"b";}"#;
        assert_eq!(
            pairs(text),
            Some(vec![("k".to_owned(), "a\";\"b".to_owned())])
        );
    }

    #[test]
    fn broken_or_nested_arrays_are_refused() {
        for text in [
            "",
            "noindex",
            r#"a:1:{s:3:"max";s:3:"160""#,
            r#"a:1:{s:5:"too-short";s:1:"x";}"#,
            "a:1:{i:0;a:0:{}}",
            "a:1:{i:0;}",
        ] {
            assert_eq!(pairs(text), None, "{text}");
        }
    }
}
