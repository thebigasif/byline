//! HTML blocks: a writer's own HTML, sanitised, shown to everyone or only to
//! some readers (task 1.7, KH-CON-5, RULES.md BL-SEC-1).

use khalid_content::{BlockKind, Visibility, parse_blocks, print_blocks, render};
use khalid_core::{Locale, SlugMode};

#[cfg(test)]
fn html(markdown: &str) -> String {
    render(markdown, &Locale::try_new("en").unwrap(), SlugMode::Ascii)
        .unwrap()
        .html
}

#[test]
fn an_html_block_round_trips() {
    let markdown = ":::html{show=\"members\"}\n<p>Thanks for reading.</p>\n:::\n";
    let blocks = parse_blocks(markdown).unwrap();
    assert_eq!(print_blocks(&blocks), markdown);
    assert_eq!(
        blocks[0].kind,
        BlockKind::SafeHtml {
            show: Visibility::Members
        }
    );
}

#[test]
fn renders_html_with_scripts_handlers_and_script_links_removed() {
    insta::assert_snapshot!(html(
        "\
:::html
<table><tr><td>Plan</td><td onclick=\"steal()\">Price</td></tr></table>
<script>alert(1)</script>
<a href=\"javascript:alert(1)\">Click</a> <a href=\"https://example.com/\">Example</a>
<iframe src=\"https://evil.example/\"></iframe>
:::
"
    ));
}

#[test]
fn html_for_members_is_not_shown_until_readers_can_sign_in() {
    assert_eq!(
        html(
            ":::html{show=\"members\"}\n<p>Members only.</p>\n:::\n\n:::html{show=\"visitors\"}\n<p>Join us.</p>\n:::\n"
        ),
        "<p>Join us.</p>\n"
    );
}

#[test]
fn show_is_everyone_visitors_or_members() {
    assert_eq!(
        parse_blocks(":::html{show=\"admins\"}\n<p>Hi.</p>\n:::\n")
            .unwrap_err()
            .to_string(),
        "line 1: an HTML block's show is everyone, visitors or members"
    );
}
