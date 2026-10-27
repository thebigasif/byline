//! Readers' comments under a post: the approved ones with one level of
//! replies, a reply form under each, and the form for a new one, all plain
//! HTML that works with scripts off (KH-WEB-2).

use chrono::NaiveDate;
use khalid_render::{CommentWriter, RenderComment, RenderComments, RenderPost, RenderThread};

use crate::html_check::assert_valid_page;
use crate::post::{part, render, sitemap_post};

#[cfg(test)]
fn comment(id: i64, author: &str, writer: CommentWriter, text: &str) -> RenderComment {
    RenderComment {
        id,
        author: author.to_owned(),
        writer,
        date: NaiveDate::from_ymd_opt(2026, 10, 27).unwrap(),
        paragraphs: RenderComment::paragraphs_of(text),
    }
}

#[cfg(test)]
fn with_comments(threads: Vec<RenderThread>) -> RenderPost {
    let mut post = sitemap_post();
    post.comments = Some(RenderComments {
        threads,
        action: "/blog/comments/".to_owned(),
        page: "/blog/what-a-sitemap-does/".to_owned(),
    });
    post
}

#[test]
fn a_comment_and_its_reply_show_with_a_form_for_each() {
    let thread = RenderThread {
        comment: comment(
            1,
            "Ben <Reader>",
            CommentWriter::Reader,
            "Useful.\n\nThanks!",
        ),
        replies: vec![comment(
            2,
            "Lena Vogt",
            CommentWriter::Staff,
            "Glad it helped.",
        )],
    };
    let page = render("en", with_comments(vec![thread]));
    assert_valid_page(&page);
    let section = part(&page, "<section class=\"comments\"", "</section>");
    insta::assert_snapshot!(section);
    assert_eq!(
        section.matches("<form").count(),
        2,
        "a reply form and a new one"
    );
    assert!(section.contains("<input type=\"hidden\" name=\"parent\" value=\"1\">"));
    assert!(section.contains("Ben &lt;Reader&gt;"));
    assert!(!page.contains("<script src"), "comments need no script");
}

#[test]
fn a_post_without_comments_invites_the_first() {
    let page = render("de", with_comments(Vec::new()));
    assert_valid_page(&page);
    assert!(page.contains("<p class=\"comments-none\">Noch keine Kommentare.</p>"));
    assert!(page.contains("id=\"comment-new-name\""));
    assert!(!page.contains("name=\"parent\""));
}

#[test]
fn a_site_that_takes_no_comments_shows_no_section() {
    let page = render("en", sitemap_post());
    assert!(!page.contains("class=\"comments\""));
    assert!(!page.contains("<form class=\"comment-form\""));
}

#[test]
fn paragraphs_split_on_blank_lines_only() {
    let paragraphs = RenderComment::paragraphs_of(" one\ntwo \n\n\n three \n");
    assert_eq!(paragraphs, ["one\ntwo", "three"]);
}
