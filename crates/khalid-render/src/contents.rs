//! A post's table of contents, built from its H2 and H3 headings.

use serde::Serialize;

/// How many H2 headings a post needs before it gets a table of contents.
/// A shorter post is quick to scan without one.
const MIN_SECTIONS: usize = 3;

/// The level of a heading a table of contents lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeadingLevel {
    /// An H2: a section.
    Two,
    /// An H3: a part of the section above it.
    Three,
}

/// One heading of a post's body, in order, with the `id` its element has.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentsHeading {
    /// H2 or H3.
    pub level: HeadingLevel,
    /// The heading element's `id`, unique in the body.
    pub id: String,
    /// The heading's text, without markup.
    pub text: String,
}

/// A post's table of contents: its sections, each with its parts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Contents {
    sections: Vec<Section>,
}

/// One H2, or an H3 that comes before any H2, and the H3s under it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct Section {
    id: String,
    text: String,
    parts: Vec<Part>,
}

/// One H3 under a section.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct Part {
    id: String,
    text: String,
}

impl Contents {
    /// The table of contents of a body with `headings`, in order, or `None`
    /// when it has fewer than three H2s.
    pub fn of(headings: Vec<ContentsHeading>) -> Option<Self> {
        let sections = headings
            .iter()
            .filter(|heading| heading.level == HeadingLevel::Two)
            .count();
        if sections < MIN_SECTIONS {
            return None;
        }
        let mut contents = Self {
            sections: Vec::with_capacity(sections),
        };
        for heading in headings {
            let ContentsHeading { level, id, text } = heading;
            match (level, contents.sections.last_mut()) {
                (HeadingLevel::Three, Some(section)) => section.parts.push(Part { id, text }),
                _ => contents.sections.push(Section {
                    id,
                    text,
                    parts: Vec::new(),
                }),
            }
        }
        Some(contents)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn heading(level: HeadingLevel, text: &str) -> ContentsHeading {
        ContentsHeading {
            level,
            id: text.to_lowercase().replace(' ', "-"),
            text: text.to_owned(),
        }
    }

    fn texts(contents: &Contents) -> Vec<(String, Vec<String>)> {
        contents
            .sections
            .iter()
            .map(|section| {
                let parts = section.parts.iter().map(|part| part.text.clone()).collect();
                (section.text.clone(), parts)
            })
            .collect()
    }

    #[test]
    fn a_post_with_fewer_than_three_sections_has_no_contents() {
        let two = vec![
            heading(HeadingLevel::Two, "What it does"),
            heading(HeadingLevel::Three, "New pages"),
            heading(HeadingLevel::Three, "Old pages"),
            heading(HeadingLevel::Two, "What it does not"),
        ];
        assert_eq!(Contents::of(two), None);
        assert_eq!(Contents::of(Vec::new()), None);
    }

    #[test]
    fn parts_sit_under_the_section_before_them() {
        let contents = Contents::of(vec![
            heading(HeadingLevel::Three, "In short"),
            heading(HeadingLevel::Two, "What it does"),
            heading(HeadingLevel::Three, "New pages"),
            heading(HeadingLevel::Two, "What it does not"),
            heading(HeadingLevel::Two, "How to make one"),
            heading(HeadingLevel::Three, "By hand"),
            heading(HeadingLevel::Three, "With a plugin"),
        ])
        .unwrap();
        assert_eq!(
            texts(&contents),
            [
                ("In short".to_owned(), vec![]),
                ("What it does".to_owned(), vec!["New pages".to_owned()]),
                ("What it does not".to_owned(), vec![]),
                (
                    "How to make one".to_owned(),
                    vec!["By hand".to_owned(), "With a plugin".to_owned()]
                ),
            ]
        );
    }
}
