//! How-to blocks: named steps, nested per ADR 0012, that give structured
//! data (task 1.7, KH-CON-5).

use std::time::Duration;

use khalid_content::{
    BlockKind, HowToBlock, Step, StructuredBlock, parse_blocks, print_blocks, render,
};
use khalid_core::{Locale, SlugMode};

const HOW_TO: &str = "\
::::howto{name=\"Add an internal link\" time=\"10:00\"}
:::step{name=\"Find an older post\"}
Search your site for the topic of the **new** post.
:::

:::step{name=\"Link to it\"}
Write a sentence that names the older post and link those words.
:::
::::
";

#[cfg(test)]
fn how_to(first: &str) -> HowToBlock {
    HowToBlock {
        name: "Add an internal link".to_owned(),
        time: Some(Duration::from_secs(600)),
        steps: vec![
            Step {
                name: "Find an older post".to_owned(),
                text: first.to_owned(),
            },
            Step {
                name: "Link to it".to_owned(),
                text: "Write a sentence that names the older post and link those words.".to_owned(),
            },
        ],
    }
}

#[cfg(test)]
fn rendered(markdown: &str) -> khalid_content::Rendered {
    render(markdown, &Locale::try_new("en").unwrap(), SlugMode::Ascii).unwrap()
}

#[test]
fn a_how_to_round_trips() {
    let blocks = parse_blocks(HOW_TO).unwrap();
    assert_eq!(print_blocks(&blocks), HOW_TO);
    assert_eq!(
        blocks[0].kind,
        BlockKind::HowTo(how_to(
            "Search your site for the topic of the **new** post."
        ))
    );
}

#[test]
fn renders_a_how_to() {
    insta::assert_snapshot!(rendered(HOW_TO).html);
}

#[test]
fn a_how_to_is_structured_data_in_plain_text() {
    assert_eq!(
        rendered(HOW_TO).structured,
        [StructuredBlock::HowTo(how_to(
            "Search your site for the topic of the new post."
        ))]
    );
}

#[test]
fn a_how_to_needs_a_step() {
    assert_eq!(
        parse_blocks("::::howto{name=\"Add a link\"}\n::::\n")
            .unwrap_err()
            .to_string(),
        "line 1: a how-to needs at least one step, written :::step{name=\"...\"}, what to do, then :::"
    );
}
