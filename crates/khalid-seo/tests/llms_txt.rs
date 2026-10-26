//! `llms.txt`, the guide for language models (task 1.13, KH-SEO-7).

#![cfg(test)]

mod common;

use common::{demo_site, internal_links_post, internal_links_post_de, locale, path};
use khalid_seo::{LlmsTxt, PageKind, PublishState, SeoPage, UrlPath, llms_txt_url};

fn demo_pages() -> Vec<SeoPage> {
    let home = SeoPage::new(PageKind::Home, locale("en"), UrlPath::root(), "");
    let mut about = SeoPage::new(PageKind::Page, locale("en"), path("/about/"), "About [us]");
    about.excerpt = String::from("Who writes\nField Notes, and why.");
    let mut draft = SeoPage::new(PageKind::Post, locale("en"), path("/draft/"), "A draft");
    draft.state = PublishState::Draft;
    let mut tag = SeoPage::new(PageKind::Tag, locale("en"), path("/tag/links/"), "Links");
    tag.item_count = Some(5);
    vec![
        home,
        internal_links_post(),
        internal_links_post_de(),
        about,
        draft,
        tag,
    ]
}

/// Checks the llms.txt structure: one `# ` title first, then an optional
/// `> ` summary, then `## ` sections whose lines are `- [title](url)` links
/// with an optional `: description`.
fn check_structure(text: &str) -> Result<usize, String> {
    let mut lines = text.lines();
    let title = lines.next().ok_or("empty file")?;
    if !title.starts_with("# ") || title.len() < 3 {
        return Err(format!("the first line is not a # title: {title:?}"));
    }
    let mut link_count = 0;
    let mut in_section = false;
    for line in lines.filter(|line| !line.is_empty()) {
        if line.starts_with("## ") {
            in_section = true;
        } else if line.starts_with("> ") {
            if in_section {
                return Err("the summary comes after a section".to_owned());
            }
        } else if let Some(item) = line.strip_prefix("- [") {
            let (label, rest) = item
                .split_once("](")
                .ok_or(format!("no link in {line:?}"))?;
            let (url, after) = rest
                .split_once(')')
                .ok_or(format!("open link in {line:?}"))?;
            if label.is_empty()
                || !url.starts_with("https://")
                || !(after.is_empty() || after.starts_with(": "))
            {
                return Err(format!("a bad link line: {line:?}"));
            }
            if !in_section {
                return Err("a link before any section".to_owned());
            }
            link_count += 1;
        } else {
            return Err(format!("an unexpected line: {line:?}"));
        }
    }
    Ok(link_count)
}

#[test]
fn the_demo_llms_txt() {
    let llms = LlmsTxt::for_site(&demo_site(), &demo_pages()).unwrap();
    insta::assert_snapshot!(llms.to_string());
}

#[test]
fn the_file_has_the_llms_txt_structure() {
    let llms = LlmsTxt::for_site(&demo_site(), &demo_pages()).unwrap();
    assert_eq!(check_structure(&llms.to_string()), Ok(4));
}

#[test]
fn the_structure_check_refuses_a_broken_file() {
    for bad in [
        "Field Notes\n",
        "# Field Notes\n- [Links](https://example.com/)\n",
        "# Field Notes\n## Posts\n- [](https://example.com/)\n",
        "# Field Notes\n## Posts\n- [Links](/relative/)\n",
        "# Field Notes\n## Posts\nsome text\n",
    ] {
        assert!(check_structure(bad).is_err(), "{bad}");
    }
}

#[test]
fn it_lives_in_the_mount_folder() {
    assert_eq!(
        llms_txt_url(&demo_site()).unwrap().as_str(),
        "https://www.example.com/blog/llms.txt"
    );
}
